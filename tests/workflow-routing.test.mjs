import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import {
  dirname, relative, resolve, sep,
} from 'node:path';
import test from 'node:test';
import { parseDocument } from 'yaml';

import {
  CONTRACT_LANES,
  classifyGitDiff,
  classifyWorkflowPaths,
  formatWorkflowOutputs,
} from '../scripts/classify-workflow-paths.mjs';

const root = resolve(import.meta.dirname, '..');
const workflow = (name) => {
  const parsed = parseDocument(readFileSync(resolve(root, '.github/workflows', name), 'utf8'));
  assert.deepEqual(parsed.errors, []);
  return parsed.toJS();
};
const ci = workflow('ci.yml');
const documentation = workflow('documentation.yml');
const none = Object.fromEntries(CONTRACT_LANES.map((lane) => [lane, false]));
const all = Object.fromEntries(CONTRACT_LANES.map((lane) => [lane, true]));

test('representative paths select only their owning optional contract lanes', () => {
  assert.deepEqual(classifyWorkflowPaths(['docs/ROADMAP.md']), none);
  assert.deepEqual(classifyWorkflowPaths(['crates/zryna-driver/src/lib.rs']), none);
  assert.deepEqual(classifyWorkflowPaths(['crates/zryna-diagnostics/src/lib.rs']), {
    ...none, diagnostics: true, provider_v4: true,
  });
  assert.deepEqual(classifyWorkflowPaths(['tests/package-release-v1/validation.test.mjs']), {
    ...none, package_release: true,
  });
  assert.deepEqual(classifyWorkflowPaths(['tests/package-source-trust.test.mjs']), {
    ...none, package_release: true,
  });
  assert.deepEqual(classifyWorkflowPaths(['adapters/typescript-6/src/worker-v4.mjs']), {
    ...none, provider_v4: true,
  });
  assert.deepEqual(classifyWorkflowPaths(['spec/wit/capability-profiles-v1/worlds.wit']), {
    ...none, wit: true,
  });
  assert.deepEqual(classifyWorkflowPaths(['spec/interop/JS_WASM_ADAPTERS_V1.md']), {
    ...none, wit: true,
  });
  assert.deepEqual(classifyWorkflowPaths(['spec/language/CROSS_TARGET_PROFILES_V1.md']), {
    ...none, wit: true,
  });
  assert.deepEqual(classifyWorkflowPaths([
    'tests/diagnostics-protocol-v2.test.mjs',
    'schemas/zryna-package-release-v1.schema.json',
  ]), { ...none, diagnostics: true, package_release: true });

  const ownershipCases = [
    ['schemas/zryna-diagnostics-v2.schema.json', ['diagnostics']],
    ['spec/diagnostics/STRUCTURED_DIAGNOSTICS_V2.md', ['diagnostics']],
    ['crates/zryna-source/src/lib.rs', ['diagnostics', 'provider_v4']],
    ['crates/zryna-frontend/src/lib.rs', ['provider_v4']],
    ['crates/zryna-syntax/src/v4.rs', ['provider_v4']],
    ['schemas/zryna-syntax-v4.schema.json', ['provider_v4']],
    ['scripts/check-provider-conformance-v4.mjs', ['provider_v4']],
    ['tests/provider-conformance-v4/fixtures/positive.zry', ['provider_v4']],
    ['schemas/zryna-package-release-v1.schema.json', ['package_release']],
    ['scripts/package-release/validate.mjs', ['package_release']],
    ['spec/package/PACKAGE_RELEASE_V1.md', ['package_release']],
    ['tests/package-source-trust.test.mjs', ['package_release']],
    ['schemas/zryna-capability-request-v1.schema.json', ['wit']],
    ['scripts/wit-capabilities/validate.mjs', ['wit']],
    ['tests/wit-capability-v1/fixtures/command-granted.json', ['wit']],
    ['tests/js-wasm-adapter-v1-vectors.json', ['wit']],
    ['spec/libraries/MINIMAL_CORE_HOST_V0.md', ['wit']],
  ];
  for (const [changedPath, lanes] of ownershipCases) {
    assert.deepEqual(classifyWorkflowPaths([changedPath]), {
      ...none,
      ...Object.fromEntries(lanes.map((lane) => [lane, true])),
    }, changedPath);
  }
});

