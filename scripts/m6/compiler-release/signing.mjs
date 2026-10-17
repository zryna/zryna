import { chmodSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { captureCompilerPublicationVerifier, verifyCompilerPublicationSignature }
  from '../../../examples/playground/restricted/daemon/compiler-publication-verifier.mjs';
import { verifierPins } from '../../../examples/playground/restricted/daemon/compiler-publication-constants.mjs';
import { bytes, command, describe, ordinary, requireValue, sha256, writeNew } from './common.mjs';
import { readResponse } from './github.mjs';
async function official(url,maximum,fetchImpl) {
  let current=url;const signal=AbortSignal.timeout(120000);
  for(let redirects=0;redirects<=4;redirects++) {
    const response=await fetchImpl(current,{headers:{'Accept-Encoding':'identity'},redirect:'manual',signal});
    if([301,302,303,307,308].includes(response.status)){
      requireValue(redirects<4,'compiler signer redirect bound');
      const next=new URL(response.headers.get('location'),current);
      requireValue(next.protocol==='https:' && !next.username && !next.password && !next.port &&
        ['github.com','release-assets.githubusercontent.com','objects.githubusercontent.com'].includes(next.hostname),
      'compiler signer official redirect');current=next.href;continue;
    }
    requireValue(response.status===200,'compiler signer official acquisition');return readResponse(response,maximum);
  }
}
export async function acquireSigner({output,fetchImpl=fetch}={}) {
  mkdirSync(output,{recursive:false,mode:0o700});
  const executable=await official('https://github.com/sigstore/cosign/releases/download/v3.0.5/cosign-linux-amd64',
    verifierPins.bytes,fetchImpl);
  const trustedRoot=await official('https://raw.githubusercontent.com/sigstore/root-signing/ddf87cc7f93c6790040e1748be1d2a4a8c82263f/targets/trusted_root.json',
    verifierPins.trustedRootBytes,fetchImpl);
  requireValue(executable.length===verifierPins.bytes && sha256(executable)===verifierPins.sha256 &&
    trustedRoot.length===verifierPins.trustedRootBytes && sha256(trustedRoot)===verifierPins.trustedRootSha256,
  'compiler signer independently approved official material pins');
  const path=join(output,'cosign-material');writeNew(path,executable);chmodSync(path,0o400);
  const policy={...verifierPins,path,ownerUid:process.getuid()};
  const captured=captureCompilerPublicationVerifier(policy);
  const runnable=join(output,'cosign');writeNew(runnable,captured);chmodSync(runnable,0o500);
  writeNew(join(output,'trusted-root.json'),trustedRoot);
  return {policy,trustedRoot,runnable};
}
export function signPublicationBlob({signer,path,bundlePath,sourceCommit,environment=process.env}) {
  requireValue(environment.GITHUB_ACTIONS==='true' && typeof environment.ACTIONS_ID_TOKEN_REQUEST_TOKEN==='string' &&
    environment.ACTIONS_ID_TOKEN_REQUEST_TOKEN.length>0 && typeof environment.ACTIONS_ID_TOKEN_REQUEST_URL==='string',
  'compiler actual GitHub OIDC signing environment');
  const endpoint=new URL(environment.ACTIONS_ID_TOKEN_REQUEST_URL);
  requireValue(endpoint.protocol==='https:' && endpoint.hostname.endsWith('.actions.githubusercontent.com') &&
    !endpoint.username && !endpoint.password,'compiler GitHub OIDC endpoint');
  const env={LANG:'C.UTF-8',HOME:join(signer.runnable,'..'),GITHUB_ACTIONS:'true',
    ACTIONS_ID_TOKEN_REQUEST_TOKEN:environment.ACTIONS_ID_TOKEN_REQUEST_TOKEN,
    ACTIONS_ID_TOKEN_REQUEST_URL:environment.ACTIONS_ID_TOKEN_REQUEST_URL};
  for(const name of ['HTTPS_PROXY','HTTP_PROXY','NO_PROXY','SSL_CERT_FILE','SSL_CERT_DIR']) {
    if(environment[name])env[name]=environment[name];
  }
  let actual;
  try {
    actual=command(signer.runnable,['sign-blob','--yes','--tlog-upload=true',
      '--fulcio-url','https://fulcio.sigstore.dev','--rekor-url','https://rekor.sigstore.dev',
      '--bundle',bundlePath,path],{cwd:join(path,'..'),env,timeout:120000,maximum:65536});
  } catch(error) {
    error.signingProof=error.proof;
    if(error.proof)retainFailure(bundlePath,error.proof);throw error;
  }
  let verification;
  try { verification=verifyCompilerPublicationSignature(ordinary(path,262144),ordinary(bundlePath,1048576),
    signer.policy,signer.trustedRoot,sourceCommit); }
  catch(error){
    error.signingProof=actual;retainFailure(bundlePath,actual);
    if(error.verification){
      writeNew(`${bundlePath}.failed-verification.json`,bytes({command:error.verification.command,result:error.verification.result}));
      writeNew(`${bundlePath}.failed-verification.stdout`,error.verification.stdout);
      writeNew(`${bundlePath}.failed-verification.stderr`,error.verification.stderr);
    }
    throw error;
  }
  return {signing:actual,verification,materials:{verifier:describe('cosign',captureCompilerPublicationVerifier(signer.policy)),
    trustedRoot:describe('trusted-root.json',signer.trustedRoot)}};
}

function retainFailure(path,proof) {
  writeNew(`${path}.failed-signing.json`,bytes(proof.record));
  writeNew(`${path}.failed-signing.stdout`,proof.stdout);writeNew(`${path}.failed-signing.stderr`,proof.stderr);
}
