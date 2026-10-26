# MSAL6 upstream-backport regression packet

This is a persistent compatibility/security regression, not a release certificate, advisory waiver, source issuer or generic ignore mechanism. The known GHSA-xcx6-jx8p-x3h5 version-range finding remains visible for6.0.1. This packet independently exercises the source-bound backport of upstream commitf550958586475dbfeb1cd44be14a2d653728850e while retaining the MSAL6 API. It does not approve a blind MSAL7 override or claim live marketplace publishing.

`tests/msal-backport.test.mjs` is imported by the existing registered preflight.test.mjs. Its ONE parent packet creates ONE fresh SDK child and exposes all ELEVEN original controls as Node named subtests:12Node tests total, not11independent process runs. The child runs exactly11same CJS/ESM controls from the frozen scratch study. Assert bodies, synthetic request/code/nonce/state/attacker metadata, parent-delegation spies, ordinary-positive and genuine base-state negative remain byte-identical. Only dependency resolution/import paths change. The SDK-only preload keeps all original HTTP/fetch/socket/TLS/DNS/datagram/WebSocket denials and additionally denies every subprocess unconditionally; the scratch-only unused packaging exception is absent.

Owner command under the repository-pinned Node22.22.1:

```
node --test --test-reporter=tap tests/msal-backport.test.mjs
```

No SDK/install/packager execution was performed while authoring this repo adaptation. Root must execute it after adopting canonicalLFpatch+rootlock. Prior scratch unpatched SDK gave5PASS6owningFAIL; patched SDK11PASS and the actual unchanged ls/package script produced byte-identical canonical VSIX30092B. Those results are retained historical observations, not a pass for this new registered source or a new root candidate.

## Resolution, material and EOL boundary

No direct MSAL dependency is introduced. materials.mjs resolves package JSON through pinned @vscode/vsce4.0.0→@azure/identity4.13.3→@azure/msal-node6.0.1, then loads only that actual transitive MSAL in the child. Resolving package JSON does NOT execute vsce/identity or their credential constructors. The parent reads only source/package/lock/provenance data. The fixed provenance JSON hash binds upstream commit, public original archive/SRI, origin raw patch e152924d… and canonical repository patch ebed7ec…/pnpm hash, and all seven applied-source afterimages.

The operational patch path is scripts/dependency-patches/msal-node-6.0.1-auth-code.patch, preserving normal `* text=auto eol=lf`. The old scratch patches/@azure__msal-node@6.0.1.patch path is origin-only. The test requires canonicalLFrawpatch SHA and the same normalized rootlock hash, plus matching workspace patchedDependencies path. It never marks the patch binary or changes .gitattributes.

Root's initial exactrawcomparison correctly rejected four generated artifacts after canonicalLFpatch application. The retained root inspection then proved exact equality after ONLY CRLF→LF; TS2 and NodeAuthError.d.ts remained raw-identical. Provenance retains BOTH original study and reviewed root raw sizes/SHA plus exact canonicalLF content hashes. The test accepts only one of those two explicitly pinned raw forms for each file and ALSO checks canonicalLF hash; arbitrary third variants, any code drift, stripped bareCR or self-supplied new baselines reject. BareCR is never normalized. Any allowed root-vs-study variance is printed as a diagnostic. This recognizes explicit EOL policy, not a behavioral rebaseline or silent pin refresh.

## Real environment and credential isolation

The SDK does not execute in preflight's environment. The parent uses spawnSync(process.execPath) without spreading/reading process.env: only freshHOME/USERPROFILE/APPDATA/LOCALAPPDATA/temp locations and CI=1 enter the child. No PATH, real Azure/provider/proxy/token/config environment is inherited. No external authentication executable or UI can start. The parent kills a timed-out direct child through spawnSync; the SDK guard forbids grandchildren. The parent-only temp directory is removed after the child is reaped; cleanup-hook failures remain test failures.

MSAL's ClientApplication constructor builds configuration, crypto, logger, in-memory NodeStorage and TokenCache from the supplied synthetic clientId/tenant/secret; no cache plugin is configured. The inherited genuine state-mismatch path rejects before network. CJS module evaluation includes AzureArc constants that consult ProgramData/ProgramFiles; they see only this empty/synthetic child environment. Client-credential region/managed-identity methods can consult MSAL_FORCE_REGION/REGION_NAME/IDENTITY/MSI variables, but those methods are not exercised and no such real values are passed. AzureIdentity EnvironmentCredential, CLI, managed-identity and browser constructors are NOT invoked by this packet: only their package JSON is resolved. No real environment/token files are discovered or read.

These controls verify constructor/query guards, payload sanitization before base delegation, ordinary delegation and state validation. They do not perform real token exchange, credentialed publishing, live interactive/device-code/broker/managed identity auth, sandbox hostile native code, or qualify Windows until the actual Windows owner run succeeds. The earlier optional synthetic credential-chain probe remains unresolved and is not included/rebranded as passing here.
