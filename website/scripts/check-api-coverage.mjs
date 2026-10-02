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
  {kind: 'struct', re: /^\s*pub struct\s+([A-Za-z_][A-Za-z0-9_]*)/},
  {kind: 'enum', re: /^\s*pub enum\s+([A-Za-z_][A-Za-z0-9_]*)/},
  {kind: 'fn', re: /^\s*pub(?:\s*\([^)]*\))?\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)/},
  {kind: 'mod', re: /^\s*pub mod\s+([A-Za-z_][A-Za-z0-9_]*)/},
];

// `impl Type {` or `impl<T> Trait for Type {`
const IMPL_RE =
  /^\s*impl(?:\s*<[^>]*>)?\s+(?:[\w:]+(?:<[^>]*>)?\s+for\s+)?([A-Za-z_][A-Za-z0-9_]*)/;

/**
 * Walk a file line by line, tracking brace depth so every `pub fn` inside an
 * `impl Type` block is recorded as `Type::fn` rather than a bare name. A bare
 * `new` appearing anywhere in the docs must not count as coverage for
 * `Tracker::new`.
 */
function collectItemsFromFile(file) {
  const source = stripTestModules(readFileSync(file, 'utf8'));
  const relPath = relative(repoRoot, file);
  const items = [];
  const implStack = []; // {owner, depth}
  let depth = 0;

  for (const line of source.split('\n')) {
    const code = line.replace(/\/\/.*$/, '');

    for (const {kind, re} of ITEM_PATTERNS) {
      const m = re.exec(code);
      if (m) {
        const owner = kind === 'fn' ? implStack.at(-1)?.owner : undefined;
        items.push({name: m[1], kind, owner, file: relPath});
        break;
      }
    }

    const implMatch = IMPL_RE.exec(code);
    const opens = (code.match(/\{/g) ?? []).length;
    const closes = (code.match(/\}/g) ?? []).length;
    if (implMatch && opens > 0) {
      implStack.push({owner: implMatch[1], depth});
    }
    depth += opens - closes;
    while (implStack.length > 0 && depth <= implStack.at(-1).depth) {
      implStack.pop();
    }
  }
  return items;
}

function collectPublicItems() {
  return listFiles(srcDir, '.rs').flatMap(collectItemsFromFile);
}

/** What the docs must mention: `Type::method` for methods, the name otherwise. */
function expectedMention(item) {
  return item.owner ? `${item.owner}::${item.name}` : item.name;
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

  const missing = items.filter((item) => {
    const mention = expectedMention(item);
    const pattern = new RegExp(
      `(^|[^A-Za-z0-9_:])${mention.replace('::', '::')}([^A-Za-z0-9_]|$)`,
    );
    return !pattern.test(docs);
  });

  const count = (kind) => items.filter((i) => i.kind === kind).length;
  const methods = items.filter((i) => i.owner).length;
  console.log(
    `${GREEN}✔${RESET} ${items.length} public items found in src/ ${DIM}(${count(
      'struct',
    )} structs, ${count('enum')} enums, ${count('fn')} fns of which ${methods} are inherent/trait methods, ${count('mod')} mods)${RESET}`,
  );

  if (missing.length > 0) {
    console.error(
      `\n${RED}✘${RESET} ${missing.length} public item(s) are not documented in docs/reference/:`,
    );
    for (const item of missing) {
      console.error(
        `   - ${item.file}:  ${item.kind} ${expectedMention(item)}`,
      );
    }
    console.error(
      '\nAdd them to the matching page under website/docs/reference/ ' +
        '(methods must be referenced as `Type::method`).',
    );
    process.exit(1);
  }

  console.log(`${GREEN}✔${RESET} all public items are documented in docs/reference/`);
}

main();
