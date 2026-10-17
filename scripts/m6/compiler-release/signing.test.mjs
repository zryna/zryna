import test from 'node:test';import assert from 'node:assert/strict';
import { mkdtempSync,readFileSync,readdirSync,rmSync,writeFileSync } from 'node:fs';import { join } from 'node:path';import { tmpdir } from 'node:os';
import { signPublicationBlob } from './signing.mjs';
test('actual failed command retains raw signing proof without pretending signature success',()=>{
  const root=mkdtempSync(join(tmpdir(),'m6-signing-failure-fixture-'));
  try{
    const path=join(root,'fixture.json'),bundlePath=join(root,'fixture.sigstore.json');writeFileSync(path,'{}\n');
    // The already installed trusted Node rejects the fixed Cosign command in this empty fixture cwd.
    // No injected verifier can mint a capability or return a fake successful signature.
    let actual;
    assert.throws(()=>signPublicationBlob({signer:{runnable:process.execPath},path,bundlePath,
      sourceCommit:'a'.repeat(40),environment:{GITHUB_ACTIONS:'true',ACTIONS_ID_TOKEN_REQUEST_TOKEN:'fixture-only',
        ACTIONS_ID_TOKEN_REQUEST_URL:'https://fixture.actions.githubusercontent.com/idtoken'}}),error=>{
      actual=error.signingProof;return Boolean(actual && actual.record.status!==0);
    });
    assert(actual.stderr.length>0);assert(readFileSync(`${bundlePath}.failed-signing.stderr`).equals(actual.stderr));
    assert.equal(JSON.parse(readFileSync(`${bundlePath}.failed-signing.json`)).status,actual.record.status);
    assert.equal(readdirSync(root).some(name=>name==='fixture.sigstore.json'),false);
  }finally{rmSync(root,{recursive:true,force:true});}
});
test('signing rejects absent OIDC authority and foreign endpoint before starting command',()=>{
  for(const environment of [{},{GITHUB_ACTIONS:'true',ACTIONS_ID_TOKEN_REQUEST_TOKEN:'fixture-only',
    ACTIONS_ID_TOKEN_REQUEST_URL:'https://example.invalid/idtoken'}]){
    assert.throws(()=>signPublicationBlob({signer:{runnable:'/nonexistent-fixture'},path:'/nonexistent-fixture',
      bundlePath:'/nonexistent-fixture',sourceCommit:'a'.repeat(40),environment}));
  }
});
