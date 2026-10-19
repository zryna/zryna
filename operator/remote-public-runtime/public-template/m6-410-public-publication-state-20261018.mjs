// Trusted operator process only. The original module keeps the opaque capability alive.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {authenticateCompilerPublication,compilerPublicationReceipt,compilerPublicationVerification} from '/workspace/zryna/examples/playground/restricted/daemon/compiler-publication-capability.mjs';
import {createPublishedSourceLauncher} from '/workspace/zryna/examples/playground/restricted/daemon/launcher.mjs';
let retained;
export async function initializeActualPublication(inputs){
 assert.equal(retained,undefined);retained=await authenticateCompilerPublication(inputs);
 return {receipt:compilerPublicationReceipt(retained),verification:compilerPublicationVerification(retained)};
}
export function actualPublicationCapability(){assert(retained);return retained}
export async function createSourceLauncher(reviewed){assert(retained);return createPublishedSourceLauncher(reviewed,retained)}
