import assert from 'node:assert/strict';
import { lstat, readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const documentPath = 'docs/CODE_NAVIGATION.md';
const document = await readFile(path.join(root, documentPath), 'utf8');

// This index uses plain relative Markdown links, not arbitrary Markdown or website URLs.
async function validateLinks(markdown) {
  const links = [...markdown.matchAll(/\[[^\]\n]+\]\(([^)\n]+)\)/g)];
  assert(links.length > 0, 'navigation links must not be empty');
  for (const [, target] of links) {
    assert.match(target, /^[A-Za-z0-9_.\/-]+$/, 'navigation link must be a plain relative path');
    assert(!path.posix.isAbsolute(target), 'navigation link must be relative');
    const relative = path.posix.normalize(path.posix.join('docs', target));
    assert(relative !== '..' && !relative.startsWith('../'), 'navigation link escapes repository');
    const parts = relative.split('/');
    let current = root;
    for (const [index, part] of parts.entries()) {
      current = path.join(current, part);
      const metadata = await lstat(current);
      assert(!metadata.isSymbolicLink(), 'navigation link must not traverse a symlink');
      if (index + 1 === parts.length) {
        assert(metadata.isFile(), 'navigation link target must be a regular file');
        assert(metadata.size <= 2 * 1024 * 1024, 'navigation link target exceeds file budget');
      } else {
        assert(metadata.isDirectory(), 'navigation link ancestor must be a directory');
      }
    }
  }
}

test('code navigation links resolve and contribution guidance exposes the index', async () => {
  await validateLinks(document);
  assert(document.trimEnd().split('\n').length <= 150, 'navigation stays compact');
  assert.deepEqual([...document.matchAll(/^## ([0-9]+)\. /gm)].map(match => Number(match[1])),
    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  const contributing = await readFile(path.join(root, 'CONTRIBUTING.md'), 'utf8');
  assert.equal([...contributing.matchAll(/\]\(docs\/CODE_NAVIGATION\.md\)/g)].length, 1);
});

test('code navigation rejects missing, escaping, non-file, and unsupported link targets', async () => {
  for (const [target, error] of [
    ['missing-navigation-target.md', /ENOENT/],
    ['../../outside.md', /escapes repository/],
    ['/README.md', /must be relative/],
    ['../crates', /must be a regular file/],
    ['https://example.invalid/source', /plain relative path/],
    ['..\\README.md', /plain relative path/],
  ]) {
    await assert.rejects(validateLinks(`[source](${target})`), error);
  }
  await assert.rejects(validateLinks('no links'), /must not be empty/);
});

test('code navigation focused pnpm commands and Cargo packages remain registered', async () => {
  const packageDocument = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
  const workspace = JSON.parse(await readFile(path.join(root, 'zryna.workspace.json'), 'utf8'));
  const scripts = [...document.matchAll(/\bpnpm ([a-z][a-z0-9:-]*)/g)].map(match => match[1]);
  assert(scripts.length > 0);
  for (const command of scripts) {
    if (command !== 'install') assert(Object.hasOwn(packageDocument.scripts, command), command);
  }
  const packages = [...document.matchAll(/\bcargo (?:test|run) --locked -p ([a-z0-9-]+)/g)]
    .map(match => match[1]);
  assert(packages.length > 0);
  for (const name of packages) assert(workspace.members.some(member => member.id === name), name);
});
