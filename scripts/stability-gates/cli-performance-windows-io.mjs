// Trusted Windows operator adapter. Helper owns the measured tree with KILL_ON_JOB_CLOSE.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { lstatSync, rmSync } from 'node:fs';
import { isAbsolute, join, posix, relative, resolve } from 'node:path';
import { canonical, MAX_DOCUMENT, MAX_LOG, readSafe } from './input.mjs';
import { checkKnownGraphs, checkSelectedBytes } from './cli-performance-methodology-v4.mjs';
import { flat, helperKeys } from './windows-job-metrics-v1/calibration/protocol.mjs';
import { localAbsolute, openWindowsBinding, validateWindowsCapture } from './cli-performance-windows-binding.mjs';
import { inventory, retention, sameFile, stageOriginalWindowsM3, validateWindowsOracles } from './cli-performance-windows-files.mjs';
import { collectWindowsRound } from './cli-performance-windows-round.mjs';
export async function captureWindowsDraft({ document, capture, oracles, root, output }) {
  assert.equal(process.platform, 'win32'); assert.equal(process.arch, 'x64');
  localAbsolute(root); localAbsolute(output); assert(root === resolve(root) && output === resolve(output));
  assert(isAbsolute(root) && isAbsolute(output));
  const inside = relative(resolve(root, '.zryna'), output);
  assert(inside.startsWith('..') || isAbsolute(inside), 'evidence outside all generated/staged directories');
  validateWindowsCapture(document, capture); validateWindowsOracles(document, capture, oracles);
  const binding = openWindowsBinding(root, capture); // operator input admission before staging/children
  const retain = retention(output);
  retain('artifact-oracles.json', Buffer.from(canonical(oracles))); retain('operator-admission.json', binding.admissionBytes);
  const environment = capture.environment; canonical(environment);
  const allowed = ['PATH', 'SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'USERPROFILE', 'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN', 'RUSTC', 'LIB', 'INCLUDE'];
  for (const [key, value] of Object.entries(environment)) assert(allowed.includes(key) && typeof value === 'string' && value.length < 8192, 'explicit bounded environment');
  const checkMaterials = () => {
    binding.check(); checkSelectedBytes(document, (path, maximum) => readSafe(root, path, maximum));
    checkKnownGraphs(document, (path, maximum) => readSafe(root, path, maximum));
    for (const family of document.families) if (family.staging) for (const copy of family.staging.copies)
      sameFile(root, { ...family.sources.find(pin => pin.path === copy.from), path: copy.to }, MAX_DOCUMENT);
  };
  const bySelector = new Map(oracles.map(row => [row.selector, row]));
  const bundle = row => posix.dirname(row.expectedResponse.manifest);
  const checkArtifactsAndStreams = (row, stdout, stderr) => {
    const oracle = bySelector.get(row.id);
    assert(stdout.equals(Buffer.from(oracle.stdout)), 'exact frozen public stdout bytes');
    assert(stderr.equals(Buffer.from(oracle.stderr)), 'exact frozen public stderr bytes');
    assert.deepEqual(inventory(root, bundle(row)), oracle.files.map(pin => pin.path));
    for (const pin of oracle.files) sameFile(root, pin, 64 * 1024 * 1024);
  };
  const finishRound = stageOriginalWindowsM3(root, document);
  return collectWindowsRound(document, capture, {
    root, output, retain, checkMaterials, finishRound,
    execute: (file, argv, timeoutMs) => {
      const result = spawnSync(file, argv, { cwd: root, env: environment, shell: false, windowsHide: true,
        encoding: null, timeout: timeoutMs, killSignal: 'SIGKILL', maxBuffer: MAX_LOG });
      // Killing the helper closes its same Job; lack of its complete closure record stays invalid.
      return { exitCode: result.status, signal: result.signal,
        error: result.error ? `${result.error.code ?? 'SPAWN'}: ${result.error.message}`.slice(0, 4096) : null,
        stdout: result.stdout ?? Buffer.alloc(0), stderr: result.stderr ?? Buffer.alloc(0) };
    },
    readSample: directory => ({ metadata: readSafe(directory, 'result.json', 4096),
      stdout: readSafe(directory, 'stdout.raw', MAX_LOG), stderr: readSafe(directory, 'stderr.raw', MAX_LOG), error: null }),
    parseHelper: (directory, raw) => {
      const result = flat(join(directory, 'result.json'), 4096, helperKeys);
      assert(readSafe(directory, 'result.json', 4096).equals(raw), 'parsed retained helper bytes'); return result;
    },
    beforeAttempt: row => {
      try { lstatSync(resolve(root, bundle(row))); } catch (error) { if (error.code === 'ENOENT') return; throw error; }
      assert.fail('owned output already exists; never replace foreign artifacts');
    },
    checkArtifactsAndStreams,
    afterAttempt: row => {
      const oracle = bySelector.get(row.id);
      checkArtifactsAndStreams(row, Buffer.from(oracle.stdout), Buffer.from(oracle.stderr));
      rmSync(resolve(root, bundle(row)), { recursive: true, force: false });
    },
  });
}
