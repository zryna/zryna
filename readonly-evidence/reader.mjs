// Trusted read-only proof reader. Own stdout is a fresh reader observation; original
// tests, processes and shared job logs remain separately retained original materials.
import assert from 'node:assert/strict';
import {readFileSync,lstatSync,writeFileSync,readdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {join,resolve} from 'node:path';
import {release} from 'node:os';
const [input,expectedSha,output,authenticationPath]=process.argv.slice(2);assert.equal(process.argv.length,6);
assert.equal(process.version,'v22.22.1');assert.equal(process.arch,'x64');assert(['linux','win32'].includes(process.platform));
const platform=process.platform==='win32'?'windows':'linux',startedAt=new Date().toISOString();
const sha=b=>createHash('sha256').update(b).digest('hex');const corpusBytes=readFileSync(input);assert.equal(sha(corpusBytes),expectedSha);assert(corpusBytes.length<=262144);
const corpus=JSON.parse(corpusBytes);assert.equal(corpus.format,'private.m6-readonly-evidence-corpus.v1');assert.equal(corpus.platform,platform);
assert.equal(corpus.source.commit,'5d79dba18117a7ff41ff08d328aa94838c0fce9c');assert.equal(corpus.source.tree,'78f42e37111204b1fb20a2363fc41fc23e62b8ef');
const authentication=JSON.parse(readFileSync(authenticationPath));assert.equal(authentication.source.commit,corpus.source.commit);assert.equal(authentication.source.tree,corpus.source.tree);assert.equal(authentication.fullCi.runId,39105995385);assert.equal(authentication.fullCi.conclusion,'success');assert.equal(authentication.fullCi.jobs,42);assert.deepEqual(authentication.artifacts,corpus.artifacts.map(a=>({id:a.id,bytes:a.size_in_bytes,sha256:a.digest.slice(7),runId:a.workflow_run.id,head:a.workflow_run.head_sha})));
const root=resolve(input,'..'),files=new Map();let total=0;
for(const row of corpus.files){assert(/^[a-zA-Z0-9_./-]+$/.test(row.path)&&!row.path.split('/').includes('..'));assert(!files.has(row.path));const p=join(root,row.path),s=lstatSync(p);assert(s.isFile()&&!s.isSymbolicLink());const b=readFileSync(p);assert.equal(b.length,row.bytes);assert.equal(sha(b),row.sha256);total+=b.length;assert(total<=32*1024*1024);files.set(row.path,b)}
const get=p=>{assert(files.has(p),'missing '+p);return files.get(p)};const json=p=>JSON.parse(get(p));
const clean=json('native/native-tooling-preparation/CHECKOUT_OBSERVATION.json');assert.equal(clean.commit,corpus.source.commit);assert.equal(clean.tree,corpus.source.tree);
const nativeCommands=[];
for(const family of ['PROJECT','LSP','FORMATTER']){const result=json('native/native-tooling-preparation/RECORDED_'+family+'/RESULT.json');assert.equal(result.failure,null);assert.equal(result.finalIdentityFailure,null);assert.equal(result.source.commit,corpus.source.commit);
 for(const observation of result.observations){assert.equal(observation.cleanupConfirmed,true);assert.equal(observation.ownerError,null);assert(observation.targetExits.every(e=>e.status===0));const stem='native/native-tooling-preparation/RECORDED_'+family+'/commands/'+observation.name;
  for(const stream of ['stdout','stderr']){const b=get(stem+'.'+stream);assert.equal(b.length,observation.pins[stream].bytes);assert.equal(sha(b),observation.pins[stream].sha256)}
  const owner=json(stem+'.json');assert.equal(owner.cleanupConfirmed,true);assert.equal(owner.ownerExit.status,0);assert.equal(owner.complete.failures.length,0);nativeCommands.push({family,name:observation.name,stem,stdout:get(stem+'.stdout').toString()})
 }}
const job=json('canonical/JOB.json');assert.equal(job.head_sha,corpus.source.commit);assert.equal(job.run_id,39105995385);assert.equal(job.conclusion,'success');
const log=get('canonical/ACTUAL_JOB.log').toString(),lines=log.split(/\r?\n/);
const proof={};const remember=(id,facts)=>{assert(!proof[id]);assert(facts.length>0);proof[id]=facts;return id};
const rust=(id,names)=>remember(id,names.map(name=>{const found=nativeCommands.flatMap(c=>c.stdout.split(/\r?\n/).map((line,index)=>({c,line,index}))).filter(v=>v.line==='test '+name+' ... ok');assert(found.length>0,'missing successful Rust observation '+name);return {kind:'original-rust-test-observation',name,observations:found.map(v=>({path:v.c.stem+'.stdout',line:v.index+1}))}}));
const tap=(id,titles)=>remember(id,titles.map(title=>{const found=[];lines.forEach((line,index)=>{if(new RegExp('Z ok [0-9]+ - '+title.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')+'$').test(line))found.push(index+1)});assert(found.length>0,'missing successful TAP observation '+title);assert(!lines.some(l=>l.includes('not ok ')&&l.endsWith(' - '+title)));return {kind:'original-tap-test-observation',title,path:'canonical/ACTUAL_JOB.log',lines:found}}));
const accept=json('accept/acceptance.json');assert.equal(accept.sourceCommit,corpus.source.commit);assert.equal(accept.runtimePathIsolated,true);assert.equal(accept.environment.platform,process.platform);
const setup=json('setup/candidate-receipt.json'),reproduction=json('setup/reproduction.json');assert.equal(setup.sourceCommit,corpus.source.commit);assert.equal(setup.sourceTree,corpus.source.tree);assert.equal(setup.status,'review-candidate');assert.equal(setup.productionAdmission,'forbidden');assert.equal(reproduction.sourceCommit,corpus.source.commit);
const editor=json('editor/m6-installed-editor/RESULT.json'),host=json('editor/m6-installed-editor/host-result.json');assert.equal(editor.source.commit,corpus.source.commit);assert.equal(editor.source.tree,corpus.source.tree);assert.equal(editor.accepted,true);assert.equal(editor.failed,0);assert.equal(editor.skipped,0);assert.equal(host.passed,true);assert.equal(host.platform,process.platform);assert.equal(host.editorVersion,'1.138.0');assert.equal(host.checks.length,9);
const editorOwner=json('editor/m6-installed-editor/commands/editor-process.json');assert.equal(editorOwner.cleanupConfirmed,true);assert.equal(editorOwner.ownerExit.status,0);assert.equal(editorOwner.complete.failures.length,0);
const supplemental=json('installed_supplement/captured/RESULT.json'),supplementOwner=json('installed_supplement/FIXED_SUPPLEMENT.json');assert.equal(supplemental.status,'captured-fixed-supplement-pass');assert.equal(supplemental.sourceCommit,corpus.source.commit);assert.equal(supplementOwner.cleanupConfirmed,true);assert.equal(supplementOwner.ownerExit.status,0);assert.equal(supplementOwner.complete.failures.length,0);
for(const [p,n,h] of [['installed_supplement/captured/client.stdin','requestBytes','requestSha256'],['installed_supplement/captured/server.stdout','serverStdoutBytes','serverStdoutSha256'],['installed_supplement/captured/server.stderr','serverStderrBytes','serverStderrSha256']]){assert.equal(get(p).length,supplemental[n]);assert.equal(sha(get(p)),supplemental[h])}
const field=(id,path,assertion,check)=>{const value=json(path);assert(check(value),'failed observed field '+id);return remember(id,[{kind:'original-json-observation',path,assertion}])};
const cases={};const gate=(family,mapping)=>{cases[family+'-'+platform]=Object.entries(mapping).map(([id,proofId])=>({id,passed:proof[proofId].length,failed:0,skipped:0,proofId}))};
gate('project',{
 creation:rust('project-creation',['new_project_is_deterministic_create_only_and_removable']),
 'unsafe-path':rust('project-unsafe',['new_project_rejects_unsafe_names_paths_and_stages_without_cleanup_damage']),
 collision:rust('project-collision',['new_project_is_deterministic_create_only_and_removable']),
 'retained-root':rust('project-retained',['project::tests::retained_root_allows_project_output_creation_without_admitting_output_as_source','project::tests::retained_project_batches_use_only_original_declared_bytes_until_full_revalidation']),
 'frozen-source':rust('project-frozen',['project::tests::current_entrypoint_mutation_fails_before_project_execution_can_continue','project::tests::frozen_source_session_rejects_undeclared_state_file_before_reading_it']),
 javascript:field('project-js','accept/acceptance.json','compilerStarter.javascript === 42',v=>v.compilerStarter.javascript===42),
 webassembly:field('project-wasm','accept/acceptance.json','compilerStarter.webassembly === 42',v=>v.compilerStarter.webassembly===42),
 'forbidden-override':field('project-overrides','installed_supplement/captured/RESULT.json','both installed-forbidden-root and installed-forbidden-node actual exit2 observations',v=>['installed-forbidden-root','installed-forbidden-node'].every(id=>v.rows.some(r=>r.id===id&&r.status==='pass'&&r.exitCode===2)))
});
gate('lsp',{
 framing:rust('lsp-framing',['framing_is_exact_bounded_and_recovers_between_frames']),
 'open-change-close':rust('lsp-revisions',['open_change_and_close_publish_revision_bound_diagnostics']),
 'unicode-crlf':field('lsp-unicode','installed_supplement/captured/RESULT.json','actual installed UTF16 astral split error -32602 with CRLF source retained',v=>v.rows.some(r=>r.id==='installed-unicode-crlf-astral-split'&&r.status==='pass'&&r.errorCode===-32602)),
 'compiler-diagnostics':rust('lsp-diagnostics',['open_change_and_close_publish_revision_bound_diagnostics','stdio_m2_routes_control_flow_and_rejects_invalid_edits']),
 'cancel-stale':rust('lsp-cancel',['cancelled_and_stale_ids_remain_reserved_until_response_emission','queued_definition_is_cancelled_and_next_request_recovers','same_length_change_suppresses_stale_definition']),
 'unsupported-method':field('lsp-unsupported','installed_supplement/captured/RESULT.json','actual installed unsupported rename error -32601 and following recovery',v=>v.rows.some(r=>r.id==='installed-unsupported-method'&&r.status==='pass'&&r.errorCode===-32601)&&v.rows.some(r=>r.id==='installed-recovery-after-unsupported'&&r.status==='pass')),
 'scalar-definition':field('lsp-definition','installed_supplement/captured/RESULT.json','actual installed scalar definition recovered after invalid UTF16 position',v=>v.rows.some(r=>r.id==='installed-scalar-definition-recovery'&&r.status==='pass'&&r.definition.range.start.line===1))
});
const F='diagnostic_sessions::';gate('formatter',{
 'scalar-document':rust('format-scalar',[F+'tests::formatting::formatting_goldens_are_idempotent_and_preserve_verified_meaning']),
 'scalar-range':rust('format-scalar-range',[F+'tests::formatting::formatting_range_preserves_outside_bytes_and_rejects_partial_functions']),
 'm2-document':rust('format-m2',[F+'formatting::control_flow_tests::m2_formatting_is_canonical_and_preserves_verified_meaning']),
 'm2-range':rust('format-m2-range',[F+'formatting::control_flow_tests::m2_range_layout_preserves_bytes_outside_the_selected_function']),
 'm3-document':rust('format-m3',[F+'formatting::data_ownership_tests::m3_multifile_formatting_is_idempotent_and_preserves_verified_syntax']),
 'm3-range':rust('format-m3-range',[F+'formatting::data_ownership_tests::m3_range_rejects_partial_declarations_and_keeps_other_files']),
 idempotence:rust('format-idempotence',[F+'tests::formatting::formatting_goldens_are_idempotent_and_preserve_verified_meaning',F+'formatting::data_ownership_tests::m3_multifile_formatting_is_idempotent_and_preserves_verified_syntax']),
 'token-comment-preservation':rust('format-preservation',[F+'formatting::control_flow_tests::m2_operator_boundaries_survive_formatting',F+'formatting::data_ownership_tests::m3_admitted_ownership_constructs_keep_topology_after_layout']),
 'rejected-source':rust('format-reject',[F+'tests::formatting::formatting_rejects_malformed_unsupported_and_semantic_errors_without_edits']),
 'saved-import':rust('format-import',['m3_saved_import_and_open_entry_format_then_reject_changed_dependency']),
 'cancel-stale':rust('format-cancel',['formatting::formatting_cancel_change_close_and_recovery_are_revision_bound','formatting::formatting_rechecks_revision_at_actual_transport_emission']),
 limits:rust('format-limits',[F+'tests::formatting::formatting_result_limit_is_inclusive_and_foreign_sessions_reject',F+'formatting::control_flow_tests::m2_formatting_result_limit_is_exact_and_preserves_semantic_admission',F+'formatting::data_ownership_tests::m3_document_limit_accepts_exact_cap_and_rejects_plus_one'])
});
const hostCheck=(id,name)=>field(id,'editor/m6-installed-editor/host-result.json','native editor check: '+name,v=>v.passed&&v.checks.includes(name));
gate('extension',{
 'vsix-identity':tap('extension-vsix',['the exact current VSIX path is required even if an old package is inventoried','installed server revision/version mismatches receive no source and invalid setups never launch']),
 'pre-source-handshake':tap('extension-handshake',['revoked workspace trust prevents an unfinished handshake from receiving source','explicit M2 selection negotiates before source and preserves the scalar default']),
 'workspace-trust':hostCheck('extension-trust','explicit workspace trust'),
 'profile-selection':tap('extension-profile',['profile switch suppresses old diagnostics and restores scalar definition','rapid profile selections settle in command order without stale source admission']),
 diagnostics:hostCheck('extension-diagnostics','invalid-to-valid diagnostics and stale recovery'),
 definition:hostCheck('extension-definition','scalar profile and definition'),
 formatting:hostCheck('extension-formatting','M3 saved-import formatting and idempotence'),
 'scalar-run':hostCheck('extension-scalar-run','real scalar JavaScript result'),
 'm2-run':hostCheck('extension-m2-run','real M2 WebAssembly result'),
 'saved-file':tap('extension-saved',['explicit command uses only user executable and exact saved bytes','compiler scaffolds and authenticates saved bytes before run; output binding rejects tampering']),
 'root-substitution':tap('extension-root',['workspace executable overrides are ignored and incompatible servers receive no document contents','complete setup relocates and derives only fixed executable paths','changed, missing, extra and linked bytes are rejected']),
 cleanup:tap('extension-cleanup',['graceful shutdown sends exit and leaves no pending requests','deactivation cancels a live process and terminates it'])
});
gate('setup',{
 inventory:field('setup-inventory','setup/candidate-receipt.json','exact-source review-candidate manifest matches actual accepted manifest',v=>v.manifestSha256===accept.manifestSha256&&v.sourceTree===corpus.source.tree),
 relocation:field('setup-relocation','accept/acceptance.json','relocated === true',v=>v.relocated===true),
 tamper:field('setup-tamper','accept/acceptance.json','mismatch/substituted-worker/missing-worker rejected',v=>v.mismatchRejected&&v.substitutedWorkerRejected&&v.missingWorkerRejected),
 compatibility:tap('setup-compatibility',['mixed candidate, compiler, server and editor identities reject even with a matching digest','manifest replacement cannot redefine the externally supplied digest']),
 reproduction:field('setup-reproduction','setup/reproduction.json','actual two independent server and VSIX replica digests equal authenticated selected bytes',v=>JSON.stringify(v.serverReplicas)===JSON.stringify([setup.serverSha256,setup.serverSha256])&&JSON.stringify(v.vsixReplicas)===JSON.stringify([setup.vsixSha256,setup.vsixSha256])),
 'project-editor-exercise':field('setup-exercise','accept/acceptance.json','actual scalar/controlflow JavaScript+Wasm values; formatter/diagnostics/definition source preservation',v=>v.compilerStarter.javascript===42&&v.compilerStarter.webassembly===42&&v.editorRun.scalar.javascript===9&&v.editorRun.scalar.webassembly===26&&v.editorRun.controlFlow.javascript===10&&v.editorRun.controlFlow.webassembly===-6&&v.formatting.scalar&&v.formatting.controlFlow&&v.formatting.dataOwnership&&v.scalarDefinition)
});
gate('transport',{
 'closed-carriers':tap('transport-carriers',['supervised response-selected authority and extra carrier fields reject while preserving cleanup token']),
 'byte-boundaries':tap('transport-bytes',['stream first extra byte cancels transport and redirect cannot replace authority']),
 'revision-identity':tap('transport-revision',['evaluation accepts only the exact latest revision and hashes after compile cleanup']),
 'late-upload':tap('transport-late',['a cancelled supervised upload cannot execute after cleanup has been acknowledged']),
 'overlapping-finish':tap('transport-finish',['overlapping finish acknowledgements cannot release another pending cleanup']),
 'failed-teardown':tap('transport-failed',['teardown acknowledgement is exact and failure retains the token for another cleanup attempt','supervised cancellation retains the reservation through independent daemon teardown']),
 'unsupported-host':tap('transport-host',['unsupported verifier hosts and arbitrary hash root version ownership or symlink claims never select an executable'])
});
const docsManifest=json('docs/bundle/manifest.json'),docsAuth=json('docs/operator-authentication.json');assert.equal(docsManifest.source.commit,corpus.source.commit);assert.equal(sha(get('docs/bundle/manifest.json')),docsAuth.expected.manifestSha256);assert.equal(docsManifest.documents.length,60);
for(const doc of docsManifest.documents){const b=get('docs/bundle/'+doc.path);assert.equal(b.length,doc.bytes);assert.equal(sha(b),doc.sha256)}
gate('documentation',{
 'support-matrix':field('docs-matrix','docs/operator-authentication.json','actual original authenticated docs source/registry/matrix/workflow/manifest and 60 documents, with all six hosted lanes',v=>v.status==='official-docs-authenticated-and-vendored-bytes-identical'&&v.expected.source.commit===corpus.source.commit&&v.documents===60&&v.matchedFiles===62),
 'export-identity':remember('docs-export',[{kind:'complete-authenticated-docs-bundle-byte-verification',manifestPath:'docs/bundle/manifest.json',manifestSha256:sha(get('docs/bundle/manifest.json')),verifiedDocuments:60}]),
 'runnable-project-editor-exercises':remember('docs-exercises',[...proof['setup-exercise'],...proof['extension-m2-run'],...proof['extension-definition']])
});
const jobs=json('full-ci/jobs.json');assert.equal(jobs.total_count,42);assert.equal(jobs.jobs.length,42);assert(jobs.jobs.every(j=>j.head_sha===corpus.source.commit&&j.conclusion==='success'&&j.status==='completed'));
const inventory=json('full-ci/inventory.json');assert.deepEqual(jobs.jobs.map(j=>j.name).sort(),inventory.jobs.map(j=>j.name).sort());
const step=(id,name)=>remember(id,[{kind:'actual-exact-source-completed-job-step',jobId:job.id,step:(()=>{const matches=job.steps.filter(s=>s.name===name);assert.equal(matches.length,1);assert.equal(matches[0].conclusion,'success');assert.equal(matches[0].status,'completed');return matches[0]})()}]);
const marker=(id,name)=>remember(id,[{kind:'original-complete-shared-log-m0-marker',path:'canonical/ACTUAL_JOB.log',marker:name,lines:(()=>{const a=[];lines.forEach((l,i)=>{if(l.match(/\[M0 [0-9]+\/18\]/)&&l.endsWith(' '+name))a.push(i+1)});assert.equal(a.length,1);return a})(),owningSuccessfulJobId:job.id}]);
const platformJob=(id,prefix)=>remember(id,[{kind:'actual-successful-exact-source-full-ci-job',job:(()=>{const needle=prefix+' ('+(platform==='linux'?'ubuntu-latest':'windows-latest')+')';const a=jobs.jobs.filter(j=>j.name===needle);assert.equal(a.length,1);assert.equal(a[0].conclusion,'success');return {id:a[0].id,name:a[0].name,source:a[0].head_sha}})()}]);
gate('canonical',{
 architecture:marker('canonical-architecture','architecture-check'),format:marker('canonical-format','rust-format'),clippy:marker('canonical-clippy','rust-clippy'),
 'workspace-tests':marker('canonical-tests','rust-tests'),rustdoc:marker('canonical-rustdoc','rustdoc'),adapter:platformJob('canonical-adapter','adapter'),
 protocol:marker('canonical-protocol','protocol-tests'),m0:step('canonical-m0','Run node scripts/run-m0-conformance.mjs'),m2:platformJob('canonical-m2','m2'),m3:platformJob('canonical-m3','m3'),tooling:step('canonical-tooling','Verify and package editor client')
});
gate('hosted',{
 'required-jobs':remember('hosted-required',[{kind:'exact-complete-required-job-name-set',runId:39105995385,attempt:1,requiredCount:42,verifiedCount:jobs.jobs.length}]),
 'exact-source':remember('hosted-source',[{kind:'all-original-full-ci-job-heads-match',source:corpus.source,runId:39105995385,verifiedJobs:42}]),
 'no-required-skips':remember('hosted-skips',[{kind:'all-required-jobs-status-completed-conclusion-success',runId:39105995385,verifiedJobs:42,meaning:'No skipped required job; internal ignored Rust tests remain ignored and are not relabelled as passing tests.'}]),
 'retained-evidence':remember('hosted-retained',[{kind:'complete-reader-input-inventory',verifiedFiles:files.size,verifiedBytes:total,originalArtifactPins:corpus.artifacts.map(a=>({id:a.id,bytes:a.size_in_bytes,sha256:a.digest.slice(7)})),meaning:'Fresh parent API identity verification recorded separately. Final durable release retention is still required.'}])
});
const result={format:'private.m6-actual-native-readonly-proof-reader.v1',status:'read-only-observations-verified',source:corpus.source,environment:{platform,osVersion:release(),architecture:'x86_64',runtime:process.version},startedAt,finishedAt:new Date().toISOString(),corpusSha256:expectedSha,corpusBytes:corpusBytes.length,verifiedFiles:files.size,verifiedBytes:total,gates:cases,proofs:proof,scope:'Own native reader command checks previously executed actual source-bound evidence. Cases count verified supporting observations, not unique historical tests; observations overlap across cases. No tests rerun, no compiler/publication/runtime admission. Historical raw streams/logs remain original artifact materials; these new reader streams describe this command only.'};
writeFileSync(output,JSON.stringify(result,null,2)+'\n',{flag:'wx',mode:0o400});
console.log(JSON.stringify({...result,proofs:undefined}));
