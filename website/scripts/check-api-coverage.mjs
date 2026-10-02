#!/usr/bin/env node
/**
 * Documentation drift guard.
 *
 * Scans ../src for public Rust items (`pub struct`, `pub enum`, `pub fn`,
 * `pub mod`) and verifies that every one of them is mentioned somewhere in
 * docs/reference/. Keeps the code reference honest when the crate changes.
 *
 * Deliberately simple: regex over source text, no Rust parsing. It skips
 * `#[cfg(test)]` modules, the tests/ directory, and private items.
 *
 * Usage: npm run check:api
 */

import {readdirSync, readFileSync, statSync} from 'node:fs';
import {dirname, join, relative, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

const websiteDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = resolve(websiteDir, '..');
const srcDir = join(repoRoot, 'src');
const referenceDir = join(websiteDir, 'docs', 'reference');

const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const DIM = '\x1b[2m';
const RESET = '\x1b[0m';

/** Recursively list files with the given extension. */
function listFiles(dir, ext) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...listFiles(full, ext));
    } else if (entry.endsWith(ext)) {
      out.push(full);
    }
  }
  return out;
}

/**
 * Strip `#[cfg(test)] mod tests { ... }` blocks so test-only helpers are not
 * mistaken for public API. Brace counting is enough for this codebase.
 */
function stripTestModules(source) {
  const marker = /#\[cfg\(test\)\]\s*mod\s+\w+\s*\{/g;
  let result = source;
  let match;
  while ((match = marker.exec(result)) !== null) {
    let depth = 1;
    let i = match.index + match[0].length;
    while (i < result.length && depth > 0) {
      if (result[i] === '{') depth += 1;
      else if (result[i] === '}') depth -= 1;
      i += 1;
    }
    result = result.slice(0, match.index) + result.slice(i);
    marker.lastIndex = 0;
  }
  return result;
}

const ITEM_PATTERNS = [
  {kind: 'struct', re: /^\s*pub struct\s+([A-Za-z_][A-Za-z0-9_]*)/gm},
  {kind: 'enum', re: /^\s*pub enum\s+([A-Za-z_][A-Za-z0-9_]*)/gm},
  {kind: 'fn', re: /^\s*pub(?:\s*\([^)]*\))?\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)/gm},
  {kind: 'mod', re: /^\s*pub mod\s+([A-Za-z_][A-Za-z0-9_]*)/gm},
];

function collectPublicItems() {
  const items = [];
  for (const file of listFiles(srcDir, '.rs')) {
    const source = stripTestModules(readFileSync(file, 'utf8'));
    for (const {kind, re} of ITEM_PATTERNS) {
      re.lastIndex = 0;
      let m;
      while ((m = re.exec(source)) !== null) {
        items.push({name: m[1], kind, file: relative(repoRoot, file)});
      }
    }
  }
  return items;
}

function readReferenceDocs() {
  const files = [
    ...listFiles(referenceDir, '.mdx'),
    ...listFiles(referenceDir, '.md'),
  ];
  if (files.length === 0) {
    console.error(`${RED}✘${RESET} no reference pages found in ${relative(repoRoot, referenceDir)}`);
    process.exit(1);
  }
  return files.map((f) => readFileSync(f, 'utf8')).join('\n');
}

function main() {
  const items = collectPublicItems();
  const docs = readReferenceDocs();

  // `pub mod` names are module declarations; they are covered by the crate
  // map table rather than by an item heading.
  const missing = items.filter(({name}) => {
    const wordBoundary = new RegExp(`(^|[^A-Za-z0-9_])${name}([^A-Za-z0-9_]|$)`);
    return !wordBoundary.test(docs);
  });

  const unique = new Set(items.map((i) => `${i.file}:${i.name}`));
  console.log(
    `${GREEN}✔${RESET} ${unique.size} public items found in src/ ${DIM}(${items.filter((i) => i.kind === 'struct').length} structs, ${items.filter((i) => i.kind === 'enum').length} enums, ${items.filter((i) => i.kind === 'fn').length} fns, ${items.filter((i) => i.kind === 'mod').length} mods)${RESET}`,
  );

  if (missing.length > 0) {
    console.error(
      `\n${RED}✘${RESET} ${missing.length} public item(s) are not documented in docs/reference/:`,
    );
    for (const item of missing) {
      console.error(`   - ${item.file}:  ${item.kind} ${item.name}`);
    }
    console.error('\nAdd them to the matching page under website/docs/reference/.');
    process.exit(1);
  }

  console.log(`${GREEN}✔${RESET} all public items are documented in docs/reference/`);
}

main();