test('local contract dependency graphs remain owned by their lanes', () => {
  const result = spawnSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], {
    cwd: root,
    encoding: 'utf8',
    shell: false,
    windowsHide: true,
  });
  assert.equal(result.error, undefined, 'cargo metadata spawn');
  assert.equal(result.status, 0, `cargo metadata status: ${result.stderr}`);
  const metadata = JSON.parse(result.stdout);
  const packages = new Map(metadata.packages.map((entry) => [entry.name, entry]));
  const rootsByLane = {
    diagnostics: ['zryna-diagnostics'],
    provider_v4: ['zryna-frontend'],
  };

  for (const [lane, laneRoots] of Object.entries(rootsByLane)) {
    const pending = [...laneRoots];
    const visited = new Set();
    while (pending.length > 0) {
      const packageName = pending.pop();
      if (visited.has(packageName)) continue;
      visited.add(packageName);
      const packageMetadata = packages.get(packageName);
      assert(packageMetadata, `${lane} package metadata: ${packageName}`);
      const packageDirectory = relative(root, dirname(packageMetadata.manifest_path))
        .split(sep).join('/');
      assert.equal(
        classifyWorkflowPaths([`${packageDirectory}/src/lib.rs`])[lane],
        true,
        `${lane} must own local dependency ${packageName}`,
      );
      for (const dependency of packageMetadata.dependencies) {
        if (typeof dependency.path === 'string') pending.push(dependency.name);
      }
    }
  }
});

test('manual, shared, unknown and malformed changes fail safe to every lane', () => {
  assert.deepEqual(classifyWorkflowPaths([], { full: true }), all);
  for (const changedPath of [
    'Cargo.lock',
    '.github/workflows/ci.yml',
    'scripts/classify-workflow-paths.mjs',
    'future-root/contract.json',
    'scripts/future-contract-helper.mjs',
    'tests/future-contract-helper.test.mjs',
    'schemas/future-contract.schema.json',
    'spec/future/CONTRACT.md',
    '../outside',
    'docs\\ROADMAP.md',
  ]) assert.deepEqual(classifyWorkflowPaths([changedPath]), all, changedPath);
  assert.equal(formatWorkflowOutputs(all),
    'diagnostics=true\npackage_release=true\nprovider_v4=true\nwit=true\n');
});

test('git diff execution fails safe without shell pipeline semantics', () => {
  const base = 'a'.repeat(40);
  const head = 'b'.repeat(40);
  const invoke = (result) => classifyGitDiff(base, head, (executable, args, options) => {
    assert.equal(executable, 'git');
    assert.deepEqual(args, ['diff', '--name-only', '--no-renames', '-z', base, head]);
    assert.equal(options.shell, false);
    assert.equal(options.maxBuffer, 2 * 1024 * 1024);
    return result;
  });
  assert.deepEqual(invoke({ status: 9, stdout: Buffer.alloc(0) }), all);
  assert.deepEqual(invoke({ status: null, error: new Error('could not start') }), all);
  assert.deepEqual(invoke({ status: 0, stdout: Buffer.from('docs/ROADMAP.md') }), all);
  assert.deepEqual(invoke({ status: 0, stdout: Buffer.from('docs/ROADMAP.md\0') }), none);
  assert.deepEqual(classifyGitDiff('main', head, () => assert.fail('must not spawn')), all);
});

test('CI retains every protected pull-request context and one manual full entry point', () => {
  assert.deepEqual(ci.on, {
    pull_request: null,
    workflow_dispatch: { inputs: { structure_base: {
      description: 'Full trusted ancestor commit for repository structure validation',
      required: true,
      type: 'string',
    } } },
  });
  assert.equal(ci.env.ZRYNA_STRUCTURE_BASE,
    "${{ github.event_name == 'pull_request' && github.event.pull_request.base.sha || inputs.structure_base }}");
  const contexts = [
    ...ci.jobs.rust.strategy.matrix.os.map((os) => ci.jobs.rust.name.replace('${{ matrix.os }}', os)),
    ci.jobs.adapter.name,
    ci.jobs.m0.name,
    ci.jobs.m2.name,
    ci.jobs.m3.name,
  ];
  assert.deepEqual(contexts, [
    'rust (ubuntu-latest)',
    'rust (windows-latest)',
    'adapter',
    'm0',
    'm2',
    'm3',
  ]);
  assert.equal(ci.concurrency['cancel-in-progress'], true);
  assert.match(ci.concurrency.group, /pull_request\.number/);
  assert.match(ci.jobs['route-contracts'].steps.at(-1).run, /workflow-paths\.mjs --all/);
  assert.deepEqual(ci.jobs.m0.needs,
    ['owned-data-quick', 'preflight', 'rust', 'adapter', 'route-contracts']);
  assert.match(ci.jobs.m0.steps[0].run, /ROUTING_RESULT/);
});

