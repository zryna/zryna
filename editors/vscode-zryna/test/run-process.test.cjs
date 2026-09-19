'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { EventEmitter } = require('node:events');
const { resolve } = require('node:path');
const { compilerProcess, stopRuns } = require('../src/run-process.cjs');

test('process uses direct argument arrays and handles errors, output limits and cancellation', async () => {
  for (const mode of ['ok', 'exit', 'error', 'limit', 'cancel']) {
    let killed = false;
    const token = { onCancellationRequested(fn) { this.cancel = fn; return { dispose() {} }; } };
    const launch = (exe, args, options) => {
      assert.equal(exe, resolve('compiler'));
      assert.deepEqual(args, ['--arg=i32:2']);
      assert.equal(options.shell, false);
      const child = new EventEmitter();
      child.stdout = new EventEmitter(); child.stderr = new EventEmitter();
      child.kill = () => { killed = true; };
      setImmediate(() => {
        if (mode === 'cancel') token.cancel();
        else if (mode === 'error') child.emit('error', new Error('missing executable'));
        else if (mode === 'limit') child.stdout.emit('data', Buffer.alloc(1024 * 1024 + 1));
        else { child.stdout.emit('data', Buffer.from('result')); child.emit('close', mode === 'exit' ? 1 : 0); }
      });
      return child;
    };
    const pending = compilerProcess(resolve('compiler'), ['--arg=i32:2'], resolve('.'), token, launch);
    if (mode === 'ok') assert.equal((await pending).stdout, 'result');
    else { await assert.rejects(pending); assert.equal(killed, mode !== 'exit'); }
  }
});

test('cancelled requests and relative or shell executable paths never spawn', async () => {
  const launch = () => assert.fail('must not launch');
  assert.throws(() => compilerProcess(resolve('compiler'), [], '.', { isCancellationRequested: true }, launch), /cancelled/);
  for (const exe of ['relative', resolve('compiler.cmd'), resolve('compiler.bat')]) {
    assert.throws(() => compilerProcess(exe, [], '.', null, launch), /absolute/);
  }
});

test('deactivation cancels a live process and terminates it', async () => {
  const { spawn } = require('node:child_process');
  let closed;
  const launch = (...args) => {
    const child = spawn(...args);
    closed = new Promise(resolve => child.on('close', resolve));
    child.stdout.once('data', () => stopRuns());
    return child;
  };
  await assert.rejects(compilerProcess(process.execPath,
    ['-e', 'console.log("ready"); setInterval(() => {}, 1000);'], process.cwd(), null, launch), /cancelled/);
  await Promise.race([closed, new Promise((_, reject) => {
    const timer = setTimeout(() => reject(new Error('Cancelled process remained alive.')), 10000);
    timer.unref();
  })]);
});
