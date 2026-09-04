import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parseDocument } from 'yaml';
import { withoutBootstrapTiming } from './npm-timing-workflow-cases.mjs';

const document = parseDocument(readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'));
assert.deepEqual(document.errors, []);
const workflow = withoutBootstrapTiming(document.toJS());
const packageDocument = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const bootstrapJobs = ['owned-data-quick', 'preflight', 'rust', 'adapter-platform', 'm2-platform', 'docs-publish'];
const nodeStep = {
  uses: 'actions/setup-node@820762786026740c76f36085b0efc47a31fe5020',
  with: { 'node-version': '22.22.1' },
};
const pnpmStep = {
  uses: 'pnpm/action-setup@0977fd99725f1db4007ccb2928dbb4e90d06cc86',
  with: { version: '11.18.0' },
};

function bootstrapOrder(candidate, metadata) {
  assert.equal(metadata.packageManager, 'pnpm@11.18.0');
  assert.equal(metadata.devEngines?.packageManager, undefined,
    'new package-manager metadata requires explicit automatic-cache review');
  const actualJobs = Object.entries(candidate.jobs).filter(([, job]) => job.steps.some(step =>
    /^(actions\/setup-node|pnpm\/action-setup)@/.test(step.uses ?? ''))).map(([id]) => id);
  assert.deepEqual(actualJobs, bootstrapJobs);
  for (const id of bootstrapJobs) {
    const steps = candidate.jobs[id].steps;
    const pnpm = steps.flatMap((step, index) => /^pnpm\/action-setup@/.test(step.uses ?? '') ? [index] : []);
    const nodes = steps.flatMap((step, index) => /^actions\/setup-node@/.test(step.uses ?? '') ? [index] : []);
    assert.equal(pnpm.length, 1, id);
    const index = pnpm[0];
    assert.deepEqual(steps[index], pnpmStep, id);
    assert.deepEqual(steps[index - 1], nodeStep, id);
    if (id === 'owned-data-quick') {
      assert.deepEqual(nodes, [index - 1], id);
    } else {
      assert.deepEqual(nodes, [index - 1, index + 1], id);
      assert.deepEqual(steps[index + 1], {
        ...nodeStep, with: { ...nodeStep.with, cache: 'pnpm' },
      }, id);
      assert.deepEqual(steps[index + 2], { run: 'pnpm install --frozen-lockfile' }, id);
    }
  }
}

test('CI pins bootstrap Node before pnpm and restores stores only after pnpm exists', () => {
  bootstrapOrder(workflow, packageDocument);
});

test('bootstrap toolchain and cache-order mutations fail closed in every setup job', () => {
  for (const id of bootstrapJobs) {
    const mutations = [
      (steps, index) => { steps.splice(index - 1, 1); },
      (steps, index) => { [steps[index - 1], steps[index]] = [steps[index], steps[index - 1]]; },
      (steps, index) => { steps[index - 1].uses = 'actions/setup-node@main'; },
      (steps, index) => { steps[index - 1].with['node-version'] = '22'; },
      (steps, index) => { steps[index - 1].with.cache = 'pnpm'; },
      (steps, index) => { steps[index - 1].if = 'false'; },
      (steps, index) => { steps[index - 1]['continue-on-error'] = true; },
      (steps, index) => { steps[index].uses = 'pnpm/action-setup@main'; },
      (steps, index) => { steps[index].with.version = 'latest'; },
      (steps, index) => { steps[index].with.run_install = true; },
      (steps, index) => { steps[index].if = 'false'; },
      (steps, index) => { steps[index]['continue-on-error'] = true; },
      steps => { steps.push(structuredClone(nodeStep)); },
      steps => { steps.push(structuredClone(pnpmStep)); },
    ];
    if (id !== 'owned-data-quick') mutations.push(
      (steps, index) => { steps.splice(index + 1, 1); },
      (steps, index) => { [steps[index], steps[index + 1]] = [steps[index + 1], steps[index]]; },
      (steps, index) => { delete steps[index + 1].with.cache; },
      (steps, index) => { steps[index + 1].with.cache = 'npm'; },
      (steps, index) => { steps[index + 1].with['node-version'] = 'latest'; },
      (steps, index) => { steps[index + 1].uses = 'actions/setup-node@main'; },
      (steps, index) => { steps[index + 1].if = 'false'; },
      (steps, index) => { steps[index + 1]['continue-on-error'] = true; },
      (steps, index) => { steps[index + 2].run = 'pnpm install'; },
    );
    for (const mutate of mutations) {
      const changed = structuredClone(workflow);
      const steps = changed.jobs[id].steps;
      mutate(steps, steps.findIndex(step => step.uses === pnpmStep.uses));
      assert.throws(() => bootstrapOrder(changed, packageDocument), `${id}: mutation must fail`);
    }
  }
  for (const metadata of [
    { ...packageDocument, packageManager: 'npm@11' },
    { ...packageDocument, packageManager: undefined },
    { ...packageDocument, devEngines: { packageManager: { name: 'npm' } } },
    { ...packageDocument, devEngines: { packageManager: [{ name: 'npm' }] } },
  ]) assert.throws(() => bootstrapOrder(workflow, metadata));
});
const aggregateNeeds = {
  adapter: ['preflight', 'adapter-platform'],
  m0: ['owned-data-quick', 'preflight', 'rust', 'adapter'],
  m2: ['m0', 'm2-platform'],
};
const matrixJobs = ['owned-data-quick', 'rust', 'adapter-platform', 'm2-platform'];
const outcomes = ['success', 'failure', 'cancelled', 'skipped', '', 'unknown', undefined];

function keys(value, expected) {
  assert.deepEqual(Object.keys(value).sort(), [...expected].sort());
}

// Interpret only the workflow's exact conjunction grammar, never execute a shell.
function aggregatePredicate(job, expectedNeeds) {
  keys(job, ['name', 'if', 'needs', 'runs-on', 'steps']);
  assert.equal(job.if, 'always()');
  assert.equal(job['runs-on'], 'ubuntu-latest');
  assert.deepEqual(job.needs, expectedNeeds);
  assert.equal(job.steps.length, 1);
  const step = job.steps[0];
  keys(step, ['name', 'env', 'run']);
  assert.equal(typeof step.run, 'string');
  const clauses = step.run.split(' && ');
  const variables = clauses.map(clause => {
    const match = /^test "\$([A-Z][A-Z0-9_]*)" = success$/.exec(clause);
    assert(match, `unsupported aggregate predicate: ${clause}`);
    return match[1];
  });
  assert.equal(new Set(variables).size, variables.length, 'duplicate predicate variable');
  keys(step.env, variables);
  const bindings = new Map(variables.map(variable => {
    const match = /^\$\{\{ needs\.([a-z][a-z0-9-]*)\.result \}\}$/.exec(step.env[variable]);
    assert(match, `unsupported result binding: ${variable}`);
    assert(job.needs.includes(match[1]), `undeclared dependency: ${match[1]}`);
    return [variable, match[1]];
  }));
  assert.deepEqual([...bindings.values()].sort(), [...job.needs].sort(), 'every dependency checked once');
  return results => variables.every(variable => results[bindings.get(variable)] === 'success');
}

function* combinations(names, prefix = {}) {
  if (names.length === 0) {
    yield prefix;
    return;
  }
  for (const outcome of outcomes) {
    yield* combinations(names.slice(1), { ...prefix, [names[0]]: outcome });
  }
}

function evaluateGraph(jobs, leaves) {
  const predicates = Object.fromEntries(Object.entries(aggregateNeeds)
    .map(([id, needs]) => [id, aggregatePredicate(jobs[id], needs)]));
  const results = { ...leaves };
  const active = new Set();
  function visit(id) {
    if (Object.hasOwn(results, id)) return results[id];
    assert(predicates[id], `missing authority: ${id}`);
    assert(!active.has(id), `cyclic aggregate dependency: ${id}`);
    active.add(id);
    const dependencies = Object.fromEntries(jobs[id].needs.map(need => [need, visit(need)]));
    active.delete(id);
    results[id] = predicates[id](dependencies) ? 'success' : 'failure';
    return results[id];
  }
  for (const id of Object.keys(predicates)) visit(id);
  return results;
}

test('CI starts independent authorities together with bounded preflight headroom', () => {
  assert.equal(workflow.jobs.preflight['timeout-minutes'], 15);
  assert.equal(workflow.jobs.preflight.if, undefined);
  assert.equal(workflow.jobs.preflight.needs, undefined);
  for (const id of matrixJobs) {
    const job = workflow.jobs[id];
    assert.equal(job.needs, undefined, `${id}: must start independently`);
    assert.equal(job.if, undefined, `${id}: authority must not be conditionally skipped`);
    assert.equal(job['continue-on-error'], undefined);
    assert.equal(job.strategy['fail-fast'], false);
    assert.deepEqual(job.strategy.matrix, { os: ['ubuntu-latest', 'windows-latest'] });
    assert.equal(job['runs-on'], '${{ matrix.os }}');
  }
});

test('actual aggregate predicates reject every Cartesian non-success result', () => {
  let checked = 0;
  for (const [id, needs] of Object.entries(aggregateNeeds)) {
    const predicate = aggregatePredicate(workflow.jobs[id], needs);
    for (const results of combinations(needs)) {
      assert.equal(predicate(results), needs.every(need => results[need] === 'success'),
        `${id}: ${JSON.stringify(results)}`);
      checked++;
    }
  }
  assert.equal(checked, 2499);
});

test('each OS authority and preflight must succeed through the actual aggregate graph', () => {
  const allSuccess = Object.fromEntries(['preflight', ...matrixJobs].map(id => [id, 'success']));
  assert.equal(evaluateGraph(workflow.jobs, allSuccess).m2, 'success');
  for (const preflight of outcomes) {
    const results = evaluateGraph(workflow.jobs, { ...allSuccess, preflight });
    for (const id of Object.keys(aggregateNeeds)) assert.equal(results[id], preflight === 'success' ? 'success' : 'failure');
  }
  for (const id of matrixJobs) {
    const operatingSystems = workflow.jobs[id].strategy.matrix.os;
    for (const legs of combinations(operatingSystems)) {
      // Any non-success matrix leg must be represented by a non-success job result.
      // The aggregate interpreter checks the actual workflow result binding, not a shell stub.
      const failedLeg = operatingSystems.find(os => legs[os] !== 'success');
      const result = failedLeg === undefined ? 'success' : legs[failedLeg];
      const evaluated = evaluateGraph(workflow.jobs, { ...allSuccess, [id]: result });
      assert.equal(evaluated.m2, failedLeg === undefined ? 'success' : 'failure');
      if (id === 'adapter-platform') assert.equal(evaluated.adapter, evaluated.m2);
      if (id !== 'm2-platform') assert.equal(evaluated.m0, evaluated.m2);
    }
  }
});

test('aggregate grammar and dependency mutations fail closed', () => {
  for (const [id, needs] of Object.entries(aggregateNeeds)) {
    for (const mutate of [
      job => { job.if = 'success()'; },
      job => { delete job.if; },
      job => { job['continue-on-error'] = true; },
      job => { job.needs = job.needs.slice(1); },
      job => { job.steps[0].if = 'success()'; },
      job => { job.steps[0]['continue-on-error'] = true; },
      job => { job.steps[0].run += ' || true'; },
      job => { job.steps[0].run += '; exit 0'; },
      job => { job.steps[0].run = job.steps[0].run.replace('= success', '!= failure'); },
      job => { job.steps[0].run = job.steps[0].run.split(' && ').slice(1).join(' && '); },
      job => { job.steps[0].run += ` && ${job.steps[0].run.split(' && ')[0]}`; },
      job => { job.steps[0].run = 'true'; },
      job => { job.steps[0].env[Object.keys(job.steps[0].env)[0]] = 'success'; },
      job => { job.steps[0].env[Object.keys(job.steps[0].env)[0]] = '${{ needs.missing.result }}'; },
      job => { job.steps[0].env[Object.keys(job.steps[0].env)[0]] = Object.values(job.steps[0].env)[1]; },
      job => { job.steps[0].env[Object.keys(job.steps[0].env)[0]] += ' || success'; },
      job => { job.steps.push({ run: 'true' }); },
    ]) {
      const changed = structuredClone(workflow.jobs[id]);
      mutate(changed);
      assert.throws(() => aggregatePredicate(changed, needs), `${id}: mutation must fail`);
    }
  }
});

test('parallel scheduling preserves all other pinned workflow authority', () => {
  bootstrapOrder(workflow, packageDocument);
  const original = structuredClone(workflow);
  for (const id of bootstrapJobs) {
    const steps = original.jobs[id].steps;
    const index = steps.findIndex(step => step.uses === pnpmStep.uses);
    if (id === 'owned-data-quick') {
      [steps[index - 1], steps[index]] = [steps[index], steps[index - 1]];
    } else {
      steps.splice(index - 1, 1);
    }
  }
  original.jobs.preflight['timeout-minutes'] = 10;
  for (const id of ['rust', 'adapter-platform', 'm2-platform']) {
    original.jobs[id].needs = 'preflight';
  }
  original.jobs.adapter.needs = 'adapter-platform';
  delete original.jobs.adapter.steps[0].env.PREFLIGHT_RESULT;
  original.jobs.adapter.steps[0].run = 'test "$PLATFORM_RESULT" = success';
  function canonical(value) {
    if (Array.isArray(value)) return value.map(canonical);
    if (value !== null && typeof value === 'object') return Object.fromEntries(
      Object.keys(value).sort().map(key => [key, canonical(value[key])]),
    );
    return value;
  }
  const digest = createHash('sha256').update(JSON.stringify(canonical(original))).digest('hex');
  assert.equal(digest, 'e17c35d5e29afbf6bfee1710ce91da6826f6562938a3213cc63d5a81a54392a0');
});
