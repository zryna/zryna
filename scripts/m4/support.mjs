import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  expectedRegistrySha256, loadAndValidateM4Conformance,
} from '../check-m4-conformance.mjs';
import { loadReviewedLimitContracts } from './limit-contracts.mjs';

export const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const BEGIN = 'zryna.m4.support.v1:begin';
const END = 'zryna.m4.support.v1:end';

export function projectM4Support(registry, registrySha256 = expectedRegistrySha256,
  plan = loadReviewedLimitContracts(registry, workspaceRoot)) {
  assert.match(registrySha256, /^[a-f0-9]{64}$/u);
  const hosts = [...new Map(registry.rows.map(row => [`${row.host.os}:${row.host.architecture}`, {
    os: row.host.os, architecture: row.host.architecture,
    portabilityClaim: row.host.portabilityClaim,
  }])).values()];
  return {
    schema: 'zryna.m4.support.v1',
    authority: 'static-contract',
    registrySha256,
    authenticatedEvidence: 'not-evaluated',
    toolchains: registry.toolchains,
    hosts,
    witClosure: registry.scope.witClosure,
    requiredCaseKindsAuthority: registry.scope.requiredCaseKindsAuthority,
    contractPlan: registry.contractPlan,
    modeRequirements: plan.modeRequirements,
    limitContracts: plan.contracts,
    limitExclusions: plan.exclusions,
    sources: registry.sources,
    requiredExternalEvidence: registry.scope.requiredExternalEvidence,
    lifecycleExemptions: registry.scope.lifecycleExemptions,
    rows: registry.rows.map(row => ({
      id: row.id, profile: row.profile, mode: row.mode, host: row.host,
      ownerIssues: row.ownerIssues, contract: row.contract, selector: row.selector,
      identities: row.identities,
      capabilities: row.capabilities, artifacts: row.artifacts, limits: row.limits,
      proofCases: row.cases.map(id => {
        const entry = registry.cases.find(item => item.id === id);
        assert(entry, `missing support proof ${id}`);
        return entry;
      }),
      requiredGates: row.requiredGates,
      proofGaps: row.limits.filter(limit => ['partial', 'unresolved'].includes(limit.mapping.kind)).map(limit => ({
        limit: limit.id, contractId: limit.contractId, mappingKind: limit.mapping.kind,
        requirements: limit.mapping.requirements,
      })),
      authenticatedExecution: 'not-evaluated',
      exclusions: row.exclusions,
    })),
    exclusions: registry.exclusions,
  };
}

export function canonicalSupportBytes(projection) {
  return Buffer.from(`${JSON.stringify(projection, null, 2)}\n`, 'utf8');
}

export function compactCompiledSupportBytes(projection) {
  return Buffer.from(`${JSON.stringify(projection)}\n`, 'utf8');
}

export function validateSupportProjection(bytes, expected) {
  assert(Buffer.from(bytes).equals(canonicalSupportBytes(expected)),
    'support projection must equal the complete canonical checked registry projection');
  return expected;
}

export function extractM4SupportFence(markdown) {
  assert.equal(markdown.split(BEGIN).length, 2, 'one support fence start');
  assert.equal(markdown.split(END).length, 2, 'one support fence end');
  const lines = markdown.split('\n');
  assert.equal(lines.filter(line => line === BEGIN).length, 1, 'standalone plain-text start');
  assert.equal(lines.filter(line => line === END).length, 1, 'standalone plain-text end');
  assert(markdown.indexOf(BEGIN) < markdown.indexOf(END), 'support fence markers must be ordered');
  const fenced = markdown.split(BEGIN)[1].split(END)[0];
  const match = /^\n\n```json\n([\s\S]*\n)```\n\n$/u.exec(fenced);
  assert(match, 'canonical JSON support fence');
  return Buffer.from(match[1], 'utf8');
}

