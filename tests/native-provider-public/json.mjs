import assert from 'node:assert/strict';

// Preserve duplicate-key rejection before comparing decoded candidate fields.
// JSON.parse alone would discard an adversarial first value.
export function strictJson(bytes,maximum){
 assert(bytes instanceof Uint8Array&&bytes.length<=maximum,'bounded JSON bytes');
 const text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes),value=JSON.parse(text),stack=[];
 let strings=0;
 for(let i=0;i<text.length;i++){
  const c=text[i];
  if(c==='"'){
   const start=i++;while(i<text.length){if(text[i]==='\\'){i+=2;continue;}if(text[i]==='"')break;i++;}
   const token=JSON.parse(text.slice(start,i+1));assert(++strings<=1048576,'JSON work bound');
   const parent=stack.at(-1);if(parent?.kind==='object'&&parent.key){assert(!parent.keys.has(token),'duplicate JSON key');parent.keys.add(token);parent.key=false;}
  }else if(c==='{'){stack.push({kind:'object',key:true,keys:new Set()});assert(stack.length<=512,'JSON depth bound');}
  else if(c==='['){stack.push({kind:'array'});assert(stack.length<=512,'JSON depth bound');}
  else if(c==='}'||c===']')stack.pop();
  else if(c===','&&stack.at(-1)?.kind==='object')stack.at(-1).key=true;
 }
 assert.equal(stack.length,0);return value;
}
