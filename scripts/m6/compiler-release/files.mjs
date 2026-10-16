import { lstatSync, mkdirSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, ordinary, orderedPaths, requireValue, writeNew } from './common.mjs';
export function directoryFiles(root,{maximumFiles=500,maximumFile=64*1024*1024,maximumTotal=96*1024*1024,exclude=[]}={}) {
  requireValue(lstatSync(root).isDirectory() && !lstatSync(root).isSymbolicLink(), 'compiler release ordinary directory');
  const result=[];let total=0;
  function visit(path,prefix,depth){
    requireValue(depth<=12,'compiler release directory depth');
    for(const name of readdirSync(path).sort()){
      const relative=prefix?`${prefix}/${name}`:name;
      if(exclude.includes(relative))continue;
      const full=join(path,name),stat=lstatSync(full);
      requireValue(!stat.isSymbolicLink(),'compiler release no source symlink');
      if(stat.isDirectory()) visit(full,relative,depth+1);
      else {
        requireValue(result.length<maximumFiles,'compiler release directory file bound');
        const data=ordinary(full,maximumFile);total+=data.length;
        requireValue(total<=maximumTotal,'compiler release directory total');
        result.push({path:relative,mode:0o644,data});
      }
    }
  }
  visit(root,'',0);result.sort((a,b)=>a.path<b.path?-1:1);orderedPaths(result.map(f=>f.path));return result;
}
export function storeFiles(root,files){
  mkdirSync(root,{recursive:false,mode:0o700});orderedPaths(files.map(f=>f.path));
  for(const file of files){
    const path=join(root,file.path);requireValue(path.startsWith(`${resolve(root)}/`),'compiler release output confinement');
    mkdirSync(resolve(path,'..'),{recursive:true,mode:0o700});writeNew(path,file.data);
  }
}
export const inventory=files=>files.map(f=>describe(f.path,f.data));