export function renderM4SupportPage(projection) {
  const rows = projection.rows.map(row =>
    `| \`${row.id}\` | \`${row.profile}\` / \`${row.mode}\` | \`${row.host.baseline}\` ${row.host.architecture} | \`${row.selector.command}\` / \`${row.selector.target ?? "none"}\` | ${row.proofGaps.length} |`);
  return `# Fixed public M4 support contract

This page and \`zryna support --json\` project one checked registry. The registry SHA-256 is
\`${projection.registrySha256}\`. The matrix defines six bounded modes on the two
exact hosted contracts below. It is a static contract, not authenticated execution or publication.
Proof mappings and source limits are requirements; their presence does not show that tests ran.
The complete canonical JSON below is the authority for every row, identity, limit and exclusion.
Its independently pinned source-contract plan distinguishes emission, loader and runtime
boundaries, owning guard proofs, public bindings and reviewed public-input exclusions.

| Row | Profile / mode | Host contract | Selector | Source proof gaps |
| --- | --- | --- | --- | --- |
${rows.join('\n')}

Each row states its precise Linux x86-64 or Windows x64 runner baseline. An unresolved
runner alias requires the actual observed baseline in authenticated evidence; it cannot certify
a concrete OS version. The Windows row makes no general desktop Windows claim. Browser execution is limited to
the pinned Chrome fixture; its Node companion is separate evidence. Default components are
build-only. Browser synchronous calls use post-hoc deadlines and cannot be preempted.
Status/clock server modes are finite loopback source-checkout arrangements, with distinct
listener approval and one-read guest clock approval. No source clock intrinsic is added.
Command modes share bounded H1 semantics; only a newly complete content-verified installation
qualifies for the installed mode. Content verification does not authenticate a signed release.

Installed component/browser/server, existing immutable beta H1, other browsers/platforms,
general WASI host authority, signed public release authentication and production admission
remain explicit exclusions. Specified-only typed-JS/library/resource/package interfaces are
prerequisites and do not become executable support. M0–M3 contracts remain unchanged.

## Runnable examples

- [Browser scalar component](BROWSER_COMPONENT_V1.md): checkout build and checked-in page.
- [Command walkthrough](WASI_COMMAND_GETTING_STARTED.md): pure/error and one literal private key;
  [execution manifest](WASI_COMMAND_MANIFEST_V1.md) retains actual results and teardown.
- [Finite server status](WASI_SERVER_STATUS_V1.md) and
  [separately approved clock/status](WASI_SERVER_CLOCK_STATUS_V1.md): checkout-only loopback.
- [Command sources](../examples/wasi-command/README.md),
  [status example](../examples/wasi-server/README.md), and
  [denied/granted clock example](../examples/wasi-server/CLOCK_EXAMPLE.md).

## Separate evidence authority

[M4 conformance](M4_CONFORMANCE.md) defines exact source/tree, actual test accounting, both
hosts, artifact reproducibility, capability denial, cleanup, required hosted checks and live
publication. Every row remains \`not-evaluated\` by this static report. Unmapped source proof
pairs remain explicit partial or unresolved gaps and retain their missing requirements.
Mapped guards, source bindings and typed exclusions still require authenticated execution
evidence. An excluded public input boundary does not certify an exact-limit runtime pass.
No containing commit or tree is embedded here: independent receipts bind this contract digest
to their actual candidate source, avoiding a source/hash cycle. A PR receipt cannot certify
the eventual merge commit or deployment.

## Canonical support JSON

The JSON is part of this Markdown document in \`zryna.docs.bundle.v1\`. The existing bundle
manifest authenticates document bytes and source provenance; there is no unmanifested sidecar.
An independently authenticated successful documentation run and manifest digest are still
required before website import. Source presence alone does not authenticate publication.

${BEGIN}

\`\`\`json
${canonicalSupportBytes(projection).toString('utf8')}\`\`\`

${END}
`;
}

export function checkSupportFiles(root = workspaceRoot) {
  const registry = loadAndValidateM4Conformance(undefined, { root });
  const projection = projectM4Support(registry, expectedRegistrySha256,
    loadReviewedLimitContracts(registry, root));
  const compiled = readFileSync(path.join(root, 'apps/zryna/src/m4_support.json'));
  assert(compiled.equals(compactCompiledSupportBytes(projection)),
    'compiled support must retain the complete exact compact projection');
  const page = readFileSync(path.join(root, 'docs/M4_SUPPORT.md'), 'utf8');
  validateSupportProjection(extractM4SupportFence(page), projection);
  assert.equal(page, renderM4SupportPage(projection), 'generated support page must be fresh');
  return projection;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  assert(args.length === 1 && ['--check', '--write'].includes(args[0]),
    'usage: node scripts/m4/support.mjs --check|--write');
  if (args[0] === '--write') {
    const projection = projectM4Support(loadAndValidateM4Conformance());
    writeFileSync(path.join(workspaceRoot, 'apps/zryna/src/m4_support.json'), compactCompiledSupportBytes(projection));
    writeFileSync(path.join(workspaceRoot, 'docs/M4_SUPPORT.md'), renderM4SupportPage(projection));
  }
  const projection = checkSupportFiles();
  process.stdout.write(`M4 static support projection: ${projection.rows.length} rows; execution not evaluated\n`);
}
