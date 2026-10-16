# Installed-editor acceptance helper

This is trusted test tooling, separate from restricted playground execution. It grants no source,
publication or native host capability. The original development-host launcher is unchanged.

The caller must authenticate the complete portable handoff and independently select its source,
archive, setup-manifest, server and VSIX pins. Same-run CI uses only the authenticated artifact
from the successful portable build in that workflow; an adjacent self-hash alone is insufficient.
The helper checks exact source and all those pins, canonical complete setup archive/installation,
and all twelve VSIX payload bodies against the reviewed checkout before installation.

Code 1.138.0 archives have fixed independently selected official HTTPS identities in pins.mjs.
Acquisition verifies exact bytes/SHA before extraction. PREPARED.json records actual executable,
product metadata and complete extracted inventory. Use --prepare-only for first independent
material review before any editor installation or GUI launch. That mode is never a GUI pass.

Actual Linux runs use the reusable workflow's fixed Linux service wrapper. After authenticated
preparation, a grant within the original preparation deadline attaches one AppArmor user-namespace
profile to that exact Code executable. The standard service runs the controller and editor as the
original nonroot runner. Its separate cleanup and profile removal cannot replace the original
editor owner's cleanup or the nine host checks. The normal editor sandbox stays enabled.

The run installs only the verified VSIX and loads only this dedicated runner as a development
extension. Raw installed payload differences, including unknown installer metadata changes,
fail closed. There is no speculative normalization rule. Only actual qualified differences may
receive a separately reviewed correction.

Workspace trust and the normal editor sandbox remain enabled. The runner first proves its fresh
folder untrusted and the installed extension inactive, then the parent clicks the normal exact
folder trust dialog through pinned Playwright/CDP on the owned loopback endpoint. Parent-folder
trust stays unselected. No --no-sandbox, --disable-workspace-trust or fabricated trust database is
used. The CDP channel belongs only to this trusted GUI harness, never to the product or user code.

Nine real host checks and three generated output identities are required. Linux Xvfb is a real
native Linux editor under a virtual display; it does not prove a physical desktop or Windows.
Windows has its own native GUI job. Unit fixtures are only filesystem/parser/lifecycle controls.
No zero-test, ignored or unavailable-host result counts as GUI acceptance.

RESULT.json, complete command logs, actual host result/inputs, installed/editor inventories and
copied generated artifacts remain outside the checkout. Primary/cleanup failures stay distinct.
The private stage is deliberately retained for independent inspection/owned cleanup; this helper
makes no general hostile process-tree isolation or whole M6 closure claim. GUI observations do
not replace playground containment, accessibility, LSP/client negatives or final hosted evidence.
The workflow's 90-day upload is temporary retention; final closure needs independent durable
publication of exact receipts and source documentation.

The full CI dispatch connects the reusable installed-editor workflow to the current
portable-setup workflow through an explicit dependency and same-run artifact handoff.
Actual Code installation metadata, debug-port output, trust UI and both-host process behavior
remain qualification prerequisites. Workflow wiring alone supplies no passing GUI observation.

The corrected process launcher uses a creation-owning trusted supervisor. Linux requires
Python's pidfd APIs plus a private child-subreaper; it binds a pidfd before releasing the
root startup barrier. Windows requires PowerShell 7 and retained .NET Process SafeHandles.
It records actual creation times and images and uses only those retained process objects.
The supervisor, runtime, requested executable and loaded runtime dependencies are hashed
before the explicit target-start operation. Missing identity, inaccessible ownership,
unknown cleanup or timeout prevents acceptance. Closing the Node pipe references only
bounds the driver; it never turns unknown process cleanup into success.

Windows runtime-module observations distinguish ordinary system DLL hardlinks from supplied
materials. Loaded dependencies at canonical direct `SystemRoot\System32\*.dll` paths may have
multiple stable links. Four named DLLs under System32 also qualify:
`WindowsPowerShell\v1.0\pwrshsip.dll`, `wbem\fastprox.dll`, `wbem\wbemprox.dll` and
`wbem\wbemsvc.dll`. Retained handles, complete hashes and before/after identities include the
actual link count; receipts distinguish direct and named policies and make no
publisher-authentication claim.
Requested targets, supervisor executables, helper sources, archives and supplied materials retain
their single-link requirement.

Archive/setup inputs are consumed from retained no-follow, identity-checked descriptors
and copied while hashing into fresh private protected files. Extraction reads only these
captured files. The trust negative actually opens a `.zry` document while untrusted and
observes inactive extension/absent diagnostics before the normal exact-folder GUI action.

All corrected helper behavior, PowerShell/.NET handle semantics, loaded dependency
availability and both real GUI lanes require independent review and actual runs for the selected source.
The ownership tests use their own trusted Node children and are not GUI or containment
certificates. No unavailable process/stream fact is synthesized as a passing observation.

Required ownership controls include an immediate natural root exit on both platforms.
Linux additionally registers a real immediate-parent-exit/detached-child subreaper case;
that Linux case supplies no Windows pass or skip credit. The owning command requires every
registered native control, with nonzero failures propagated by the workflow.
A Windows created root that positively exited before its image query records actual
image unavailable, its retained-handle start/exit times and the separately selected
requested executable. Running roots and discovered processes have no such fallback;
an observed image mismatch remains a failure even after exit.
