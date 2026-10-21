// Exact source inputs -> actual bounded commands -> retained original captures.
// This scoped runner cannot issue a complete #414 acceptance from omitted lanes.
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync,realpathSync,existsSync,readFileSync} from 'node:fs';
import {resolve,isAbsolute,relative,sep,join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {loadRegistry,repositoryRoot,registrySha256,canonical,sha256,acceptanceStatus} from '../tests/native-provider-public/registry.mjs';
import {runWorkerParity} from '../tests/native-provider-public/worker-parity.mjs';
import {runCliParity} from '../tests/native-provider-public/cli-parity.mjs';

assert.equal(process.version,'v22.22.1','pinned Node required');
const args=process.argv.slice(2),options={};assert.equal(args.length,8,'--cli --node --output --lane are required');
for(let i=0;i<args.length;i+=2){assert(['--cli','--node','--output','--lane'].includes(args[i])&&!(args[i]in options));options[args[i]]=args[i+1];}
for(const key of ['--cli','--node','--output'])assert(isAbsolute(options[key])&&resolve(options[key])===options[key]);
assert(['worker','cli','all'].includes(options['--lane']));
for(const key of ['--cli','--node'])assert(realpathSync(options[key])===options[key]);
const output=options['--output'],outside=relative(repositoryRoot,output);assert(!existsSync(output)&&(isAbsolute(outside)||outside==='..'||outside.startsWith('..'+sep)),'new external evidence directory');
const git=(...argv)=>{const p=spawnSync('git',['-C',repositoryRoot,...argv],{encoding:'utf8',timeout:30000});assert.equal(p.status,0);return p.stdout.trim();};
const source={commit:git('rev-parse','HEAD'),tree:git('rev-parse','HEAD^{tree}')};assert.equal(git('status','--porcelain'),'','committed clean exact candidate');
const registry=loadRegistry(),host=process.platform+'-'+process.arch;assert(registry.supportedHosts.includes(host),'actual supported host required');mkdirSync(output,{mode:0o700});const startedAt=new Date().toISOString(),lanes=Object.fromEntries(registry.requiredAcceptanceLanes.map(id=>[id,{status:'not-run',reason:'Requires its own actual current-candidate execution; no historical pass substituted.'}]));
const pin=path=>{const b=readFileSync(path);return {path,bytes:b.length,sha256:sha256(b)};};
writeFileSync(join(output,'INTENT.json'),canonical({source,host,registrySha256,cli:pin(options['--cli']),node:pin(options['--node']),lane:options['--lane'],startedAt,scope:'Selected actual source comparisons only; complete414supportrequiresallremaininglanes.'}),{flag:'wx',mode:0o600});
let failed=false;
try{
 const params={registry,root:repositoryRoot,cli:options['--cli'],node:options['--node']};
 if(['worker','all'].includes(options['--lane']))lanes['worker-snapshot-parity']=await runWorkerParity({...params,output:join(output,'worker')});
 if(['cli','all'].includes(options['--lane']))lanes['cli-build-parity']=await runCliParity({...params,output:join(output,'cli')});
 failed=Object.values(lanes).some(r=>r.status==='failed');loadRegistry();assert.deepEqual({commit:git('rev-parse','HEAD'),tree:git('rev-parse','HEAD^{tree}')},source);assert.equal(git('status','--porcelain'),'','candidate stayed clean');
}catch(error){failed=true;writeFileSync(join(output,'FAILURE.json'),canonical({message:String(error.message),scope:'Actual failure preserved, no complete acceptance.'}),{flag:'wx',mode:0o600});}
const acceptance=acceptanceStatus(registry,lanes);writeFileSync(join(output,'RESULT.json'),canonical({format:'zryna.native-provider-public-scope-result.v1',source,host,registrySha256,startedAt,finishedAt:new Date().toISOString(),status:failed?'failed':'selected-lanes-executed',acceptance,publicActivation:false,lanes}),{flag:'wx',mode:0o600});
console.log(JSON.stringify({output,status:failed?'failed':'selected-lanes-executed',acceptance,publicActivation:false}));process.exitCode=failed?1:0;
