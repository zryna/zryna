// Retain actual public observations before Rust compares the frozen source oracle.
import {pathToFileURL} from 'node:url';
Object.defineProperty(globalThis,Symbol.for('zryna.generic.ownership.runtime.v1'),{
 get(){throw new Error('ordinary module attempted ambient runtime injection');}
});
const module=await import(pathToFileURL($artifactPath).href),observations=[];
for(const row of $record.public_rows){for(let i=0;i<row.repeats;i++){
 let value=null,error=null;try{value=module.entry(...row.arguments);}catch(failure){error=failure.message;}
 observations.push({value,error});
}}
console.log(JSON.stringify({exports:Object.keys(module),observations,ordinaryPublicArtifact:true}));