test('classification failure runs all optional lanes and each matrix uses its exact output', () => {
  const route = ci.jobs['route-contracts'];
  const command = route.steps.at(-1).run;
  assert.match(command, /workflow-paths\.mjs \\\n\s+--git-diff "\$BASE_SHA" "\$HEAD_SHA"/);
  assert.doesNotMatch(command, /git diff|\|\s*node/);
  for (const [id, output] of [
    ['diagnostics-contract', 'diagnostics'],
    ['package-release-contract', 'package_release'],
    ['provider-conformance-v4', 'provider_v4'],
    ['wit-capability-contract', 'wit'],
  ]) {
    const job = ci.jobs[id];
    assert.equal(job.needs, 'route-contracts');
    assert.equal(job.if, `needs.route-contracts.outputs.${output} == 'true'`);
    assert.deepEqual(job.strategy.matrix.os, ['ubuntu-latest', 'windows-latest']);
    assert.equal(job['continue-on-error'], undefined);
  }
});

test('consolidation preserves every prior contract command and pinned action', () => {
  const commands = (id) => ci.jobs[id].steps.flatMap((step) => step.run ? [step.run] : []);
  assert.deepEqual(commands('diagnostics-contract'), [
    'pnpm install --frozen-lockfile',
    'pnpm diagnostics:contract',
    'cargo test --locked -p zryna-diagnostics',
    'cargo clippy --locked -p zryna-diagnostics --all-targets -- -D warnings',
    'cargo fmt --all -- --check',
  ]);
  assert.deepEqual(commands('package-release-contract'), [
    'pnpm install --frozen-lockfile',
    'pnpm package:contract',
  ]);
  assert.deepEqual(commands('provider-conformance-v4'), [
    'pnpm install --frozen-lockfile',
    'pnpm provider:conformance:v4',
  ]);
  assert.deepEqual(commands('wit-capability-contract'), [
    'pnpm install --frozen-lockfile',
    'pnpm wit:contract',
  ]);
  for (const id of [
    'diagnostics-contract',
    'package-release-contract',
    'provider-conformance-v4',
    'wit-capability-contract',
  ]) {
    const uses = ci.jobs[id].steps.flatMap((step) => step.uses ? [step.uses] : []);
    assert(uses.includes('actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1'));
    assert(uses.includes('actions/setup-node@820762786026740c76f36085b0efc47a31fe5020'));
    assert(uses.includes('pnpm/action-setup@0977fd99725f1db4007ccb2928dbb4e90d06cc86'));
    assert(!uses.some((use) => use.endsWith('@main')));
  }
});

test('only CI handles pull requests and every superseded pull-request run cancels', () => {
  const names = readdirSync(resolve(root, '.github/workflows')).sort();
  assert.deepEqual(names, ['ci.yml', 'documentation.yml']);
  for (const name of names) {
    const candidate = workflow(name);
    if (!Object.hasOwn(candidate.on, 'pull_request')) continue;
    assert.equal(candidate.concurrency['cancel-in-progress'], true, name);
    assert.match(candidate.concurrency.group, /pull_request\.number/, name);
  }
});

test('main runs only documentation validation and publication with short retention', () => {
  assert.deepEqual(documentation.on, { push: { branches: ['main'] } });
  assert.deepEqual(Object.keys(documentation.jobs), ['docs-publish']);
  assert.equal(documentation.concurrency['cancel-in-progress'], true);
  const publisher = documentation.jobs['docs-publish'];
  assert.equal(publisher.needs, undefined);
  assert(publisher.steps.some((step) => step.run === 'pnpm docs:check'));
  const upload = publisher.steps.at(-1);
  assert.equal(upload.uses,
    'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02');
  assert.equal(upload.with['retention-days'], 7);
});
