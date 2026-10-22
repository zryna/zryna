import { resolve } from 'node:path';
import { prepareAdmission } from './admission.mjs';
import { buildCompiler } from './build.mjs';
import { assembleCompiler } from './assemble.mjs';
import { publishCompiler } from './publish.mjs';
import { requireValue } from './common.mjs';
const [mode,...args]=process.argv.slice(2);
const values={};
for(let index=0;index<args.length;index+=2){
  requireValue(/^--[a-z-]+$/.test(args[index]) && args[index+1] && !Object.hasOwn(values,args[index]),
    'compiler release CLI closed arguments');values[args[index]]=args[index+1];
}
const options=allowed=>{
  requireValue(Object.keys(values).length===allowed.length && allowed.every(key=>Object.hasOwn(values,key)),
    'compiler release CLI required arguments');
  return Object.fromEntries(allowed.map(key=>[key.slice(2).replace(/-([a-z])/g,(_,c)=>c.toUpperCase()),
    key==='--replica'?Number(values[key]):resolve(values[key])]));
};
try {
  if(mode==='admit')await prepareAdmission({...options(['--source-root','--output']),deferQualification:true});
  else if(mode==='build')await buildCompiler(options(['--source-root','--cargo-home','--output','--admission-root','--replica']));
  else if(mode==='assemble')await assembleCompiler({...options(['--source-root','--admission-root','--first-root','--second-root','--output']),deferQualification:true});
  else if(mode==='publish')await publishCompiler(options(['--source-root','--admission-root','--first-root','--second-root','--assembled-root','--output']));
  else requireValue(false,'compiler release CLI mode');
} catch(error){
  console.error(error.message);process.exitCode=1;
}
