#!/usr/bin/env python3
"""Verify compiled SteamLogger binaries, dynamic library imports, and startup.

Checks performed:
  * linux-x64:    ELF64 x86-64 binary linking only standard base glibc libraries
                  (no external OpenSSL or non-system shared libraries).
  * windows-x64:  PE32+ x86-64 executable with static CRT linkage (no external
                  VCRUNTIME140 / MSVCP140 / UCRT DLL prerequisites).
  * windows7-x64: All windows-x64 checks plus Windows 7 SP1 compatibility
                  screening of PE import and delay-import tables (no Windows 8+
                  DLLs such as bcryptprimitives.dll, combase.dll, or
                  api-ms-win-core-synch-l1-2-0.dll, and no Windows 8+/10+ APIs
                  such as ProcessPrng or WaitOnAddress).
  * --smoke:      Invokes the binary with a nonexistent config path in an
                  isolated temp directory to confirm the OS loader resolves all
                  imports and `main` exits cleanly with its config error.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

LINUX_INTERPRETER = "/lib64/ld-linux-x86-64.so.2"
ALLOWED_LINUX_LIBS = {
    "libc.so.6",
    "libm.so.6",
    "libgcc_s.so.1",
    "libpthread.so.0",
    "libdl.so.2",
    "librt.so.1",
    "ld-linux-x86-64.so.2",
}

EXTERNAL_CRT_PREFIXES = (
    "vcruntime",
    "msvcp",
    "concrt",
    "vcomp",
    "ucrtbase",
    "api-ms-win-crt-",
)
EXTERNAL_CRT_EXACT = {
    "msvcr100.dll",
    "msvcr110.dll",
    "msvcr120.dll",
}

POST_WIN7_DLLS = {
    "bcryptprimitives.dll",
    "combase.dll",
    "shcore.dll",
}
POST_WIN7_DLL_PREFIXES = (
    "api-ms-win-core-winrt-",
    "api-ms-win-core-synch-l1-2-",
    "api-ms-win-core-path-",
    "api-ms-win-shcore-",
)
POST_WIN7_SYMBOLS = {
    "ProcessPrng",
    "WaitOnAddress",
    "WakeByAddressSingle",
    "WakeByAddressAll",
    "GetSystemTimePreciseAsFileTime",
    "GetSystemTimeAdjustmentPrecise",
    "GetOverlappedResultEx",
    "CreateFile2",
    "RoInitialize",
    "RoGetActivationFactory",
    "RoOriginateError",
    "RoOriginateErrorW",
    "SetThreadDescription",
    "GetThreadDescription",
}


def verify_elf64_x64(data: bytes, path: Path) -> None:
    if len(data) < 64 or data[:4] != b"\x7fELF":
        raise RuntimeError(f"{path}: not an ELF binary")
    if data[4] != 2 or data[5] != 1:
        raise RuntimeError(f"{path}: expected 64-bit little-endian ELF")
    (machine,) = struct.unpack_from("<H", data, 18)
    if machine != 0x3E:
        raise RuntimeError(f"{path}: expected EM_X86_64 (0x3E), got {machine:#x}")

    (phoff,) = struct.unpack_from("<Q", data, 32)
    phentsize, phnum = struct.unpack_from("<HH", data, 54)

    interpreter: str | None = None
    dynamic_seg: tuple[int, int] | None = None
    loads: list[tuple[int, int, int]] = []

    for i in range(phnum):
        p_type, _, p_offset, p_vaddr, _, p_filesz = struct.unpack_from(
            "<IIQQQQ", data, phoff + i * phentsize
        )
        if p_type == 1:  # PT_LOAD
            loads.append((p_vaddr, p_offset, p_filesz))
        elif p_type == 2:  # PT_DYNAMIC
            dynamic_seg = (p_offset, p_filesz)
        elif p_type == 3:  # PT_INTERP
            interpreter = data[p_offset : p_offset + p_filesz].rstrip(b"\x00").decode("ascii")

    def vaddr_to_offset(vaddr: int) -> int:
        for seg_vaddr, seg_offset, seg_filesz in loads:
            if seg_vaddr <= vaddr < seg_vaddr + seg_filesz:
                return seg_offset + (vaddr - seg_vaddr)
        raise RuntimeError(f"{path}: virtual address {vaddr:#x} outside PT_LOAD segments")

    needed_offsets: list[int] = []
    strtab_vaddr: int | None = None
    if dynamic_seg is not None:
        dyn_off, dyn_size = dynamic_seg
        for pos in range(dyn_off, dyn_off + dyn_size, 16):
            d_tag, d_val = struct.unpack_from("<qQ", data, pos)
            if d_tag == 0:  # DT_NULL
                break
            if d_tag == 1:  # DT_NEEDED
                needed_offsets.append(d_val)
            elif d_tag == 5:  # DT_STRTAB
                strtab_vaddr = d_val

    needed: list[str] = []
    if needed_offsets:
        if strtab_vaddr is None:
            raise RuntimeError(f"{path}: DT_NEEDED present without DT_STRTAB")
        strtab_off = vaddr_to_offset(strtab_vaddr)
        for off in needed_offsets:
            start = strtab_off + off
            end = data.index(b"\x00", start)
            needed.append(data[start:end].decode("ascii"))

    if interpreter is not None and interpreter != LINUX_INTERPRETER:
        raise RuntimeError(
            f"{path}: unexpected ELF interpreter {interpreter!r} (expected {LINUX_INTERPRETER!r})"
        )

    disallowed = sorted(lib for lib in needed if lib not in ALLOWED_LINUX_LIBS)
    if disallowed:
        raise RuntimeError(
            f"{path}: links non-baseline shared libraries: {', '.join(disallowed)}"
        )

    print(
        f"ELF64 x86-64 verified: {path} "
        f"(interpreter={interpreter or 'static'}, needed={needed})"
    )


def parse_pe32plus_imports(data: bytes, path: Path) -> list[tuple[str, list[str], str]]:
    if len(data) < 0x40 or data[:2] != b"MZ":
        raise RuntimeError(f"{path}: not a PE executable (missing MZ header)")
    (pe_off,) = struct.unpack_from("<I", data, 0x3C)
    if pe_off + 24 > len(data) or data[pe_off : pe_off + 4] != b"PE\x00\x00":
        raise RuntimeError(f"{path}: invalid PE signature")

    coff = pe_off + 4
    machine, num_sections, _, _, _, opt_size, _ = struct.unpack_from("<HHIIIHH", data, coff)
    if machine != 0x8664:
        raise RuntimeError(
            f"{path}: expected x64 PE machine 0x8664, got {machine:#x}"
        )

    opt = coff + 20
    (magic,) = struct.unpack_from("<H", data, opt)
    if magic != 0x20B:
        raise RuntimeError(f"{path}: expected PE32+ magic 0x20B, got {magic:#x}")

    (num_dirs,) = struct.unpack_from("<I", data, opt + 108)
    dirs_off = opt + 112

    sec_table = opt + opt_size
    sections: list[tuple[int, int, int, int]] = []
    for i in range(num_sections):
        virt_size, virt_addr, raw_size, raw_ptr = struct.unpack_from(
            "<IIII", data, sec_table + i * 40 + 8
        )
        sections.append((virt_size, virt_addr, raw_size, raw_ptr))

    def rva_to_offset(rva: int) -> int | None:
        for virt_size, virt_addr, raw_size, raw_ptr in sections:
            span = max(virt_size, raw_size)
            if virt_addr <= rva < virt_addr + span:
                delta = rva - virt_addr
                if delta >= raw_size:
                    return None
                return raw_ptr + delta
        return None

    def read_cstr(rva: int) -> str:
        off = rva_to_offset(rva)
        if off is None or off >= len(data):
            raise RuntimeError(f"{path}: unmapped string RVA {rva:#x}")
        end = data.index(b"\x00", off)
        return data[off:end].decode("ascii", errors="replace")

    def read_thunk_symbols(thunk_rva: int) -> list[str]:
        if thunk_rva == 0:
            return []
        off = rva_to_offset(thunk_rva)
        if off is None:
            return []
        symbols: list[str] = []
        while off + 8 <= len(data):
            (entry,) = struct.unpack_from("<Q", data, off)
            off += 8
            if entry == 0:
                break
            if entry & (1 << 63):
                # IMAGE_SNAP_BY_ORDINAL64 (e.g. ws2_32.dll ordinal imports).
                continue
            hint_name_rva = entry & 0x7FFFFFFF
            sym_off = rva_to_offset(hint_name_rva)
            if sym_off is None or sym_off + 2 >= len(data):
                raise RuntimeError(
                    f"{path}: unmapped IMAGE_IMPORT_BY_NAME RVA {hint_name_rva:#x}"
                )
            end = data.index(b"\x00", sym_off + 2)
            symbols.append(data[sym_off + 2 : end].decode("ascii", errors="replace"))
        return symbols

    imports: list[tuple[str, list[str], str]] = []

    # Directory 1: standard import descriptors (20 bytes each).
    if num_dirs > 1:
        imp_rva, _ = struct.unpack_from("<II", data, dirs_off + 8 * 1)
        if imp_rva != 0:
            cursor = rva_to_offset(imp_rva)
            if cursor is None:
                raise RuntimeError(f"{path}: unmapped import directory RVA {imp_rva:#x}")
            while cursor + 20 <= len(data):
                orig_thunk, ts, fwd, name_rva, first_thunk = struct.unpack_from(
                    "<IIIII", data, cursor
                )
                cursor += 20
                if (orig_thunk, ts, fwd, name_rva, first_thunk) == (0, 0, 0, 0, 0):
                    break
                dll = read_cstr(name_rva)
                syms = read_thunk_symbols(orig_thunk or first_thunk)
                imports.append((dll, syms, "eager"))

    # Directory 13: delay-load import descriptors (32 bytes each).
    if num_dirs > 13:
        del_rva, _ = struct.unpack_from("<II", data, dirs_off + 8 * 13)
        if del_rva != 0:
            cursor = rva_to_offset(del_rva)
            if cursor is None:
                raise RuntimeError(
                    f"{path}: unmapped delay-import directory RVA {del_rva:#x}"
                )
            while cursor + 32 <= len(data):
                fields = struct.unpack_from("<IIIIIIII", data, cursor)
                cursor += 32
                if all(v == 0 for v in fields):
                    break
                attrs, dll_name_rva, _, _, int_rva, _, _, _ = fields
                if attrs & 0x1:  # dlattrRva
                    dll = read_cstr(dll_name_rva)
                    syms = read_thunk_symbols(int_rva)
                    imports.append((dll, syms, "delay"))

    return imports


def verify_pe_x64(data: bytes, path: Path, *, win7: bool) -> None:
    imports = parse_pe32plus_imports(data, path)
    lower_dlls = sorted({dll.lower() for dll, _, _ in imports})
    if "kernel32.dll" not in lower_dlls:
        raise RuntimeError(
            f"{path}: expected kernel32.dll in PE imports, found {lower_dlls}"
        )

    crt_dlls = sorted(
        dll
        for dll in lower_dlls
        if dll in EXTERNAL_CRT_EXACT or dll.startswith(EXTERNAL_CRT_PREFIXES)
    )
    if crt_dlls:
        raise RuntimeError(
            f"{path}: dynamically imports external C/C++ runtime DLLs "
            f"(expected +crt-static): {', '.join(crt_dlls)}"
        )

    if win7:
        bad_dlls = sorted(
            dll
            for dll in lower_dlls
            if dll in POST_WIN7_DLLS or dll.startswith(POST_WIN7_DLL_PREFIXES)
        )
        if bad_dlls:
            raise RuntimeError(
                f"{path}: imports post-Windows 7 DLLs: {', '.join(bad_dlls)}"
            )

        bad_symbols: list[str] = []
        for dll, syms, mode in imports:
            for sym in syms:
                if sym in POST_WIN7_SYMBOLS:
                    bad_symbols.append(f"{dll}!{sym} ({mode})")
        if bad_symbols:
            raise RuntimeError(
                f"{path}: imports post-Windows 7 APIs: {', '.join(sorted(bad_symbols))}"
            )

    total_syms = sum(len(syms) for _, syms, _ in imports)
    target_label = "Windows 7 SP1 x64" if win7 else "Windows x64"
    print(
        f"PE32+ ({target_label}) verified: {path} "
        f"(dlls={lower_dlls}, named_symbols={total_syms})"
    )


def run_smoke_test(path: Path) -> None:
    exe = path.resolve()
    with tempfile.TemporaryDirectory(prefix="steamlogger-smoke-") as tmp:
        missing_cfg = Path(tmp) / "__missing_smoke_config__.toml"
        proc = subprocess.run(
            [str(exe), str(missing_cfg)],
            cwd=tmp,
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )
    if proc.returncode != 1:
        raise RuntimeError(
            f"{path}: smoke run returned exit code {proc.returncode:#x} "
            f"(expected 1)\nstdout: {proc.stdout}\nstderr: {proc.stderr}"
        )
    if "failed to load config from" not in proc.stderr:
        raise RuntimeError(
            f"{path}: smoke run stderr missing expected config error: {proc.stderr!r}"
        )
    print(f"Smoke test passed: {path} exited cleanly with expected config message")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--kind",
        required=True,
        choices=("linux-x64", "windows-x64", "windows7-x64"),
        help="Target platform validation profile",
    )
    parser.add_argument(
        "--smoke",
        action="store_true",
        help="Execute the binary with a missing config path to verify loader startup",
    )
    parser.add_argument("binary", type=Path, help="Path to compiled executable")
    args = parser.parse_args()

    if not args.binary.is_file():
        raise SystemExit(f"error: binary not found: {args.binary}")
    data = args.binary.read_bytes()
    if not data:
        raise SystemExit(f"error: binary is empty: {args.binary}")

    if args.kind == "linux-x64":
        verify_elf64_x64(data, args.binary)
    elif args.kind == "windows-x64":
        verify_pe_x64(data, args.binary, win7=False)
    elif args.kind == "windows7-x64":
        verify_pe_x64(data, args.binary, win7=True)

    if args.smoke:
        run_smoke_test(args.binary)

    return 0


if __name__ == "__main__":
    sys.exit(main())
