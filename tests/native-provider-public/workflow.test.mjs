import assert from 'node:assert/strict';
import test from 'node:test';
import {readFileSync} from 'node:fs';
import {parse} from 'yaml';

const text=readFileSync(new URL('../../.github/workflows/native-provider-public.yml',import.meta.url),'utf8');
const workflow=parse(text),job=workflow.jobs.candidate,steps=job.steps;
const named=name=>steps.find(step=>step.name===name);

test('current candidate workflow binds both physical hosts, exact checkout and finite budgets',()=>{
 assert.deepEqual(job.strategy.matrix.os,['ubuntu-24.04','windows-2022']);assert.equal(job.strategy['fail-fast'],false);assert.equal(job['timeout-minutes'],60);
 const checkout=named('Checkout the actual candidate');assert.equal(checkout.with.ref,'${{ github.event.pull_request.head.sha || github.sha }}');assert.equal(checkout.with['persist-credentials'],false);assert.equal(checkout.with['fetch-depth'],0);
 assert.equal(steps.filter(step=>step.uses?.startsWith('actions/checkout@')).length,1);assert(!text.includes('ACTIVATION_CONSUMER_SHA'));
 assert.equal(steps.find(step=>step.uses?.startsWith('dtolnay/rust-toolchain@')).with.toolchain,'1.97.1');assert.equal(steps.find(step=>step.uses?.startsWith('actions/setup-node@')).with['node-version'],'22.22.1');
 assert.equal(workflow.permissions.contents,'read');
});
test('complete actual corpus and original verifier gates cannot become skipped synthetic passes',()=>{
 const corpus=named('Compare every frozen worker and public BUILD root');assert(corpus.run.includes('scripts/run-native-provider-public.mjs'));assert(corpus.run.includes('--lane all'));assert(corpus.run.includes('--cli "$cli"'));assert(corpus.run.includes('--node "$env:ZRYNA_NODE"'));
 for(const name of ['Run native worker framing and verifier controls','Run actual diagnostic, span and resource boundary differential','Run source-bound IR and backend differential controls']){const step=named(name);assert(step?.run.includes('cargo test --locked'));assert(step.run.includes('$LASTEXITCODE'));assert(!step.if);assert(!step['continue-on-error']);}
 assert(named('Run actual diagnostic, span and resource boundary differential').run.includes('--include-ignored'));
 assert(named('Run actual source architecture authority').run.includes('architecture check --root'));
 assert(named('Validate frozen registry, transport controls and workflow contract').run.includes('tests/native-provider-public/workflow.test.mjs'));
 assert(!steps.some(step=>step['continue-on-error']===true));
});
test('actual original captures and failures upload even when qualification fails',()=>{
 const upload=named('Preserve actual candidate commands, outputs and failures');assert.equal(upload.if,'always()');assert.equal(upload.with['if-no-files-found'],'error');assert.equal(upload.with.path,'${{ runner.temp }}/native-provider-public');assert.equal(upload.with['retention-days'],7);
 for(const step of steps.filter(step=>step.uses))assert(/@[a-f0-9]{40}$/.test(step.uses),'whole action revision');
});

test('IR integration and filtered cost targets cannot silently become zero-test library passes',()=>{
 const ir=named('Run source-bound IR and backend differential controls');assert(ir.run.includes('--test native_activation_parity'));assert(!ir.run.includes('--lib native_activation_parity'));assert(ir.run.includes('verify-rust-log.mjs'));
 const full=named('Run complete frozen verified IR parity');assert(full.run.includes('--features native-provider-qualification --test native_public_ir_parity'));assert(full.run.includes('--test-threads=1'));assert(full.run.includes('$env:ZRYNA_NATIVE_CLI = (Resolve-Path'));assert(full.run.includes('verify-rust-log.mjs'));
 const cost=named('Run provider identity and physical comparison budget controls');for(const filter of ['native_provider::tests::','module_closure::tests::','module_closure_tests::provider_work::','ownership_closure::tests::'])assert(cost.run.includes(filter));assert(cost.run.includes('--lib $filter'));assert(cost.run.includes('verify-rust-log.mjs'));
 for(const step of steps.filter(step=>step.run?.includes('cargo test ')))assert(step.run.includes('verify-rust-log.mjs'),'all actual Rust filters reject zero matched tests');
});

test('public runtimes and all logical fault sources execute both current frontends',()=>{
 const runtime=named('Run ordinary public runtime corpus and exact cleanup parity');for(const target of ['--test m3_public','--test m2_conformance','--test cli m1_'])assert(runtime.run.includes(target));
 const fault=named('Run all original logical and physical faults through both frontends');assert(fault.run.includes('--lib ownership_commands::conformance::native_fault_parity::'));assert(fault.run.includes('$required = if ($IsWindows) { 1 } else { 2 }'));assert(fault.run.includes('verify-rust-log.mjs'));assert(!fault.if);assert(!fault['continue-on-error']);
});
