import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import setup from '../scripts/portable-setup/setup.cjs';

test('editor CLI lookup supports legacy and versioned installations and rejects ambiguity', () => {
  const root = mkdtempSync(join(tmpdir(), 'zryna-editor-cli-'));
  const executable = join(root, 'Code.exe');
  const add = prefix => {
    const file = join(root, prefix, 'resources', 'app', 'out', 'cli.js');
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, '');
    return file;
  };
  try {
    assert.throws(() => setup.editorCli(executable), /unique VS Code CLI/);
    const versioned = add('7debcd0e2a');
    assert.equal(setup.editorCli(executable), versioned);
    add('1234567890');
    assert.throws(() => setup.editorCli(executable), /unique VS Code CLI/);
    const legacy = add('');
    assert.equal(setup.editorCli(executable), legacy);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
