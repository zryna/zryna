import assert from 'node:assert/strict';
import { reject } from './evidence-input.mjs';

const counterNames = ['tests', 'suites', 'pass', 'fail', 'cancelled', 'skipped', 'todo'];
function level(spaces) {
  if (spaces.length % 4) reject('Node TAP nesting indentation mismatch');
  return spaces.length / 4;
}
function details(lines, index, spaces) {
  const block = lines.slice(index + 1, index + 5);
  if (block[0] !== `${spaces}  ---` || block[3] !== `${spaces}  ...` ||
      !new RegExp(`^${spaces}  duration_ms: (?:\\d+(?:\\.\\d+)?(?:e[+-]?\\d+)?)$`).test(block[1])) {
    reject('complete Node TAP result diagnostics required');
  }
  const type = new RegExp(`^${spaces}  type: '(test|suite)'$`).exec(block[2]);
  if (!type) reject('explicit Node TAP test/suite type required');
  return type[1];
}
function complete(node) {
  if (node.children.length && node.plan !== node.children.length) reject('Node TAP nested plan/row mismatch');
  if (node.plan !== null && node.plan !== node.children.length) reject('Node TAP plan/row mismatch');
}
export function requireNodeTests(output) {
  if (/^\s*[✔✖▶﹣ℹ]/m.test(output)) reject('unsupported Node reporter; fixed producer requires complete TAP');
  const lines = output.replace(/\r\n/g, '\n').split('\n');
  const root = { children: [], plan: null };
  const stack = [root];
  const rows = [];
  const counters = [];
  let version = false;
  let footer = false;
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index];
    if (line === 'TAP version 13') {
      if (version || rows.length || stack.length !== 1 || footer) reject('single initial Node TAP version required');
      version = true; continue;
    }
    const start = /^( *)# Subtest: (.+)$/.exec(line);
    const result = /^( *)(ok|not ok) (\d+) - (.+?)(?: # (SKIP|TODO)(?: .*)?)?$/.exec(line);
    const plan = /^( *)1\.\.(\d+)$/.exec(line);
    if (start || result || plan) {
      if (!version || footer) reject('Node TAP execution outside complete report');
      const context = stack.at(-1);
      if (start) {
        if (level(start[1]) !== stack.length - 1 || context.plan !== null) reject('Node TAP start hierarchy mismatch');
        stack.push({ name: start[2], children: [], plan: null });
      } else if (plan) {
        if (level(plan[1]) !== stack.length - 1 || context.plan !== null) reject('Node TAP duplicate or misplaced plan');
        context.plan = Number(plan[2]);
        complete(context);
      } else {
        if (stack.length === 1 || level(result[1]) !== stack.length - 2 || result[4] !== context.name) reject('Node TAP named result/start hierarchy mismatch');
        if (result[2] !== 'ok' || result[5] === 'TODO') reject('failed or TODO named Node TAP result');
        complete(context);
        context.type = details(lines, index, result[1]);
        context.skipped = result[5] === 'SKIP';
        if (context.skipped && context.children.length) reject('skipped Node TAP parent borrowed executed children');
        stack.pop();
        const parent = stack.at(-1);
        if (Number(result[3]) !== parent.children.length + 1) reject('Node TAP duplicate, missing or reordered sibling result');
        parent.children.push(context); rows.push(context);
        index += 4;
      }
    } else {
      const counter = /^# (tests|suites|pass|fail|cancelled|skipped|todo) (\d+)$/.exec(line);
      if (counter) {
        if (!version || stack.length !== 1 || root.plan === null) reject('Node TAP counters precede completed hierarchy');
        footer = true; counters.push(counter);
      } else if (/^(?: *# Subtest:| *(?:not )?ok\b| *1\.\.| *type:|# (?:tests|suites|pass|fail|cancelled|skipped|todo)\b)/.test(line)) {
        reject('malformed or truncated Node TAP structure');
      }
    }
  }
  if (!version || stack.length !== 1 || root.plan === null) reject('incomplete Node TAP hierarchy or root plan');
  complete(root);
  assert.deepEqual(counters.map(match => match[1]), counterNames, 'single complete ordered Node TAP counter inventory');
  const counts = Object.fromEntries(counters.map(match => [match[1], Number(match[2])]));
  const tests = rows.filter(node => node.type === 'test');
  // Node22 countCompletedTest counts parent tests too; suites have a separate total.
  const observed = { tests: tests.length, suites: rows.length - tests.length,
    pass: tests.filter(node => !node.skipped).length, fail: 0, cancelled: 0,
    skipped: tests.filter(node => node.skipped).length, todo: 0 };
  assert.deepEqual(counts, observed, 'Node TAP named test/suite/outcome counter accounting');
  if (!counts.tests || !counts.pass || !tests.some(node => !node.skipped && node.children.length === 0)) {
    reject('actual passing named Node leaf tests absent');
  }
  return counts;
}
