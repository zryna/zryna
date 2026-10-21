# Fixed public M4 support contract

This page and `zryna support --json` project one checked registry. The registry SHA-256 is
`0175a09f4ec1efef49439877df40e9d77e8062c8e4cd53442416df6a2dbbd6d3`. The matrix defines six bounded modes on the two
exact hosted contracts below. It is a static contract, not authenticated execution or publication.
Proof mappings and source limits are requirements; their presence does not show that tests ran.
The complete canonical JSON below is the authority for every row, identity, limit and exclusion.
Its independently pinned source-contract plan distinguishes emission, loader and runtime
boundaries, owning guard proofs, public bindings and reviewed public-input exclusions.

| Row | Profile / mode | Host contract | Selector | Source proof gaps |
| --- | --- | --- | --- | --- |
| `component-linux` | `default-m1` / `checkout-build` | `ubuntu-latest:observed-baseline-required` x64 | `build` / `component` | 0 |
| `component-windows` | `default-m1` / `checkout-build` | `windows-latest:observed-baseline-required` x64 | `build` / `component` | 0 |
| `browser-linux` | `browser-component-v1` / `checkout-browser` | `ubuntu-22.04` x64 | `build` / `component` | 0 |
| `browser-windows` | `browser-component-v1` / `checkout-browser` | `windows-2022` x64 | `build` / `component` | 0 |
| `command-linux` | `command-h1-v1` / `checkout-run` | `ubuntu-latest:observed-baseline-required` x64 | `run` / `wasi-command` | 0 |
| `command-windows` | `command-h1-v1` / `checkout-run` | `windows-latest:observed-baseline-required` x64 | `run` / `wasi-command` | 0 |
| `installed-command-linux` | `command-h1-v1` / `fresh-content-verified-installation` | `ubuntu-24.04` x64 | `run` / `wasi-command` | 0 |
| `installed-command-windows` | `command-h1-v1` / `fresh-content-verified-installation` | `windows-2022` x64 | `run` / `wasi-command` | 0 |
| `server-status-linux` | `server-status-v1` / `checkout-serve` | `ubuntu-latest:observed-baseline-required` x64 | `serve` / `none` | 0 |
| `server-status-windows` | `server-status-v1` / `checkout-serve` | `windows-latest:observed-baseline-required` x64 | `serve` / `none` | 0 |
| `server-clock-linux` | `server-clock-status-v1` / `checkout-serve` | `ubuntu-latest:observed-baseline-required` x64 | `serve` / `none` | 0 |
| `server-clock-windows` | `server-clock-status-v1` / `checkout-serve` | `windows-latest:observed-baseline-required` x64 | `serve` / `none` | 0 |

Each row states its precise Linux x86-64 or Windows x64 runner baseline. An unresolved
runner alias requires the actual observed baseline in authenticated evidence; it cannot certify
a concrete OS version. The Windows row makes no general desktop Windows claim. Browser execution is limited to
the pinned Chrome fixture; its Node companion is separate evidence. Default components are
build-only. Browser synchronous calls use post-hoc deadlines and cannot be preempted.
Status/clock server modes are finite loopback source-checkout arrangements, with distinct
listener approval and one-read guest clock approval. No source clock intrinsic is added.
Command modes share bounded H1 semantics; only a newly complete content-verified installation
qualifies for the installed mode. Content verification does not authenticate a signed release.

Installed component/browser/server, existing immutable beta H1, other browsers/platforms,
general WASI host authority, signed public release authentication and production admission
remain explicit exclusions. Specified-only typed-JS/library/resource/package interfaces are
prerequisites and do not become executable support. M0–M3 contracts remain unchanged.

## Runnable examples

- [Browser scalar component](BROWSER_COMPONENT_V1.md): checkout build and checked-in page.
- [Command walkthrough](WASI_COMMAND_GETTING_STARTED.md): pure/error and one literal private key;
  [execution manifest](WASI_COMMAND_MANIFEST_V1.md) retains actual results and teardown.
- [Finite server status](WASI_SERVER_STATUS_V1.md) and
  [separately approved clock/status](WASI_SERVER_CLOCK_STATUS_V1.md): checkout-only loopback.
- [Command sources](../examples/wasi-command/README.md),
  [status example](../examples/wasi-server/README.md), and
  [denied/granted clock example](../examples/wasi-server/CLOCK_EXAMPLE.md).

## Separate evidence authority

[M4 conformance](M4_CONFORMANCE.md) defines exact source/tree, actual test accounting, both
hosts, artifact reproducibility, capability denial, cleanup, required hosted checks and live
publication. Every row remains `not-evaluated` by this static report. Unmapped source proof
pairs remain explicit partial or unresolved gaps and retain their missing requirements.
Mapped guards, source bindings and typed exclusions still require authenticated execution
evidence. An excluded public input boundary does not certify an exact-limit runtime pass.
No containing commit or tree is embedded here: independent receipts bind this contract digest
to their actual candidate source, avoiding a source/hash cycle. A PR receipt cannot certify
the eventual merge commit or deployment.

## Canonical support JSON

The JSON is part of this Markdown document in `zryna.docs.bundle.v1`. The existing bundle
manifest authenticates document bytes and source provenance; there is no unmanifested sidecar.
An independently authenticated successful documentation run and manifest digest are still
required before website import. Source presence alone does not authenticate publication.

zryna.m4.support.v1:begin

```json
{
  "schema": "zryna.m4.support.v1",
  "authority": "static-contract",
  "registrySha256": "0175a09f4ec1efef49439877df40e9d77e8062c8e4cd53442416df6a2dbbd6d3",
  "authenticatedEvidence": "not-evaluated",
  "toolchains": {
    "rust": "1.97.1",
    "node": "22.22.1",
    "pnpm": "11.18.0",
    "wasmtime": "48.0.1",
    "witParser": "0.258.0",
    "wasi": "0.2.12",
    "browser": "Chrome for Testing 153.0.8010.12",
    "browserRunner": "playwright-core 1.63.0"
  },
  "hosts": [
    {
      "os": "linux",
      "architecture": "x64",
      "portabilityClaim": "exact-hosted-baseline-only"
    },
    {
      "os": "win32",
      "architecture": "x64",
      "portabilityClaim": "exact-hosted-baseline-only"
    }
  ],
  "witClosure": [
    {
      "logicalPath": "spec/wit/capability-profiles-v1/worlds.wit",
      "path": "spec/wit/capability-profiles-v1/worlds.wit",
      "bytes": 1129,
      "sha256": "cee8923feb7e50b545a56ecf01c14ca2fd9264e2dfdbe9f54eca6b07f0886cfe"
    },
    {
      "logicalPath": "wasi/cli/command.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/command.wit",
      "bytes": 154,
      "sha256": "31501d47bbb3b114c4fcb5218d42e69a4583d298384518c6adda1f2f97442153"
    },
    {
      "logicalPath": "wasi/cli/environment.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/environment.wit",
      "bytes": 811,
      "sha256": "5e58cd47b162e9f004bc9224a29c2ec3cf9e1a3d4c910154ce53742f828af91f"
    },
    {
      "logicalPath": "wasi/cli/exit.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/exit.wit",
      "bytes": 638,
      "sha256": "05453b1a965cdbd08fd1f9fa5df25f98746a68245bf6ff484ff661ed1e192c15"
    },
    {
      "logicalPath": "wasi/cli/imports.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/imports.wit",
      "bytes": 865,
      "sha256": "2be596bf68d2a4babe8ab388b390af96b4fcd7e4436eea599bd9c4d5705c22f7"
    },
    {
      "logicalPath": "wasi/cli/run.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/run.wit",
      "bytes": 116,
      "sha256": "a7df5376da8230e0509fffbb8355641af0765cc5cff8064bbe58fef1cbbe63c1"
    },
    {
      "logicalPath": "wasi/cli/stdio.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/stdio.wit",
      "bytes": 547,
      "sha256": "87e289b61c200a8cfc615767ef74920da51fdd75d7fac413dba67fff76e0f95f"
    },
    {
      "logicalPath": "wasi/cli/terminal.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/terminal.wit",
      "bytes": 2021,
      "sha256": "fc87cd0bc3f76ff7181d3666617a38ee10239f8e0344db2a52cb79f46c6b2d31"
    },
    {
      "logicalPath": "wasi/clocks/monotonic-clock.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/clocks/monotonic-clock.wit",
      "bytes": 1954,
      "sha256": "3453cd071d43c1045d417adea2bec82d66580639b71c09cad641f8a5753da947"
    },
    {
      "logicalPath": "wasi/clocks/timezone.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/clocks/timezone.wit",
      "bytes": 2220,
      "sha256": "657bb268af30308f7b34f28e0b2bb83f69655789158813dd9926e10b57ebba43"
    },
    {
      "logicalPath": "wasi/clocks/wall-clock.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/clocks/wall-clock.wit",
      "bytes": 1783,
      "sha256": "59a16c2aa86e89bb0a15eeb81b4205ea607cc7e97bebffdf62622b369da9b3d5"
    },
    {
      "logicalPath": "wasi/clocks/world.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/clocks/world.wit",
      "bytes": 240,
      "sha256": "b998c5cba8ef680051e83fe12bdc9d579775efe05bc93c27202c125ecc82e76d"
    },
    {
      "logicalPath": "wasi/filesystem/preopens.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/filesystem/preopens.wit",
      "bytes": 295,
      "sha256": "d67e1e166fdcbd0336784b774997d9af777b3843bca21cc651ba88f524f716e1"
    },
    {
      "logicalPath": "wasi/filesystem/types.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/filesystem/types.wit",
      "bytes": 28872,
      "sha256": "ff4b5f880ef38afb462fa37f00f75a75b476feb5dc0fc2da8a3633e2ae592ee6"
    },
    {
      "logicalPath": "wasi/filesystem/world.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/filesystem/world.wit",
      "bytes": 170,
      "sha256": "3b126a50383dc58c7b85bfbde38836179a3df2acc9cddd3130d8dd112b77b2ae"
    },
    {
      "logicalPath": "wasi/http/handler.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/http/handler.wit",
      "bytes": 1959,
      "sha256": "1f66174b4058b102367e69ce8aa657b3e9dbc767f6ade960108faaa1731f0945"
    },
    {
      "logicalPath": "wasi/http/proxy.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/http/proxy.wit",
      "bytes": 1938,
      "sha256": "8342e207c6ddcd642babce6e744a940c6521892e1370562c7918eb2116de379c"
    },
    {
      "logicalPath": "wasi/http/types.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/http/types.wit",
      "bytes": 27964,
      "sha256": "d67973c394247f8ed912368ef81a47408254163e26f11de2d1440ba3f4b66eec"
    },
    {
      "logicalPath": "wasi/io/error.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/io/error.wit",
      "bytes": 1507,
      "sha256": "a4eb0f5805db705c3c9754d5fc1396c5d75ada2628a4c0ac599310e194e38a9b"
    },
    {
      "logicalPath": "wasi/io/poll.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/io/poll.wit",
      "bytes": 1796,
      "sha256": "385347a39e16284d6c78e9776014be4a5e90aeee74c8de4eabbc6ea5e3b1b2c5"
    },
    {
      "logicalPath": "wasi/io/streams.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/io/streams.wit",
      "bytes": 11870,
      "sha256": "8b667deda5c89045ff30cea91693cc5cf1510cc2fdd96a20053459c6bf27e7e9"
    },
    {
      "logicalPath": "wasi/io/world.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/io/world.wit",
      "bytes": 161,
      "sha256": "366f9d1a2b5b11cbdbb7a15155b25aa6999c1d1da30e4ba58047d11526014be3"
    },
    {
      "logicalPath": "wasi/random/insecure-seed.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/random/insecure-seed.wit",
      "bytes": 1157,
      "sha256": "ef468057768649dd74b23baf659626fd80fc7b653cdb2347437503ffa03fd00a"
    },
    {
      "logicalPath": "wasi/random/insecure.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/random/insecure.wit",
      "bytes": 944,
      "sha256": "5c3f5c1e2250edc4db2685a983880822304d0a0847c88e068f2a4a04209f7be1"
    },
    {
      "logicalPath": "wasi/random/random.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/random/random.wit",
      "bytes": 1230,
      "sha256": "c9553bdc37f0306e7d6908de669330675d528e79762954ed1539f32dbe9f61c4"
    },
    {
      "logicalPath": "wasi/random/world.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/random/world.wit",
      "bytes": 223,
      "sha256": "fc32132c0aee89c0a846dfa5358dfe2143350acca1a470052270bf427815110d"
    },
    {
      "logicalPath": "wasi/sockets/instance-network.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/instance-network.wit",
      "bytes": 301,
      "sha256": "1e4a5d97df44421503a169e1d514eca2cf160f545c44d320ce8a82f726a40cf8"
    },
    {
      "logicalPath": "wasi/sockets/ip-name-lookup.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/ip-name-lookup.wit",
      "bytes": 2797,
      "sha256": "9bbdac3007844acc4a292736be3fe01e39df2ea968f8d1e4568fec20869752f0"
    },
    {
      "logicalPath": "wasi/sockets/network.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/network.wit",
      "bytes": 5180,
      "sha256": "7813868ee56552a434319f5fbf141d55cc1d37272b2976debcc75a50b9cb52e6"
    },
    {
      "logicalPath": "wasi/sockets/tcp-create-socket.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/tcp-create-socket.wit",
      "bytes": 1594,
      "sha256": "96a6a1a93b859127ee60273f0a77cda8d60930f7952174fb22469293b5da6a3a"
    },
    {
      "logicalPath": "wasi/sockets/tcp.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/tcp.wit",
      "bytes": 21317,
      "sha256": "1e9d17a8e0770abbc2ddead9dcc66793393bafe549f4b7effe73d8ad27272295"
    },
    {
      "logicalPath": "wasi/sockets/udp-create-socket.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/udp-create-socket.wit",
      "bytes": 1584,
      "sha256": "c52b9bf91ef4e30e7ec08ad1fc9e77ef005a0a87648c5d2e416dd6e35369f08c"
    },
    {
      "logicalPath": "wasi/sockets/udp.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/udp.wit",
      "bytes": 15936,
      "sha256": "83d0808198c95f261897676a54c93f1914061190d7a08c99847a6e3ddbeabada"
    },
    {
      "logicalPath": "wasi/sockets/world.wit",
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/world.wit",
      "bytes": 436,
      "sha256": "abf77820f4208da60dab68dc9be63c510eb0e5083014963dc478326798cd771b"
    }
  ],
  "requiredCaseKindsAuthority": "independent-mode-contract-plan",
  "contractPlan": {
    "path": "tests/m4-limit-contracts-v1.json",
    "bytes": 88928,
    "sha256": "b35368682735c16683f45929f52c5343b8e28c23e0927309a462d2a66526da6f"
  },
  "modeRequirements": [
    {
      "mode": "checkout-build",
      "requiredCaseKinds": [
        "positive",
        "negative",
        "identity",
        "reproducibility",
        "capability-denial"
      ]
    },
    {
      "mode": "checkout-browser",
      "requiredCaseKinds": [
        "positive",
        "negative",
        "identity",
        "reproducibility",
        "capability-denial"
      ]
    },
    {
      "mode": "checkout-run",
      "requiredCaseKinds": [
        "positive",
        "negative",
        "identity",
        "reproducibility",
        "capability-denial",
        "publication",
        "lifecycle"
      ]
    },
    {
      "mode": "fresh-content-verified-installation",
      "requiredCaseKinds": [
        "positive",
        "negative",
        "identity",
        "reproducibility",
        "capability-denial",
        "publication",
        "lifecycle"
      ]
    },
    {
      "mode": "checkout-serve",
      "requiredCaseKinds": [
        "positive",
        "negative",
        "identity",
        "reproducibility",
        "capability-denial",
        "lifecycle"
      ]
    }
  ],
  "limitContracts": [
    {
      "id": "component-component-bytes",
      "rowIds": [
        "component-linux",
        "component-windows"
      ],
      "limitId": "component-bytes",
      "minimum": 0,
      "maximum": 1048576,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "emission",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/scalar_component/audit.rs",
          "symbol": "MAX_COMPONENT_BYTES/check_byte_budget"
        }
      ],
      "guardScopes": [
        "production-component-audit",
        "production-component-audit"
      ],
      "publicRequirement": "emission-only",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "component-public-build"
      ]
    },
    {
      "id": "browser-component-bytes",
      "rowIds": [
        "browser-linux",
        "browser-windows"
      ],
      "limitId": "component-bytes",
      "minimum": 0,
      "maximum": 1048576,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "loader",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/scalar_component/audit.rs",
          "symbol": "MAX_COMPONENT_BYTES/check_byte_budget owning audit ceiling"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/scalar_component/mod.rs",
          "symbol": "emit_scalar_component canonical authenticated browser component factory"
        },
        {
          "path": "crates/zryna-driver/src/browser_component.rs",
          "symbol": "generate from ValidatedScalarComponent"
        },
        {
          "path": "crates/zryna-driver/src/browser_component/loader.js",
          "symbol": "instantiateBrowserComponent exact sealed expectedBytes and componentSha256"
        }
      ],
      "guardScopes": [
        "production-component-audit",
        "production-component-audit"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-browser-source-authenticated-canonical-component-byte-boundary",
        "public-browser-source-authenticated-canonical-component-byte-boundary"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "browser-node-public",
        "browser-chrome-public"
      ]
    },
    {
      "id": "browser-deadline",
      "rowIds": [
        "browser-linux",
        "browser-windows"
      ],
      "limitId": "deadline",
      "minimum": 1,
      "maximum": 30000,
      "unit": "milliseconds",
      "semantics": "reject-first-extra",
      "domain": "loader",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/browser_component/loader.js",
          "symbol": "instantiateBrowserComponent deadlineMs accepted1..30000"
        },
        {
          "path": "tests/browser-component/run.mjs",
          "symbol": "actual Node/Chrome deadline admission corpus"
        }
      ],
      "guardScopes": [
        "public-node-loader-option-admission",
        "public-node-loader-option-admission"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "pinned-chrome-loader-option-admission",
        "pinned-chrome-loader-option-admission"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "browser-node-public",
        "browser-chrome-public"
      ]
    },
    {
      "id": "command-component-bytes",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "component-bytes",
      "minimum": 0,
      "maximum": 1048576,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit.rs",
          "symbol": "audit byte ceiling"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/shell.rs",
          "symbol": "MAX_BYTES"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit.rs",
          "symbol": "audit authenticated sourcecode/deny padding"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact.rs",
          "symbol": "Artifact factory/issuer/source/code/composition canonical revalidation"
        },
        {
          "path": "crates/zryna-syntax/src/command_h1_v1.rs",
          "symbol": "authenticated public H1 source/literal spans"
        },
        {
          "path": "crates/zryna-source/src/lib.rs",
          "symbol": "source bytes/span authentication"
        }
      ],
      "guardScopes": [
        "private-H1-whole-graph-component-byte-policy-language-denies-padding",
        "private-H1-whole-graph-component-byte-policy-language-denies-padding"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-checkout-H1-source-authenticated-canonical-ceiling-execution",
        "public-checkout-H1-source-authenticated-canonical-ceiling-execution"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-request-bytes",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "request-bytes",
      "minimum": 0,
      "maximum": 4096,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_request.rs",
          "symbol": "admit/MAX_REQUEST_BYTES/MAX_KEY_BYTES/MAX_VALUE_BYTES"
        }
      ],
      "guardScopes": [
        "production-request-parser",
        "production-request-parser"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-key-bytes",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "key-bytes",
      "minimum": 1,
      "maximum": 64,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_request.rs",
          "symbol": "admit/MAX_REQUEST_BYTES/MAX_KEY_BYTES/MAX_VALUE_BYTES"
        }
      ],
      "guardScopes": [
        "production-request-parser",
        "production-request-parser"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-value-bytes",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "value-bytes",
      "minimum": 0,
      "maximum": 1024,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_request.rs",
          "symbol": "admit/MAX_REQUEST_BYTES/MAX_KEY_BYTES/MAX_VALUE_BYTES"
        }
      ],
      "guardScopes": [
        "production-request-parser",
        "production-request-parser"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-manifest-bytes",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "manifest-bytes",
      "minimum": 0,
      "maximum": 16384,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/manifest/observation.rs",
          "symbol": "limits max_manifest_bytes16384"
        },
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/manifest/validation.rs",
          "symbol": "validate"
        }
      ],
      "guardScopes": [
        "production-manifest-decoder",
        "production-manifest-decoder"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-transfer-live",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "transfer-live",
      "minimum": 0,
      "maximum": 16,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
          "symbol": "canonical storage limits/accounting"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
          "symbol": "audit sealed storage"
        }
      ],
      "guardScopes": [
        "sealed-storage-core-execution",
        "sealed-storage-core-execution"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-transfer-allocations",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "transfer-allocations",
      "minimum": 0,
      "maximum": 4096,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
          "symbol": "canonical storage limits/accounting"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
          "symbol": "audit sealed storage"
        }
      ],
      "guardScopes": [
        "sealed-storage-core-execution",
        "sealed-storage-core-execution"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-transfer-allocation-bytes",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "transfer-allocation-bytes",
      "minimum": 0,
      "maximum": 4096,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
          "symbol": "canonical storage limits/accounting"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
          "symbol": "audit sealed storage"
        }
      ],
      "guardScopes": [
        "sealed-storage-core-execution",
        "sealed-storage-core-execution"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-transfer-bytes",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "transfer-bytes",
      "minimum": 0,
      "maximum": 1048576,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
          "symbol": "canonical storage limits/accounting"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
          "symbol": "audit sealed storage"
        }
      ],
      "guardScopes": [
        "sealed-storage-core-execution",
        "sealed-storage-core-execution"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-fuel",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "fuel",
      "minimum": 0,
      "maximum": 100000,
      "unit": "fuel-units",
      "semantics": "fixed-enforcement",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-engine-measured-fixed-fuel-enforcement"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "command-fuel-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-deadline",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "deadline",
      "minimum": 1,
      "maximum": 5000,
      "unit": "milliseconds",
      "semantics": "fixed-enforcement",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "sealed-command-execution"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "command-deadline-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-wasm-stack",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "wasm-stack",
      "minimum": 0,
      "maximum": 65536,
      "unit": "bytes",
      "semantics": "fixed-enforcement",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-engine-stack-trap"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "command-wasm-stack-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-backtrace-frames",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "backtrace-frames",
      "minimum": 0,
      "maximum": 1,
      "unit": "count",
      "semantics": "truncate-output",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-engine-diagnostic-frame-truncation"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "command-backtrace-frames-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-instances",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "instances",
      "minimum": 0,
      "maximum": 2,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-StoreLimits-raw-core-instances",
        "private-production-StoreLimits-raw-core-instances"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-memories",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "memories",
      "minimum": 0,
      "maximum": 1,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-StoreLimits-raw-core-object-count",
        "private-production-StoreLimits-raw-core-object-count"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-tables",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "tables",
      "minimum": 0,
      "maximum": 0,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-StoreLimits-raw-core-object-count",
        "private-production-StoreLimits-raw-core-object-count"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-memory-bytes",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "memory-bytes",
      "minimum": 0,
      "maximum": 16777216,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-ResourceLimiter-memory-policy",
        "private-production-ResourceLimiter-memory-policy"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "command-memory-pages",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "memory-pages",
      "minimum": 0,
      "maximum": 256,
      "unit": "pages",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-StoreLimits-host-Memory-grow",
        "private-production-StoreLimits-host-Memory-grow"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "command-public-pure",
        "command-public-input"
      ]
    },
    {
      "id": "installed-command-component-bytes",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "component-bytes",
      "minimum": 0,
      "maximum": 1048576,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit.rs",
          "symbol": "audit byte ceiling"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/shell.rs",
          "symbol": "MAX_BYTES"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit.rs",
          "symbol": "audit authenticated sourcecode/deny padding"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact.rs",
          "symbol": "Artifact factory/issuer/source/code/composition canonical revalidation"
        },
        {
          "path": "crates/zryna-syntax/src/command_h1_v1.rs",
          "symbol": "authenticated public H1 source/literal spans"
        },
        {
          "path": "crates/zryna-source/src/lib.rs",
          "symbol": "source bytes/span authentication"
        }
      ],
      "guardScopes": [
        "private-H1-whole-graph-component-byte-policy-language-denies-padding",
        "private-H1-whole-graph-component-byte-policy-language-denies-padding"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "fresh-content-verified-installed-command",
        "fresh-content-verified-installed-command"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-request-bytes",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "request-bytes",
      "minimum": 0,
      "maximum": 4096,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_request.rs",
          "symbol": "admit/MAX_REQUEST_BYTES/MAX_KEY_BYTES/MAX_VALUE_BYTES"
        }
      ],
      "guardScopes": [
        "production-request-parser",
        "production-request-parser"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-key-bytes",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "key-bytes",
      "minimum": 1,
      "maximum": 64,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_request.rs",
          "symbol": "admit/MAX_REQUEST_BYTES/MAX_KEY_BYTES/MAX_VALUE_BYTES"
        }
      ],
      "guardScopes": [
        "production-request-parser",
        "production-request-parser"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-value-bytes",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "value-bytes",
      "minimum": 0,
      "maximum": 1024,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_request.rs",
          "symbol": "admit/MAX_REQUEST_BYTES/MAX_KEY_BYTES/MAX_VALUE_BYTES"
        }
      ],
      "guardScopes": [
        "production-request-parser",
        "production-request-parser"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-manifest-bytes",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "manifest-bytes",
      "minimum": 0,
      "maximum": 16384,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/manifest/observation.rs",
          "symbol": "limits max_manifest_bytes16384"
        },
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/manifest/validation.rs",
          "symbol": "validate"
        }
      ],
      "guardScopes": [
        "production-manifest-decoder",
        "production-manifest-decoder"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-transfer-live",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "transfer-live",
      "minimum": 0,
      "maximum": 16,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
          "symbol": "canonical storage limits/accounting"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
          "symbol": "audit sealed storage"
        }
      ],
      "guardScopes": [
        "sealed-storage-core-execution",
        "sealed-storage-core-execution"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-transfer-allocations",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "transfer-allocations",
      "minimum": 0,
      "maximum": 4096,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
          "symbol": "canonical storage limits/accounting"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
          "symbol": "audit sealed storage"
        }
      ],
      "guardScopes": [
        "sealed-storage-core-execution",
        "sealed-storage-core-execution"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-transfer-allocation-bytes",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "transfer-allocation-bytes",
      "minimum": 0,
      "maximum": 4096,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
          "symbol": "canonical storage limits/accounting"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
          "symbol": "audit sealed storage"
        }
      ],
      "guardScopes": [
        "sealed-storage-core-execution",
        "sealed-storage-core-execution"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-transfer-bytes",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "transfer-bytes",
      "minimum": 0,
      "maximum": 1048576,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
          "symbol": "canonical storage limits/accounting"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
          "symbol": "audit sealed storage"
        }
      ],
      "guardScopes": [
        "sealed-storage-core-execution",
        "sealed-storage-core-execution"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-fuel",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "fuel",
      "minimum": 0,
      "maximum": 100000,
      "unit": "fuel-units",
      "semantics": "fixed-enforcement",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-engine-measured-fixed-fuel-enforcement"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "installed-command-fuel-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-deadline",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "deadline",
      "minimum": 1,
      "maximum": 5000,
      "unit": "milliseconds",
      "semantics": "fixed-enforcement",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "sealed-command-execution"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "installed-command-deadline-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-wasm-stack",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "wasm-stack",
      "minimum": 0,
      "maximum": 65536,
      "unit": "bytes",
      "semantics": "fixed-enforcement",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-engine-stack-trap"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "installed-command-wasm-stack-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-backtrace-frames",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "backtrace-frames",
      "minimum": 0,
      "maximum": 1,
      "unit": "count",
      "semantics": "truncate-output",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-engine-diagnostic-frame-truncation"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "installed-command-backtrace-frames-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-instances",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "instances",
      "minimum": 0,
      "maximum": 2,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-StoreLimits-raw-core-instances",
        "private-production-StoreLimits-raw-core-instances"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-memories",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "memories",
      "minimum": 0,
      "maximum": 1,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-StoreLimits-raw-core-object-count",
        "private-production-StoreLimits-raw-core-object-count"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-tables",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "tables",
      "minimum": 0,
      "maximum": 0,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-StoreLimits-raw-core-object-count",
        "private-production-StoreLimits-raw-core-object-count"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-memory-bytes",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "memory-bytes",
      "minimum": 0,
      "maximum": 16777216,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-ResourceLimiter-memory-policy",
        "private-production-ResourceLimiter-memory-policy"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "installed-command-memory-pages",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "memory-pages",
      "minimum": 0,
      "maximum": 256,
      "unit": "pages",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "guardScopes": [
        "private-production-StoreLimits-host-Memory-grow",
        "private-production-StoreLimits-host-Memory-grow"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "installed-pure",
        "installed-found"
      ]
    },
    {
      "id": "server-status-body-bytes",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "body-bytes",
      "minimum": 1,
      "maximum": 65536,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-header-bytes",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "header-bytes",
      "minimum": 64,
      "maximum": 8192,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-header-fields",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "header-fields",
      "minimum": 0,
      "maximum": 32,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-path-bytes",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "path-bytes",
      "minimum": 0,
      "maximum": 256,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-attempts",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "attempts",
      "minimum": 1,
      "maximum": 64,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "public-server-configuration-admission"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "public-server-configuration-admission"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-configuration-bytes",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "configuration-bytes",
      "minimum": 1,
      "maximum": 1024,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_workspace/approval.rs",
          "symbol": "Listener::capture"
        },
        {
          "path": "crates/zryna-driver/src/command_request/capture.rs",
          "symbol": "CapturedFile::capture"
        }
      ],
      "guardScopes": [
        "public-CLI-private-file-capture-and-loopback-execution",
        "public-CLI-private-file-capture-and-loopback-execution"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-CLI-private-file-capture-and-loopback-execution",
        "public-CLI-private-file-capture-and-loopback-execution"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-listener-approval-bytes",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "listener-approval-bytes",
      "minimum": 1,
      "maximum": 1024,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_workspace/approval.rs",
          "symbol": "Listener::capture"
        },
        {
          "path": "crates/zryna-driver/src/command_request/capture.rs",
          "symbol": "CapturedFile::capture"
        }
      ],
      "guardScopes": [
        "public-CLI-private-file-capture-and-loopback-execution",
        "public-CLI-private-file-capture-and-loopback-execution"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-CLI-private-file-capture-and-loopback-execution",
        "public-CLI-private-file-capture-and-loopback-execution"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-request-deadline",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "request-deadline",
      "minimum": 1,
      "maximum": 5000,
      "unit": "milliseconds",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "public-server-configuration-admission",
        "public-server-configuration-admission"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-server-configuration-admission",
        "public-server-configuration-admission"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-service-lifetime",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "service-lifetime",
      "minimum": 1,
      "maximum": 30000,
      "unit": "milliseconds",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "public-server-configuration-admission",
        "public-server-configuration-admission"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-server-configuration-admission",
        "public-server-configuration-admission"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-concurrent-requests",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "concurrent-requests",
      "minimum": 0,
      "maximum": 1,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::envelope requests1"
        },
        {
          "path": "crates/zryna-driver/src/server_lifecycle/registry.rs",
          "symbol": "Registry admission active request quota"
        },
        {
          "path": "crates/zryna-driver/src/server_lifecycle/limits.rs",
          "symbol": "Limits::validate"
        }
      ],
      "guardScopes": [
        "private-Config-bound-production-lifecycle-real-Store-admission",
        "private-Config-bound-production-lifecycle-real-Store-admission"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-guest-memory",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "guest-memory",
      "minimum": 0,
      "maximum": 65536,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/envelope.rs",
          "symbol": "Envelope::validate/store_limits/engine"
        }
      ],
      "guardScopes": [
        "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy",
        "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-guest-fuel",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "guest-fuel",
      "minimum": 0,
      "maximum": 100000,
      "unit": "fuel-units",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/envelope.rs",
          "symbol": "Envelope::validate/store_limits/engine"
        }
      ],
      "guardScopes": [
        "private-Config-bound-Envelope-fuel-admission",
        "private-Config-bound-Envelope-fuel-admission"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-guest-resources",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "guest-resources",
      "minimum": 0,
      "maximum": 5,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_runtime/resources.rs",
          "symbol": "Resources::insert/take"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::enter"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke fixed mediated operation topology"
        }
      ],
      "guardScopes": [
        "private-production-resource-registry",
        "private-production-resource-registry"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "server-status-guest-resources-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-guest-callbacks",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "guest-callbacks",
      "minimum": 0,
      "maximum": 8,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_runtime/resources.rs",
          "symbol": "Resources::insert/take"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::enter"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke fixed mediated operation topology"
        }
      ],
      "guardScopes": [
        "private-production-Host-enter-policy",
        "private-production-Host-enter-policy"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "server-status-guest-callbacks-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-status-component-bytes",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "component-bytes",
      "minimum": 0,
      "maximum": 1048576,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/audit.rs",
          "symbol": "audit MAX_BYTES before decoder/canonical replay"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/shell.rs",
          "symbol": "MAX_BYTES canonical graph"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/bridge.rs",
          "symbol": "fixed Reply/ClockRead bridge"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/mod.rs",
          "symbol": "scalar_export enforces noarg i32 scalar signature"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/prepared.rs",
          "symbol": "Prepared::new_public solefunction"
        },
        {
          "path": "crates/zryna-ir/src/lib.rs",
          "symbol": "MAX_IR_EXPRESSIONS_PER_FUNCTION/MAX_IR_EXPORT_NAME_BYTES"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/scalar_audit.rs",
          "symbol": "audit_profile restricts canonical scalar opcodes"
        }
      ],
      "guardScopes": [],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "server-status-component-bytes-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-status-public"
      ]
    },
    {
      "id": "server-clock-body-bytes",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "body-bytes",
      "minimum": 1,
      "maximum": 65536,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-header-bytes",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "header-bytes",
      "minimum": 64,
      "maximum": 8192,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-header-fields",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "header-fields",
      "minimum": 0,
      "maximum": 32,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-path-bytes",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "path-bytes",
      "minimum": 0,
      "maximum": 256,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "production-loopback-transport"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-attempts",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "attempts",
      "minimum": 1,
      "maximum": 64,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "production-loopback-transport",
        "public-server-configuration-admission"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "production-loopback-transport",
        "public-server-configuration-admission"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-configuration-bytes",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "configuration-bytes",
      "minimum": 1,
      "maximum": 1024,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_workspace/approval.rs",
          "symbol": "Listener::capture"
        },
        {
          "path": "crates/zryna-driver/src/command_request/capture.rs",
          "symbol": "CapturedFile::capture"
        }
      ],
      "guardScopes": [
        "public-CLI-private-file-capture-and-loopback-execution",
        "public-CLI-private-file-capture-and-loopback-execution"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-CLI-private-file-capture-and-loopback-execution",
        "public-CLI-private-file-capture-and-loopback-execution"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-listener-approval-bytes",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "listener-approval-bytes",
      "minimum": 1,
      "maximum": 1024,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_workspace/approval.rs",
          "symbol": "Listener::capture"
        },
        {
          "path": "crates/zryna-driver/src/command_request/capture.rs",
          "symbol": "CapturedFile::capture"
        }
      ],
      "guardScopes": [
        "public-CLI-private-file-capture-and-loopback-execution",
        "public-CLI-private-file-capture-and-loopback-execution"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-CLI-private-file-capture-and-loopback-execution",
        "public-CLI-private-file-capture-and-loopback-execution"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-request-deadline",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "request-deadline",
      "minimum": 1,
      "maximum": 5000,
      "unit": "milliseconds",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "public-server-configuration-admission",
        "public-server-configuration-admission"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-server-configuration-admission",
        "public-server-configuration-admission"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-service-lifetime",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "service-lifetime",
      "minimum": 1,
      "maximum": 30000,
      "unit": "milliseconds",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::validate/Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_transport/framing.rs",
          "symbol": "parse"
        }
      ],
      "guardScopes": [
        "public-server-configuration-admission",
        "public-server-configuration-admission"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-server-configuration-admission",
        "public-server-configuration-admission"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-concurrent-requests",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "concurrent-requests",
      "minimum": 0,
      "maximum": 1,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::envelope requests1"
        },
        {
          "path": "crates/zryna-driver/src/server_lifecycle/registry.rs",
          "symbol": "Registry admission active request quota"
        },
        {
          "path": "crates/zryna-driver/src/server_lifecycle/limits.rs",
          "symbol": "Limits::validate"
        }
      ],
      "guardScopes": [
        "private-Config-bound-production-lifecycle-real-Store-admission",
        "private-Config-bound-production-lifecycle-real-Store-admission"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-guest-memory",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "guest-memory",
      "minimum": 0,
      "maximum": 65536,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/envelope.rs",
          "symbol": "Envelope::validate/store_limits/engine"
        }
      ],
      "guardScopes": [
        "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy",
        "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-guest-fuel",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "guest-fuel",
      "minimum": 0,
      "maximum": 100000,
      "unit": "fuel-units",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_transport/config.rs",
          "symbol": "Config::envelope"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/envelope.rs",
          "symbol": "Envelope::validate/store_limits/engine"
        }
      ],
      "guardScopes": [
        "private-Config-bound-Envelope-fuel-admission",
        "private-Config-bound-Envelope-fuel-admission"
      ],
      "publicRequirement": "guard-prerequisite",
      "publicScopes": [],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-guest-resources",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "guest-resources",
      "minimum": 0,
      "maximum": 5,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_runtime/resources.rs",
          "symbol": "Resources::insert/take"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::enter"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke fixed mediated operation topology"
        }
      ],
      "guardScopes": [
        "private-production-resource-registry",
        "private-production-resource-registry"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "server-clock-guest-resources-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-guest-callbacks",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "guest-callbacks",
      "minimum": 0,
      "maximum": 8,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_runtime/resources.rs",
          "symbol": "Resources::insert/take"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::enter"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke fixed mediated operation topology"
        }
      ],
      "guardScopes": [
        "private-production-Host-enter-policy",
        "private-production-Host-enter-policy"
      ],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "server-clock-guest-callbacks-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-component-bytes",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "component-bytes",
      "minimum": 0,
      "maximum": 1048576,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/audit.rs",
          "symbol": "audit MAX_BYTES before decoder/canonical replay"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/shell.rs",
          "symbol": "MAX_BYTES canonical graph"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/bridge.rs",
          "symbol": "fixed Reply/ClockRead bridge"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/mod.rs",
          "symbol": "scalar_export enforces noarg i32 scalar signature"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/prepared.rs",
          "symbol": "Prepared::new_public solefunction"
        },
        {
          "path": "crates/zryna-ir/src/lib.rs",
          "symbol": "MAX_IR_EXPRESSIONS_PER_FUNCTION/MAX_IR_EXPORT_NAME_BYTES"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/scalar_audit.rs",
          "symbol": "audit_profile restricts canonical scalar opcodes"
        }
      ],
      "guardScopes": [],
      "publicRequirement": "reviewed-exclusion",
      "publicScopes": [],
      "allowedExclusionIds": [
        "server-clock-component-bytes-public-input-exclusion"
      ],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-guest-request-bytes",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "guest-request-bytes",
      "minimum": 1,
      "maximum": 2048,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_workspace/guest.rs",
          "symbol": "Guest::capture exact request1/separate approval1"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/grants.rs",
          "symbol": "Approval::one_monotonic_clock_read/Grants::admit effective<=approval; privateMAX_CLOCK_READS16 notpublic1"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::new/Host::clock remaining quota checked_sub before provider/output"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke monotonic now"
        }
      ],
      "guardScopes": [
        "public-checkout-server-clock",
        "public-checkout-server-clock"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-checkout-server-clock",
        "public-checkout-server-clock"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-guest-approval-bytes",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "guest-approval-bytes",
      "minimum": 1,
      "maximum": 1024,
      "unit": "bytes",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_workspace/guest.rs",
          "symbol": "Guest::capture exact request1/separate approval1"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/grants.rs",
          "symbol": "Approval::one_monotonic_clock_read/Grants::admit effective<=approval; privateMAX_CLOCK_READS16 notpublic1"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::new/Host::clock remaining quota checked_sub before provider/output"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke monotonic now"
        }
      ],
      "guardScopes": [
        "public-checkout-server-clock",
        "public-checkout-server-clock"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-checkout-server-clock",
        "public-checkout-server-clock"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    },
    {
      "id": "server-clock-clock-reads",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "clock-reads",
      "minimum": 1,
      "maximum": 1,
      "unit": "count",
      "semantics": "reject-first-extra",
      "domain": "runtime",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_workspace/guest.rs",
          "symbol": "Guest::capture exact request1/separate approval1"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/grants.rs",
          "symbol": "Approval::one_monotonic_clock_read/Grants::admit effective<=approval; privateMAX_CLOCK_READS16 notpublic1"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::new/Host::clock remaining quota checked_sub before provider/output"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke monotonic now"
        }
      ],
      "guardScopes": [
        "private-production-host-operation-real-Store-clock-quota",
        "private-production-host-operation-real-Store-clock-quota"
      ],
      "publicRequirement": "reachable-pair",
      "publicScopes": [
        "public-server-clock-request-admission-and-execution",
        "public-server-clock-request-admission"
      ],
      "allowedExclusionIds": [],
      "requiredBindingCaseIds": [
        "server-public-envelope-binding",
        "server-clock-public"
      ]
    }
  ],
  "limitExclusions": [
    {
      "id": "command-fuel-public-input-exclusion",
      "code": "fixed-fuel-budget-not-selectable-input",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "fuel",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "proofCaseIds": [
        "h1-fixed-fuel-enforcement"
      ],
      "quantitative": null
    },
    {
      "id": "command-deadline-public-input-exclusion",
      "code": "fixed-duration-not-selectable-input",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "deadline",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "proofCaseIds": [
        "command-deadline-cleanup"
      ],
      "quantitative": null
    },
    {
      "id": "command-wasm-stack-public-input-exclusion",
      "code": "native-stack-byte-occupancy-not-observable",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "wasm-stack",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "proofCaseIds": [
        "h1-private-stack-enforcement"
      ],
      "quantitative": null
    },
    {
      "id": "command-backtrace-frames-public-input-exclusion",
      "code": "diagnostic-frame-truncation-not-rejection",
      "rowIds": [
        "command-linux",
        "command-windows"
      ],
      "limitId": "backtrace-frames",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "proofCaseIds": [
        "h1-backtrace-output-cap"
      ],
      "quantitative": null
    },
    {
      "id": "installed-command-fuel-public-input-exclusion",
      "code": "fixed-fuel-budget-not-selectable-input",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "fuel",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "proofCaseIds": [
        "h1-fixed-fuel-enforcement"
      ],
      "quantitative": null
    },
    {
      "id": "installed-command-deadline-public-input-exclusion",
      "code": "fixed-duration-not-selectable-input",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "deadline",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "proofCaseIds": [
        "command-deadline-cleanup"
      ],
      "quantitative": null
    },
    {
      "id": "installed-command-wasm-stack-public-input-exclusion",
      "code": "native-stack-byte-occupancy-not-observable",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "wasm-stack",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "proofCaseIds": [
        "h1-private-stack-enforcement"
      ],
      "quantitative": null
    },
    {
      "id": "installed-command-backtrace-frames-public-input-exclusion",
      "code": "diagnostic-frame-truncation-not-rejection",
      "rowIds": [
        "installed-command-linux",
        "installed-command-windows"
      ],
      "limitId": "backtrace-frames",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
          "symbol": "engine/limits/FUEL/DEADLINE"
        }
      ],
      "proofCaseIds": [
        "h1-backtrace-output-cap"
      ],
      "quantitative": null
    },
    {
      "id": "server-status-guest-resources-public-input-exclusion",
      "code": "fixed-public-handler-cannot-exhaust-guard",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "guest-resources",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_runtime/resources.rs",
          "symbol": "Resources::insert/take"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::enter"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke fixed mediated operation topology"
        }
      ],
      "proofCaseIds": [
        "server-private-resource-quota-exact-limit",
        "server-private-resource-quota-first-extra",
        "server-public-envelope-binding",
        "server-status-public"
      ],
      "quantitative": null
    },
    {
      "id": "server-status-guest-callbacks-public-input-exclusion",
      "code": "fixed-public-handler-cannot-exhaust-guard",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "guest-callbacks",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_runtime/resources.rs",
          "symbol": "Resources::insert/take"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::enter"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke fixed mediated operation topology"
        }
      ],
      "proofCaseIds": [
        "server-private-callback-quota-exact-limit",
        "server-private-callback-quota-first-extra",
        "server-public-envelope-binding",
        "server-status-public"
      ],
      "quantitative": null
    },
    {
      "id": "server-status-component-bytes-public-input-exclusion",
      "code": "canonical-public-shape-below-ceiling",
      "rowIds": [
        "server-status-linux",
        "server-status-windows"
      ],
      "limitId": "component-bytes",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/audit.rs",
          "symbol": "audit MAX_BYTES before decoder/canonical replay"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/shell.rs",
          "symbol": "MAX_BYTES canonical graph"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/bridge.rs",
          "symbol": "fixed Reply/ClockRead bridge"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/mod.rs",
          "symbol": "scalar_export enforces noarg i32 scalar signature"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/prepared.rs",
          "symbol": "Prepared::new_public solefunction"
        },
        {
          "path": "crates/zryna-ir/src/lib.rs",
          "symbol": "MAX_IR_EXPRESSIONS_PER_FUNCTION/MAX_IR_EXPORT_NAME_BYTES"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/scalar_audit.rs",
          "symbol": "audit_profile restricts canonical scalar opcodes"
        }
      ],
      "proofCaseIds": [
        "server-public-shape-size-exclusion",
        "server-component-first-extra-length-guard",
        "server-public-envelope-binding",
        "server-status-public"
      ],
      "quantitative": {
        "kind": "canonical-single-scalar-component",
        "baselineCaseId": "server-public-shape-size-exclusion",
        "baselineMaximumBytes": 8212,
        "expressionNodes": 16383,
        "maximumLiteralBytes": 6,
        "exportOccurrences": 2,
        "maximumExportBytes": 128,
        "framingAllowanceBytes": 256,
        "maximumBytes": 66067
      }
    },
    {
      "id": "server-clock-guest-resources-public-input-exclusion",
      "code": "fixed-public-handler-cannot-exhaust-guard",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "guest-resources",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_runtime/resources.rs",
          "symbol": "Resources::insert/take"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::enter"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke fixed mediated operation topology"
        }
      ],
      "proofCaseIds": [
        "server-private-resource-quota-exact-limit",
        "server-private-resource-quota-first-extra",
        "server-public-envelope-binding",
        "server-clock-public"
      ],
      "quantitative": null
    },
    {
      "id": "server-clock-guest-callbacks-public-input-exclusion",
      "code": "fixed-public-handler-cannot-exhaust-guard",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "guest-callbacks",
      "sourceBindings": [
        {
          "path": "crates/zryna-driver/src/server_runtime/resources.rs",
          "symbol": "Resources::insert/take"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/host.rs",
          "symbol": "Host::enter"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/operations.rs",
          "symbol": "invoke fixed mediated operation topology"
        }
      ],
      "proofCaseIds": [
        "server-private-callback-quota-exact-limit",
        "server-private-callback-quota-first-extra",
        "server-public-envelope-binding",
        "server-clock-public"
      ],
      "quantitative": null
    },
    {
      "id": "server-clock-component-bytes-public-input-exclusion",
      "code": "canonical-public-shape-below-ceiling",
      "rowIds": [
        "server-clock-linux",
        "server-clock-windows"
      ],
      "limitId": "component-bytes",
      "sourceBindings": [
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/audit.rs",
          "symbol": "audit MAX_BYTES before decoder/canonical replay"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/shell.rs",
          "symbol": "MAX_BYTES canonical graph"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/bridge.rs",
          "symbol": "fixed Reply/ClockRead bridge"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/component_server/mod.rs",
          "symbol": "scalar_export enforces noarg i32 scalar signature"
        },
        {
          "path": "crates/zryna-driver/src/server_runtime/prepared.rs",
          "symbol": "Prepared::new_public solefunction"
        },
        {
          "path": "crates/zryna-ir/src/lib.rs",
          "symbol": "MAX_IR_EXPRESSIONS_PER_FUNCTION/MAX_IR_EXPORT_NAME_BYTES"
        },
        {
          "path": "crates/zryna-backend-webassembly/src/scalar_audit.rs",
          "symbol": "audit_profile restricts canonical scalar opcodes"
        }
      ],
      "proofCaseIds": [
        "server-public-shape-size-exclusion",
        "server-component-first-extra-length-guard",
        "server-public-envelope-binding",
        "server-clock-public"
      ],
      "quantitative": {
        "kind": "canonical-single-scalar-component",
        "baselineCaseId": "server-public-shape-size-exclusion",
        "baselineMaximumBytes": 8272,
        "expressionNodes": 16383,
        "maximumLiteralBytes": 6,
        "exportOccurrences": 2,
        "maximumExportBytes": 128,
        "framingAllowanceBytes": 256,
        "maximumBytes": 66127
      }
    }
  ],
  "sources": [
    {
      "path": ".github/workflows/browser-conformance.yml",
      "bytes": 4326,
      "sha256": "dd94fb12379c91825a40f8eb7f4a92a8e9a8f23ffcc40de9b656e92f42f6c2c0"
    },
    {
      "path": ".github/workflows/ci.yml",
      "bytes": 34527,
      "sha256": "204df1a4849cda57fde57e4c0e581294181db6ebde3a8133fe60a3dd1c801601"
    },
    {
      "path": ".github/workflows/documentation.yml",
      "bytes": 4026,
      "sha256": "0545f410fe47c72a964b6333dd2f6e8f2cf57b7a315a9b0afc3737640cb0e18a"
    },
    {
      "path": ".github/workflows/installed-command-h1.yml",
      "bytes": 5308,
      "sha256": "37f9945966567f50ad44beefe4f576a42cfc1ce429cfdf23a3a3140d16bf6cea"
    },
    {
      "path": "CONTRIBUTING.md",
      "bytes": 10747,
      "sha256": "b7d0ea3420968b17228ef3aceb90b8a5fda8a500f38b9c80ec99b946ed92a37b"
    },
    {
      "path": "Cargo.lock",
      "bytes": 51965,
      "sha256": "9545d525d63767831ae8611166c3294e0059d17ffde0961ba9b66e6237be7476"
    },
    {
      "path": "Cargo.toml",
      "bytes": 3137,
      "sha256": "71edfbf1d4b57caf90fa12a5bbac918ad9fe0965f3da8603f39f2cabaff0f898"
    },
    {
      "path": "LICENSE",
      "bytes": 11344,
      "sha256": "0002a5d91c25d453f5f72bfc792f427480c68c8e44834b8f0958787ad149f73f"
    },
    {
      "path": "NOTICE",
      "bytes": 1799,
      "sha256": "671bdd95d5a510fd1b1f4d65644f520ba55d6d516b91c4df5587d1d56e493701"
    },
    {
      "path": "adapters/typescript-6/package.json",
      "bytes": 1849,
      "sha256": "b29e03bce446f6fab1169e3c2cf476952d636b57cfc9a1ae8ea5ef549875afaa"
    },
    {
      "path": "adapters/typescript-6/src/limits-v3.mjs",
      "bytes": 123,
      "sha256": "c3e5d4adea4e08d4f9475269cee3f4e5f870ba052a6ea76ff1e6cb5fb23f6913"
    },
    {
      "path": "adapters/typescript-6/src/limits-v4.mjs",
      "bytes": 719,
      "sha256": "7c6417f75bee60072b65eb5fe7847254835dde4bb07d6500e53c797a7cfba29b"
    },
    {
      "path": "adapters/typescript-6/src/v4/boundary/configuration.mjs",
      "bytes": 5957,
      "sha256": "d8f7ef8062e336c94478129d3009284484ba36745befcdc8094fd29de6df4a90"
    },
    {
      "path": "adapters/typescript-6/src/v4/boundary/dispatch.mjs",
      "bytes": 1715,
      "sha256": "1f96827210a2b16dd6c739589685d1c053e1f26d3fe05bffdb4d8c08d6bceb71"
    },
    {
      "path": "adapters/typescript-6/src/v4/boundary/errors.mjs",
      "bytes": 446,
      "sha256": "b8409f3d4be9c6f4d70aef34158d41e9f5cd3c98c2e1117a5cf7c80ceb6438e2"
    },
    {
      "path": "adapters/typescript-6/src/v4/boundary/request.mjs",
      "bytes": 4636,
      "sha256": "3ca196b0b46a8684fb6b771c87fd07ba344cb7baee486541c5d25f049173476e"
    },
    {
      "path": "adapters/typescript-6/src/v4/boundary/transport.mjs",
      "bytes": 2833,
      "sha256": "6abc2158478b3bdb9048263f85a74ce35bd8960425db6a384e78477b02c0eff1"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/constructions.mjs",
      "bytes": 7092,
      "sha256": "247cfcf34a628abd9e3956808b5080e4ba2c555d245b89045c6c029468e451f8"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/data-declarations.mjs",
      "bytes": 5053,
      "sha256": "daf6146ceee7dda4169eaf67263e235d9cb4218d5b993d0057585e692197135c"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/diagnostics.mjs",
      "bytes": 2218,
      "sha256": "242c2c40f5683b322fbb8e0b29e1c0d6343c72a81d04943796e9b2987fdde6b4"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/expression-arena.mjs",
      "bytes": 923,
      "sha256": "f09e19068dc6c272a4ce882e40a7423d1eeea8d6172b12eef964d588060cf162"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/expressions.mjs",
      "bytes": 13206,
      "sha256": "a767019781ddd03f0aa7d4270ff8d2bfbb5dbca511ec954106f8741bef564f72"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/functions.mjs",
      "bytes": 3835,
      "sha256": "8278f75ee7803d1733832d893713415586d03ce2a13eaac006016ed47c36422c"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/imports.mjs",
      "bytes": 4110,
      "sha256": "f258c8ea8c1df190be34dba99316177beb9e664d3af5ed528c3293d0b02daaf4"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/matches.mjs",
      "bytes": 4894,
      "sha256": "e1775e50d486bd83e61bc0cdbec30cf7ede3823d173fb95424242da931ba98e2"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/names.mjs",
      "bytes": 1082,
      "sha256": "5c7bb3e124c1e41ea98d33615cfb1656efbb0609ea3e07c4e71c9e189fe8455d"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/source.mjs",
      "bytes": 4316,
      "sha256": "9892ecac270ac6c9881e107de3e986044e37f3b337cb87b61674983d342e87ee"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/spans.mjs",
      "bytes": 1470,
      "sha256": "e94c22d773c7035b4a36f0b1bddb70641797a393f5d2512054f78de0e5fcf116"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/statements.mjs",
      "bytes": 11754,
      "sha256": "2e8dfede5aac0561992438fffaad393ecdee3e836754211caf593864daada3c9"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/tokens.mjs",
      "bytes": 1409,
      "sha256": "5fd6718e26341f79e8bb3651c38d237cd9e9b3385f64d043e340431e3da79b9f"
    },
    {
      "path": "adapters/typescript-6/src/v4/syntax/types.mjs",
      "bytes": 4412,
      "sha256": "27a3715eb8fed6f96ac8e23a0bbc1431d1af7af857442d364546f2f7f3e030f4"
    },
    {
      "path": "adapters/typescript-6/src/worker-v3.mjs",
      "bytes": 39227,
      "sha256": "665ab7eaebae5110377b9832e746859ed4b351c26548c489970ba1d08cd04552"
    },
    {
      "path": "adapters/typescript-6/src/worker-v4.mjs",
      "bytes": 77,
      "sha256": "50bbc8a578a83d544e6c6be0c0bcf1f36c6e2b916d5bfb468be450da4aecab07"
    },
    {
      "path": "adapters/typescript-6/src/worker.mjs",
      "bytes": 32112,
      "sha256": "404f253a90e070c89fb9a2078e7d0c8144549d34559356cc80d5587e3679e868"
    },
    {
      "path": "apps/zryna-language-server/Cargo.toml",
      "bytes": 543,
      "sha256": "9c1d034e82cd41d2106318015e1a594ad0743e190eb73137e2eded449038d8fd"
    },
    {
      "path": "apps/zryna/Cargo.toml",
      "bytes": 876,
      "sha256": "e4f30f20c60e41073ea41c69d646a37150a6050bdaca436df67fd3ed685260fd"
    },
    {
      "path": "apps/zryna/src/cli_paths.rs",
      "bytes": 453,
      "sha256": "1fde83537611aa5dd44ccbafdbe78d6f3443f60359a9541a1dbdf81a4c3720ec"
    },
    {
      "path": "apps/zryna/src/support.rs",
      "bytes": 1076,
      "sha256": "2f92ad0cd8df21403fc23fe1d0597ef2103dfd97d8da8a98dec8a9d37d7c677f"
    },
    {
      "path": "apps/zryna/tests/cli.rs",
      "bytes": 75959,
      "sha256": "d98be49ba8f3e8ebbd5fdc017ab2d531ad5e5cd4a04ab33259a647339dcf2ba7"
    },
    {
      "path": "apps/zryna/tests/fixtures/browser_byte_boundary.rs",
      "bytes": 7461,
      "sha256": "ac29d9f1f7f8fca3137149ed7c10181440adc68aa2127c39866911d4e49314c1"
    },
    {
      "path": "apps/zryna/tests/fixtures/private_file.rs",
      "bytes": 3456,
      "sha256": "4a2ba46b11f6502974b63744945b00c16ee5281a08c46cc3f01209090aedf12d"
    },
    {
      "path": "apps/zryna/tests/support_contract.rs",
      "bytes": 3611,
      "sha256": "1ea07571fd1d67ab7cd71f72bd35dfea6c0fac243879f7e309a057e5ae2f13b4"
    },
    {
      "path": "apps/zryna/tests/wasi_command.rs",
      "bytes": 9182,
      "sha256": "e0791941e553d3eb1acdefdba087d916064fd56b0ef3d27167d6ce682f10ada5"
    },
    {
      "path": "apps/zryna/tests/wasi_command/byte_boundary.rs",
      "bytes": 4925,
      "sha256": "1a16ebff34a53d15111c05a6807d95711ce64957fc67f82784ebfc1a90bf9818"
    },
    {
      "path": "apps/zryna/tests/wasi_server.rs",
      "bytes": 8727,
      "sha256": "7ae5c6dce5598d6a519786ec7e866cb8e81e99bef3f9322d745a8a0678ec91f8"
    },
    {
      "path": "apps/zryna/tests/wasi_server/clock_support.rs",
      "bytes": 1819,
      "sha256": "2923a9f851324ca5f6d73de6d7df262b8363dc64718af83c7f37fccef21ec5f0"
    },
    {
      "path": "apps/zryna/tests/wasi_server/clock_tests.rs",
      "bytes": 13027,
      "sha256": "0879533552332a3b6fa002623b03a605fab81007f468f0d986af7ddff5e83c3d"
    },
    {
      "path": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
      "bytes": 2749,
      "sha256": "a34a3450a6d6e45fb3401e27e26f5d2a042d681b2df55da9416c2ac88424e15b"
    },
    {
      "path": "apps/zryna/tests/wasi_server/source_tests.rs",
      "bytes": 4544,
      "sha256": "6ac49eba794825ed58768687478b7a410446912264be7f8bbd3de34c9a7626ae"
    },
    {
      "path": "apps/zryna/tests/wasi_server/support.rs",
      "bytes": 7124,
      "sha256": "f3be9a159a50a75543f1e7c097a82d9aa88b15cbd2c3f7e856f30b938f4a38a4"
    },
    {
      "path": "crates/zryna-abi/Cargo.toml",
      "bytes": 326,
      "sha256": "49c0601cf2201b96725424b8f5a1336d50bce3a1ac5902b67106a19a61557001"
    },
    {
      "path": "crates/zryna-abi/src/lib.rs",
      "bytes": 49478,
      "sha256": "7ad4af8c9723ac280eb58311a1fbfc36a0f64aafa8412a00b723c36a227aa53e"
    },
    {
      "path": "crates/zryna-architecture/Cargo.toml",
      "bytes": 470,
      "sha256": "87ba545ab27c7c750e6c6e6611ae5ca4c5bc7546b9cab82bf61f4e9df6dcb0da"
    },
    {
      "path": "crates/zryna-backend-javascript/Cargo.toml",
      "bytes": 585,
      "sha256": "b6d6310639f098e1edc7135564d8f3d22606cbc5efe722b58137abd1b0006cd8"
    },
    {
      "path": "crates/zryna-backend-native/Cargo.toml",
      "bytes": 926,
      "sha256": "a73a430c78fca62e8851a6ab0217193cf6d07354c120ebd9baf373335d99a4e6"
    },
    {
      "path": "crates/zryna-backend-webassembly/Cargo.toml",
      "bytes": 779,
      "sha256": "2593f58139f078e912aaa41efb7f6cf608bb2d9dab7ea46ebdeeebd84ba50c90"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact.rs",
      "bytes": 9097,
      "sha256": "b058015eb74dd779fffd34d4b492cbd1b86ed687213054b0587c3c0ec829dabe"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/fingerprint.rs",
      "bytes": 3727,
      "sha256": "4e04be5c2f1d54a9a006dc015d9a3c8587010771df9cb0f121fc2ca4e9554a09"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/fingerprint/tests.rs",
      "bytes": 3019,
      "sha256": "d29884b56aa6bf8fb8983dc0329897363db894bb034689824e709a265a6485d3"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs",
      "bytes": 4257,
      "sha256": "1772e470e51a2f6c7042ae3ef3d2666a7fe2428f98379f5d1a67e2eb87d739dc"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
      "bytes": 9550,
      "sha256": "430f947bea1e9c26b35a7152433142bffd53b6bf8a5628c6484321a1527d83b3"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit.rs",
      "bytes": 10946,
      "sha256": "1efe2b62287908ae653040f1b0edc78aa0d95a913354b9af22740202c1d41f2b"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/graph_budget.rs",
      "bytes": 3064,
      "sha256": "ee87aeee3a5a091432f988a9c23b53dd65ed4b05c7c4ad5e6c08cb65a1c41782"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests.rs",
      "bytes": 5055,
      "sha256": "184c567e57cea0d99ef9dffe7a91a742c69b1566cd712e890af9b92107e0be26"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/binding.rs",
      "bytes": 3434,
      "sha256": "81087547307a2b01f8321994c8bc592f4d8142cd4c0aef79d0eaaa22fbca32f3"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
      "bytes": 4463,
      "sha256": "45581c48d91c4859c3a41d0255111d7e3098b2a572aea2777ccbad066302a425"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/graph.rs",
      "bytes": 6265,
      "sha256": "9e2c536dad9957ed7791fedafebb7035dfe7d32fd207357d3cf40ea6637c26bd"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/process.rs",
      "bytes": 1678,
      "sha256": "47228cbf1c5d4eb7352e98600752100cf179cab88a0c4953ef8688f6a2c4ff92"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/scanner.rs",
      "bytes": 3209,
      "sha256": "1c177e5b52c614ec614ad1804d126ffd47567bf0f6540b1fb1119dd04fc0276b"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/types.rs",
      "bytes": 5466,
      "sha256": "c484dfb94c19299e6ead3ff0ea6dbfd741be7fa0fd99a9965a3a3c0309c600e6"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_tests.rs",
      "bytes": 11075,
      "sha256": "bc3180ae49cd2eff4dee74c84f2effcae1f2ec8993d23c355d6c62a1c2d6dc74"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/component_tests/interface.rs",
      "bytes": 4146,
      "sha256": "ec56b920682ae55e297971f347657c42a3959b72647500d47ae0dd777a178174"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/environment_audit.rs",
      "bytes": 1409,
      "sha256": "39b358cc555d2bb5406fdee65826dd610f580c8e22c18119e372a9739cfbd23f"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/environment_audit/normalize.rs",
      "bytes": 2273,
      "sha256": "548b63561abfefa219906a22deefc5cf28597fe6a2bed8b51418e9aeca446773"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/environment_audit/reference.rs",
      "bytes": 3712,
      "sha256": "5c63d75eeb1c7cfac002fad505fad89256e1110876ee4b3adb5a00dea0847a71"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/environment_audit/reference/bridge.txt",
      "bytes": 5376,
      "sha256": "fd9b61cb05f7f06bf48493bda7650dd4b39ef6c37c74596d5ca1e1da69722f59"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/environment_audit/reference/key-byte.txt",
      "bytes": 238,
      "sha256": "5e889a2620abdd627fbb39c482cd27a3e155a71fd654a39826cf8ce11b30f89a"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/environment_audit/tests.rs",
      "bytes": 7414,
      "sha256": "3d032468d9d6b5bbcae204eca41fe9ed3bb3dde56f21910ed170601061c1c4ca"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/environment_audit/tests/callsites.rs",
      "bytes": 3318,
      "sha256": "63afc75c63c4672a3a58bbd7fb09fd3ff1144403e9ab0361c0588aafad54d543"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/environment_audit/tests/mutations.rs",
      "bytes": 7280,
      "sha256": "5dda0ba40c23242886347bc05a3ddf9ba03667e69cd01b7a3094840c4cb4c45a"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/interface_audit.rs",
      "bytes": 2543,
      "sha256": "dc2341b948b9281a0952c880b9e31e5f7f8442b8eb28a3de7cb017ae86f0652f"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit.rs",
      "bytes": 6249,
      "sha256": "4c55011179f878399a52190f3c82bc98dbe652ed31de8c41493884e554b11818"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol.rs",
      "bytes": 7883,
      "sha256": "214f59b03814fe0dad39e072c40e1b446878fdb3d6291f013207fff29048a651"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/nested_calls.rs",
      "bytes": 7816,
      "sha256": "15b19777605701c61c3cfc99ab84efd39aa8217a2adedac35411f63a1267d77b"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/nested_calls/clone_suffix.rs",
      "bytes": 5089,
      "sha256": "4c01272642a960a6d2e1c7aafc75cbbd7060155ebf5a57687984e7cd0b4fc5cd"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/nested_calls/drop_body.rs",
      "bytes": 4662,
      "sha256": "7d8ed6cf694fc3a8f9f18765096634cdec577329d40978ca360fac442fdd0c15"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/steps.rs",
      "bytes": 6194,
      "sha256": "c34bee6b183f79eae2d88ded4c7af32fa4b7e848670a8548b5aa34ebb9cece6d"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/tests.rs",
      "bytes": 5701,
      "sha256": "a815b5c69fc368c4be454b9c34d374c4e787fd0b5ca2e2a7623c0eb239dd3dbd"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/tests/bindings.rs",
      "bytes": 4918,
      "sha256": "6644ff5f876f495ba589de4a47fe28f457b0c5131339830c01b628f9085b0fce"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/tests/compatibility.rs",
      "bytes": 2644,
      "sha256": "c450760e79217735d131b8cbd05fb06d6f74d0bed7edcf0208cb16c1b097f937"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/tests/helper_roles.rs",
      "bytes": 2752,
      "sha256": "4e213494d385566872de5b8185540bfa0e642ea48913e8efa858458f413888fa"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/tests/mutations.rs",
      "bytes": 5334,
      "sha256": "bcba2aad676eb7659e09703fd5de7247c5380842c29edea65442ea2b86b0b925"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/clone_protocol/tests/nested_types.rs",
      "bytes": 6090,
      "sha256": "67d8879b3c5e77c1f6f709565add188b2d733ab31f6cde9945252be4b6a45054"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_audit/declarations.rs",
      "bytes": 7482,
      "sha256": "1aab65d5cfc68fbb17497959160ead9efe7c60512afad238ecb8808fc793d9a7"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_tests.rs",
      "bytes": 11573,
      "sha256": "90069d08f3d33d96ce03c30946f31f9901e66f0a16c4ab0be758376fadeff976"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_tests/cleanup.rs",
      "bytes": 7700,
      "sha256": "c9a187f4be7b9c19316d876f2433efdcfe4f8a0d2fdbeced4cb93be7e9ba782b"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_tests/frontier.rs",
      "bytes": 7187,
      "sha256": "c0a5f6e36f9a987abd88a3d45b9280f25e08fbec724ab8e2b4eb454c46eb3e45"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/language_tests/vector_growth.rs",
      "bytes": 624,
      "sha256": "ec25a06789045d7ac762454cf0bc0f53034aaf7fdf7e782d67b8f375070068c8"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/run_audit.rs",
      "bytes": 4958,
      "sha256": "03532a7d8151a9ea047c3411291153ab7f0050fe6d242431932532ce11d528bf"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/shell.rs",
      "bytes": 2886,
      "sha256": "9d640bce3e7ce5358cc3a6eab0398e7167fece40cd4b2b34141c2d1bff67ee28"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage.rs",
      "bytes": 4367,
      "sha256": "ce92e17da49c14a48f9b2f8945edb01669efb46342e335bdabb70caaf7da406c"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit.rs",
      "bytes": 8696,
      "sha256": "4437822710b878f473166fcdfae5e8fb5f099bb421b1c0dc82c6009eecb88e21"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape.rs",
      "bytes": 3472,
      "sha256": "684a96dd406d4acd493f5146713d242e43676b0fd96b9440c5150e7345ce5cf9"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/allocate.txt",
      "bytes": 1204,
      "sha256": "970b7dac84a24d04bcfe01e611d6b79b638c1da1a02c12fc812c19eddec21979"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/copy.txt",
      "bytes": 1248,
      "sha256": "731846a41d3361776152902b3f0d67cc50d6b5b013f9373e8043c09c31c1b821"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/drain.txt",
      "bytes": 435,
      "sha256": "28823d5249dee0a3c9f596eec08100db597c7ecadb624328e3f29e052ff4e84a"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/find.txt",
      "bytes": 430,
      "sha256": "10470396bcdfac303c3f15fdc7c0c21e4f707bcb40ceefddccfbb75b65276611"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/free.txt",
      "bytes": 337,
      "sha256": "cc81446c0c5b91c7ba97e1c4eba264c618ef8b50898281b784ce0e6bfc3245c6"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/language-allocate.txt",
      "bytes": 555,
      "sha256": "2d0a2b2d05c770ca5b8d00b89e7e5c6d6cdff573dc19e363062a286c4b3a0946"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/realloc.txt",
      "bytes": 831,
      "sha256": "50031041077d03535000626f2550b7d3822aaa1ed9cd337f136adff83a4b9cb8"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/state.txt",
      "bytes": 332,
      "sha256": "6694a9cfb2835cfef1db4755a685b23585512e41efa7637b12081ba583e03aa6"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/audit/shape/validate.txt",
      "bytes": 763,
      "sha256": "e3637a1faf3376a72579ea970eee8372e73fa2b189b9295f2b2c1ae616c0698a"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/canonical.rs",
      "bytes": 4645,
      "sha256": "43505bea1b85513da33c812db6851c4fbaf28c8c90421c7c45f1877f188fd3ad"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/language.rs",
      "bytes": 3066,
      "sha256": "fe2a2f837313b789611a306918cb70c72e85ffb12709c2cca915163b7506a0bc"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/reallocation.rs",
      "bytes": 3255,
      "sha256": "809c756c28a8e9a1bc8c65271eec3eb53349fe58669db7b3d847ccd882a1f24d"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests.rs",
      "bytes": 8364,
      "sha256": "0e8c5c29f073ce57f2f8bc60ba898ce456b1a101b516feaed990b3ed3dd20be9"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/mutations.rs",
      "bytes": 4120,
      "sha256": "f73faa705c86986ab8689b9799593b7e3fc9e69aa510b6c070184d5a7664d7e5"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/mutations/boundaries.rs",
      "bytes": 4266,
      "sha256": "2f8da252d6733b39ce077be395a0543661e6f181385cc45037e00476d0fd125e"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
      "bytes": 2850,
      "sha256": "7997369de52519d73d3c8b165e3f94e20fa0470d3263564047b94144ef4b9093"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_command/tests/bytes.rs",
      "bytes": 2084,
      "sha256": "c2c5ffcf3744696e037f6fde127800c5587a3fce0fe4ffd49e3b49422a4805f3"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_server/audit.rs",
      "bytes": 6840,
      "sha256": "cabddc2b28d0a285315ce509ad383a317e762d41e132176cb409973a27d3da13"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_server/bridge.rs",
      "bytes": 5922,
      "sha256": "30307f6d0676ef3a8ca3d9eb6cdd7952f12d6bdfceabcd3c1780cac096e34ec6"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_server/memory.rs",
      "bytes": 4185,
      "sha256": "3646c37a17ef0b98e90d4fb4387f7f8b5271b806486b3fc702a4a30fc3f5c414"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_server/mod.rs",
      "bytes": 5037,
      "sha256": "29ef601b33bf6474132238d492ee5c59bba99fb107fbd8f5dbeedf6cc7936c9f"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_server/shell.rs",
      "bytes": 9567,
      "sha256": "33873303de435e71e7e6f7cbff7f75135ca9643afa10c1db44a9644e53b47dcc"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
      "bytes": 7902,
      "sha256": "677749dc6720848801a24095822ad5278c79920f244b5e25c6f74d3ca7098e32"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs",
      "bytes": 5575,
      "sha256": "203a77f9114eeb834171fc913b39f7aa9bf9761d59de21af5dea6cfc70dde0e2"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/component_server/world.rs",
      "bytes": 3189,
      "sha256": "776d400342456e18e6d2bd7c12fda832cecd89b63ba8775409b940ade7b7aa27"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/lib.rs",
      "bytes": 62611,
      "sha256": "fbaf2ade46e72cfb615e3a22052914d4fae9ca2f12dc0de8ec5ee7f82257bd82"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/scalar_audit.rs",
      "bytes": 3443,
      "sha256": "4104e38d23b1712421b4b96731434a7bcc0c2e5c57f4dd19a5ab1b620fcc1bbc"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/scalar_component/audit.rs",
      "bytes": 10752,
      "sha256": "6f0190f480bc2535c2c3161746e76a5f2b29fa60dff74377344c3a6cd84d0a2e"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/scalar_component/mod.rs",
      "bytes": 9714,
      "sha256": "6c821dac763d5d3d60860028a83bfb2ae21d3a22cb0124e9e7c14d94649a5f88"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
      "bytes": 11738,
      "sha256": "68c8e5f974e73c5f41756dc8a5d6da092e95fa5aa364d86d6fa3afafe08f0368"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/scalar_component/tests/byte_boundary.rs",
      "bytes": 6195,
      "sha256": "79df62ad60847154003f3ee4a31a347f822e5db6854be58d5796bbbf26daae19"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/wit_world_audit.rs",
      "bytes": 12010,
      "sha256": "bcac3a74c466b4c8bf65bf140efb898dbf0dcccff99f270191bd18056e15b3ee"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/wit_world_audit/browser.rs",
      "bytes": 2224,
      "sha256": "b88279e8f9979e5349bd2a3390e36582208a3131b999f2f9020f39239da063c8"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/wit_world_audit/command.rs",
      "bytes": 1186,
      "sha256": "9ff1b4387c014f86c3fc39da7eb202ff38ecba71fe8e05f01f519e5f563f1920"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/wit_world_audit/pins.rs",
      "bytes": 13524,
      "sha256": "8c2381456b60c23ab77617e136551a46dd96719806197340ed78ed74a75e5851"
    },
    {
      "path": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
      "bytes": 5648,
      "sha256": "6f270c8706c00009d3f5f20a6c1be901699071f38815ac68a0d040ce48940814"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/.gitattributes",
      "bytes": 18,
      "sha256": "171d10098791686f1968eec0fc11a42f40a34e58e8f9084e7e0c0c2e7862ab44"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/command.wit",
      "bytes": 154,
      "sha256": "31501d47bbb3b114c4fcb5218d42e69a4583d298384518c6adda1f2f97442153"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/environment.wit",
      "bytes": 811,
      "sha256": "5e58cd47b162e9f004bc9224a29c2ec3cf9e1a3d4c910154ce53742f828af91f"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/exit.wit",
      "bytes": 638,
      "sha256": "05453b1a965cdbd08fd1f9fa5df25f98746a68245bf6ff484ff661ed1e192c15"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/imports.wit",
      "bytes": 865,
      "sha256": "2be596bf68d2a4babe8ab388b390af96b4fcd7e4436eea599bd9c4d5705c22f7"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/run.wit",
      "bytes": 116,
      "sha256": "a7df5376da8230e0509fffbb8355641af0765cc5cff8064bbe58fef1cbbe63c1"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/stdio.wit",
      "bytes": 547,
      "sha256": "87e289b61c200a8cfc615767ef74920da51fdd75d7fac413dba67fff76e0f95f"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/cli/terminal.wit",
      "bytes": 2021,
      "sha256": "fc87cd0bc3f76ff7181d3666617a38ee10239f8e0344db2a52cb79f46c6b2d31"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/clocks/monotonic-clock.wit",
      "bytes": 1954,
      "sha256": "3453cd071d43c1045d417adea2bec82d66580639b71c09cad641f8a5753da947"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/clocks/timezone.wit",
      "bytes": 2220,
      "sha256": "657bb268af30308f7b34f28e0b2bb83f69655789158813dd9926e10b57ebba43"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/clocks/wall-clock.wit",
      "bytes": 1783,
      "sha256": "59a16c2aa86e89bb0a15eeb81b4205ea607cc7e97bebffdf62622b369da9b3d5"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/clocks/world.wit",
      "bytes": 240,
      "sha256": "b998c5cba8ef680051e83fe12bdc9d579775efe05bc93c27202c125ecc82e76d"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/filesystem/preopens.wit",
      "bytes": 295,
      "sha256": "d67e1e166fdcbd0336784b774997d9af777b3843bca21cc651ba88f524f716e1"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/filesystem/types.wit",
      "bytes": 28872,
      "sha256": "ff4b5f880ef38afb462fa37f00f75a75b476feb5dc0fc2da8a3633e2ae592ee6"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/filesystem/world.wit",
      "bytes": 170,
      "sha256": "3b126a50383dc58c7b85bfbde38836179a3df2acc9cddd3130d8dd112b77b2ae"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/http/handler.wit",
      "bytes": 1959,
      "sha256": "1f66174b4058b102367e69ce8aa657b3e9dbc767f6ade960108faaa1731f0945"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/http/proxy.wit",
      "bytes": 1938,
      "sha256": "8342e207c6ddcd642babce6e744a940c6521892e1370562c7918eb2116de379c"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/http/types.wit",
      "bytes": 27964,
      "sha256": "d67973c394247f8ed912368ef81a47408254163e26f11de2d1440ba3f4b66eec"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/io/error.wit",
      "bytes": 1507,
      "sha256": "a4eb0f5805db705c3c9754d5fc1396c5d75ada2628a4c0ac599310e194e38a9b"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/io/poll.wit",
      "bytes": 1796,
      "sha256": "385347a39e16284d6c78e9776014be4a5e90aeee74c8de4eabbc6ea5e3b1b2c5"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/io/streams.wit",
      "bytes": 11870,
      "sha256": "8b667deda5c89045ff30cea91693cc5cf1510cc2fdd96a20053459c6bf27e7e9"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/io/world.wit",
      "bytes": 161,
      "sha256": "366f9d1a2b5b11cbdbb7a15155b25aa6999c1d1da30e4ba58047d11526014be3"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/random/insecure-seed.wit",
      "bytes": 1157,
      "sha256": "ef468057768649dd74b23baf659626fd80fc7b653cdb2347437503ffa03fd00a"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/random/insecure.wit",
      "bytes": 944,
      "sha256": "5c3f5c1e2250edc4db2685a983880822304d0a0847c88e068f2a4a04209f7be1"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/random/random.wit",
      "bytes": 1230,
      "sha256": "c9553bdc37f0306e7d6908de669330675d528e79762954ed1539f32dbe9f61c4"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/random/world.wit",
      "bytes": 223,
      "sha256": "fc32132c0aee89c0a846dfa5358dfe2143350acca1a470052270bf427815110d"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/instance-network.wit",
      "bytes": 301,
      "sha256": "1e4a5d97df44421503a169e1d514eca2cf160f545c44d320ce8a82f726a40cf8"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/ip-name-lookup.wit",
      "bytes": 2797,
      "sha256": "9bbdac3007844acc4a292736be3fe01e39df2ea968f8d1e4568fec20869752f0"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/network.wit",
      "bytes": 5180,
      "sha256": "7813868ee56552a434319f5fbf141d55cc1d37272b2976debcc75a50b9cb52e6"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/tcp-create-socket.wit",
      "bytes": 1594,
      "sha256": "96a6a1a93b859127ee60273f0a77cda8d60930f7952174fb22469293b5da6a3a"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/tcp.wit",
      "bytes": 21317,
      "sha256": "1e9d17a8e0770abbc2ddead9dcc66793393bafe549f4b7effe73d8ad27272295"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/udp-create-socket.wit",
      "bytes": 1584,
      "sha256": "c52b9bf91ef4e30e7ec08ad1fc9e77ef005a0a87648c5d2e416dd6e35369f08c"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/udp.wit",
      "bytes": 15936,
      "sha256": "83d0808198c95f261897676a54c93f1914061190d7a08c99847a6e3ddbeabada"
    },
    {
      "path": "crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/sockets/world.wit",
      "bytes": 436,
      "sha256": "abf77820f4208da60dab68dc9be63c510eb0e5083014963dc478326798cd771b"
    },
    {
      "path": "crates/zryna-diagnostics/Cargo.toml",
      "bytes": 307,
      "sha256": "b95def49322aa122afb5b41f6f1a32bfa340d0105d1042408f4fd2458ace5123"
    },
    {
      "path": "crates/zryna-driver/Cargo.toml",
      "bytes": 2131,
      "sha256": "f210a6f343422e5d6edf44d269641d1246abc6f1e3a2efc35ce27198a45073eb"
    },
    {
      "path": "crates/zryna-driver/src/browser_component.rs",
      "bytes": 3701,
      "sha256": "90eb6d1d49c6950f313b469d8e5194ec9fde4abe80385dbb4f8a70717482cbcb"
    },
    {
      "path": "crates/zryna-driver/src/browser_component/loader.js",
      "bytes": 4022,
      "sha256": "c113ae5edd2f1b732c37023bcaa19b0c62cb988aa2939ad373e69008466f1038"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/envelope.rs",
      "bytes": 2054,
      "sha256": "221b962d3729c155c6a776487efc061accd56c4841b17b560de95ce3f5fe2730"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/host.rs",
      "bytes": 5883,
      "sha256": "a82a329a6ce336875579a27cf463597addf87c875c8daa38d308c18b0619fac0"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/manifest.rs",
      "bytes": 7805,
      "sha256": "4231021da0f0bd734312e16506451e712b25c2bcdcf2f1b20e6051f8dbdef495"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/manifest/model.rs",
      "bytes": 6462,
      "sha256": "9cf0bc99435ed9fea41684b7e8e2a457fcba05b3acc266f2cce2f6a606f02f45"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/manifest/observation.rs",
      "bytes": 4933,
      "sha256": "72cce52e3a6dffa7489abeec94d3823811d9d7b836817ae127f8dfcfba894ba7"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/manifest/tests.rs",
      "bytes": 8839,
      "sha256": "e9a1af783d6cd206a95f8680f57d34252040591e060f4ca4f26ebca095875f1e"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
      "bytes": 10469,
      "sha256": "bb439b44b97f8acf479a9dba814233b8b3362e4354baa7445519e067f92e8a9e"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/manifest/validation.rs",
      "bytes": 7175,
      "sha256": "fec1ee687cb3ee2c68b70cef9d51355c2b40f27c4930c8322aad05805be15402"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/manifest/validation/fingerprint.rs",
      "bytes": 3999,
      "sha256": "085f0f133f3921673a17ee60161c342d9e619bc2c306bbd9197047effbd79628"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/manifest/wire.rs",
      "bytes": 2702,
      "sha256": "30e7dfd59d4799ee0f1b0d10a58d126a53cc02f8efdcd1d478d1ebdd71ee124b"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/mod.rs",
      "bytes": 5814,
      "sha256": "7ecf2f63009bc3ca3979973c90c46438b9d7cc74ed6c17ae0d4d92e9ca259c0d"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/policy.rs",
      "bytes": 1683,
      "sha256": "da714f313ca1f937369f6bed3182f41f0d0d27ef1768d4c02dd2f10546c22ef1"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/preparation.rs",
      "bytes": 3876,
      "sha256": "c83d927976e16ff1c8bb796c0fccc6a5ce51cfb7d69934628060e0dbb21d80cc"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/record.rs",
      "bytes": 3369,
      "sha256": "8fa1cb6978eab08fa016c30994edd0b973814e5f4a89cc5e656093043b74acc6"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/session.rs",
      "bytes": 5889,
      "sha256": "5dfe68f2cd275cbd739fac23c3e22d63bf5034b3bdad88e2272e6820eba7082a"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/session/tests.rs",
      "bytes": 3512,
      "sha256": "eee99e0d024df546e0018b03f8ca687d09c2baee9aa630302824139fa51b21f4"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests.rs",
      "bytes": 3499,
      "sha256": "c3302a531fd81e61936b52c26f2e2571bdadfa1b331bc3de3df8ffdfd0c0e7b2"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs",
      "bytes": 2383,
      "sha256": "604fdd1ff098a2b2726aff2f52a69f9402ce73f4d9da2dd44d5432f1f1c8bc34"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/consume.rs",
      "bytes": 943,
      "sha256": "af5b48e0c0df6647cb5b22365d2b18c85eb73d54d0bc2f9b140bf68225473ac8"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
      "bytes": 1706,
      "sha256": "d6696a454afa19af998cd81027681b1c2e62e9be190e250b162115ff3e66a5c9"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
      "bytes": 2952,
      "sha256": "c322606997329d2b4f832e23c37f16567e66d3335fa3c0e89e8663a848e98a7e"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes/component.rs",
      "bytes": 2851,
      "sha256": "156db5e4e03b3e10e23fb12dc048034fd3b546a6db77148a27c3d92a92460282"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes/component/core.rs",
      "bytes": 2662,
      "sha256": "5661caa1105d4a46d74921de2166206cc43197fedb00283df59882b98d7fdb3d"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes/component/interfaces.rs",
      "bytes": 2745,
      "sha256": "04ee76f2ebb32dfd26c9fdaff81749495185a7b9543a3daa074166c807290620"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
      "bytes": 2610,
      "sha256": "463f9ef8bbbf35aab83c0360217d384e8cbfd51f7fb137e4f8c151fdb8cc9873"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs",
      "bytes": 7109,
      "sha256": "849124fc1c62ec53da6297ef74ba3f136c1f12639062dec0a29a6484bf1b4b5f"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs",
      "bytes": 477,
      "sha256": "0cc73531b82032736b23a3c1b2ba91ff9e4f57261e9b05c07d2353b4cbcd3bc5"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/host.rs",
      "bytes": 5713,
      "sha256": "77152cc17793ad4079d7899b4e8e1f625f66b4bf5402c59526f537e6c6f0750d"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
      "bytes": 599,
      "sha256": "4873b2d0a9c98a96a903eb8d393aa3af1b19e7e11542780d855b9af51bd9dd3d"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
      "bytes": 3114,
      "sha256": "2b5ef0ff7d96f86c0f0a1569019b39db00b8328263136e3b5b9e1f50f33fd3c1"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs",
      "bytes": 4333,
      "sha256": "451d9fa1b6110ac4eff97f2c0a91fbacc42f76d2551e5ed0972569e64c44c794"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/private_input.rs",
      "bytes": 4193,
      "sha256": "be7e7320ac31b8c1fc97240eb45da6d1e5cc4ab716ec4aba2c607c9fbfc9916e"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs",
      "bytes": 1936,
      "sha256": "77342761b9ec5985edc8d2c3d64fb6fa64423bdd5699e236ca2d481605ce0276"
    },
    {
      "path": "crates/zryna-driver/src/command_h1_workspace.rs",
      "bytes": 9128,
      "sha256": "57f372de3bfff3993f1b3e63c629ee9ea8c09ac9f0d99ab5a0982a5b6aa785c3"
    },
    {
      "path": "crates/zryna-driver/src/command_request.rs",
      "bytes": 2655,
      "sha256": "f51cdd4a9d48c2b28b854511130cc7edd5c82111db2455dc63f56dbf77f44b0b"
    },
    {
      "path": "crates/zryna-driver/src/command_request/capture.rs",
      "bytes": 5060,
      "sha256": "475afcf79f5bab48f2f739be3e4c98a682c2805f4924ca081062db45320a6697"
    },
    {
      "path": "crates/zryna-driver/src/command_request/capture/binding.rs",
      "bytes": 4456,
      "sha256": "5e097bfa6c06e05f3f9cf23b4623b875ddf7b8ebd040d11568608224d60cc543"
    },
    {
      "path": "crates/zryna-driver/src/command_request/capture/platform.rs",
      "bytes": 6120,
      "sha256": "28e5bc1dea971b89b32c5731d2e2cd4cca47a3e523501498fada0dc7cf4481c9"
    },
    {
      "path": "crates/zryna-driver/src/command_request/capture/tests.rs",
      "bytes": 2009,
      "sha256": "38b9f5d1b9ca71bc6072ac7facf8ecb04afb6e40951d6057130a9dac09c20b11"
    },
    {
      "path": "crates/zryna-driver/src/command_request/capture/tests/unix.rs",
      "bytes": 10480,
      "sha256": "4100ea0982c457f9a67071c9680ba96e76462c80da6e7196a9e7c540c4ab8152"
    },
    {
      "path": "crates/zryna-driver/src/command_request/capture/tests/windows.rs",
      "bytes": 9651,
      "sha256": "fe5c0b559f89f322e845afe003bc6cf80eef2af2408e4b9de1d8a9553cd3ab5b"
    },
    {
      "path": "crates/zryna-driver/src/command_request/tests.rs",
      "bytes": 5698,
      "sha256": "f5be2ee419d33eef3d6b73339f0f4cf6c0217f4e4d2db11171241e5793af0277"
    },
    {
      "path": "crates/zryna-driver/src/command_request/tests/bounds.rs",
      "bytes": 2548,
      "sha256": "e69dc90592892e5f9f915e8d0387182a4a44989dfa00d08ffe927154d8288267"
    },
    {
      "path": "crates/zryna-driver/src/command_request/tests/rejection.rs",
      "bytes": 9266,
      "sha256": "e010b0c6951eb58fc3b399b5e24238ec81c9f5a91a02bbfd07dfd2643f69dc06"
    },
    {
      "path": "crates/zryna-driver/src/command_request/wire.rs",
      "bytes": 3289,
      "sha256": "ac0a2c474daf9f5ce34ed4adfa7fdf40177f8c7de4309373122290bd27ca9588"
    },
    {
      "path": "crates/zryna-driver/src/lib.rs",
      "bytes": 11559,
      "sha256": "72328945432447b351708a9e62e6bc3ac025734ca7b70f246b322cac873be92e"
    },
    {
      "path": "crates/zryna-driver/src/native.rs",
      "bytes": 100378,
      "sha256": "50f165a26575a02f7e8b96f0c281d5e81647ba3d4e7823fff6c44726a9dd57f4"
    },
    {
      "path": "crates/zryna-driver/src/runtime_boundary_tests/fuel_meter.rs",
      "bytes": 3268,
      "sha256": "7ec201cdf3ece95198cffe1653ee69e2498aedb49e968d9206b213377149c74e"
    },
    {
      "path": "crates/zryna-driver/src/scalar_adapter_interface/consumer.rs",
      "bytes": 5424,
      "sha256": "6cd7689b66cbca897068f461bc4a5243628d023b1ef86a099863cb0667f796fd"
    },
    {
      "path": "crates/zryna-driver/src/scalar_adapter_interface/scalar_call.js",
      "bytes": 1072,
      "sha256": "93b2a9a8cca08d24adc0ca1c98cb216210320c66c91d64082bdbd60eb577624e"
    },
    {
      "path": "crates/zryna-driver/src/scalar_adapter_interface/tests.rs",
      "bytes": 10634,
      "sha256": "1f4d7c17ce7043c27c27ce104bd41dcd5c9ba18a847c8cd1809fe31d893c4651"
    },
    {
      "path": "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs",
      "bytes": 12689,
      "sha256": "327d61cae66452eb3db6e6bad88d5a04733de5d4b957fc3c3e4b29127f08f27a"
    },
    {
      "path": "crates/zryna-driver/src/server_lifecycle/deadline_tests.rs",
      "bytes": 4133,
      "sha256": "de68888a1ecbbd005d55460610c3f9a718be91fc5cd2c07f28d19b1ae73905a3"
    },
    {
      "path": "crates/zryna-driver/src/server_lifecycle/harness.rs",
      "bytes": 16370,
      "sha256": "80105d0930ebf93f6ed55829b9659abe25a9e5ffe218d85a729b8688d0993e3a"
    },
    {
      "path": "crates/zryna-driver/src/server_lifecycle/limits.rs",
      "bytes": 3401,
      "sha256": "8aab3ca6f39fe7c914eedf1e1033d70598fd35fd05ad91d51412c97fb471d126"
    },
    {
      "path": "crates/zryna-driver/src/server_lifecycle/mod.rs",
      "bytes": 3801,
      "sha256": "82707340a4296a2e6e2864993aa1c22648e8aa6b86211ee7fddfde76724f5113"
    },
    {
      "path": "crates/zryna-driver/src/server_lifecycle/registry.rs",
      "bytes": 5350,
      "sha256": "0cc757d1df27eaf406bf29ed2cfd228efdf467d1ace8f079a02579a49bd04cbd"
    },
    {
      "path": "crates/zryna-driver/src/server_lifecycle/request.rs",
      "bytes": 6561,
      "sha256": "7a16df386623ddb22f6d366900d9e070602c2399c290aaad3ca925bc97c61153"
    },
    {
      "path": "crates/zryna-driver/src/server_lifecycle/termination_tests.rs",
      "bytes": 7492,
      "sha256": "52f08293ad418fe0318b355676d112db51c6e513e7b1c4a9bf75e85695599182"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/envelope.rs",
      "bytes": 2897,
      "sha256": "b4d8fa4a7bea0466c66f7364030efe8a208830edbdff9255c6615e86099a27d4"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/execution.rs",
      "bytes": 10050,
      "sha256": "b9b653b138e91a399e46e5e005f2e65a50ec131537ff9cc20185c1c18099e260"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/grants.rs",
      "bytes": 2758,
      "sha256": "7681a1ac33b5c4f1c1d5e2e2e25609733c61677a9d189f6ae3edb45d8b983d55"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/harness.rs",
      "bytes": 535,
      "sha256": "b75078f0833bf2441793492ef678675deffafee35a7754b486e09f901d18b767"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/host.rs",
      "bytes": 4044,
      "sha256": "0c8990da9c09b2fd1a68b952aec9586abd4ac848a04e947bcf0a26f69629efe9"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/linker.rs",
      "bytes": 4079,
      "sha256": "b1d1fb10f40ac53a1884f0d0083bc60578cd005dc30b7ee5a11fc7ca6b4f7419"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/mod.rs",
      "bytes": 2425,
      "sha256": "41df0b1b4b999b7f9507c25764f1dcaa2173283a6664b81c1bd580f862a97bc5"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/observation.rs",
      "bytes": 1043,
      "sha256": "2919f62a0a1d817f6f396386a3d9b70650f0aa8fbeb518ef68224fcf600d664b"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/operations.rs",
      "bytes": 5850,
      "sha256": "2233fe472985ee296045c42a206749ceb846d11835b88f0b1149b4f47208cfb7"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/prepared.rs",
      "bytes": 5861,
      "sha256": "79b074d7892f98c9e5e4fd9f5058bd49ed8717987b12855a97e97d1ad7131bd8"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/resources.rs",
      "bytes": 3263,
      "sha256": "904a5c609e5dea8ef34b33e36ef5448e9c15f8185185bd723712851f4b85b444"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/startup.rs",
      "bytes": 799,
      "sha256": "410dea8adb99b2c35ac6600be9c0f19a9ee371f108d09e7a6c078c5299fdf37a"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/test_support.rs",
      "bytes": 1020,
      "sha256": "598fc96a85062ebac6047ad8131ad3161be73f36a989b914a21efe9f6557979a"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/admission.rs",
      "bytes": 6934,
      "sha256": "9b20cc8c60a1532141eac65035e5a063a5251d68da33ae8d4f128248e71c58fb"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
      "bytes": 1938,
      "sha256": "e424967aa7f1657757f78b97e8f316d73138954d9f3a223a1e92337dc5eb4055"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs",
      "bytes": 3044,
      "sha256": "72b269a6c205c772b5126134036db03179a1cce68ea0f18ea747e731acd29a04"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/epoch.rs",
      "bytes": 1836,
      "sha256": "a401ee2ec86676c5e16a8150c647594a1c37e9fb2ee766781dc6fafd169ee3b1"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/execution.rs",
      "bytes": 6726,
      "sha256": "0021c5e807429ca56d1144cec9dfc475416ed403459f17f1a6c79b208adb6139"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
      "bytes": 1340,
      "sha256": "8a540cb6f0a75ea1b357a90e605161dc7637ce58c006447b92179d559bc8cfe6"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
      "bytes": 1050,
      "sha256": "cbe801728e09a94753e4cdba6af28d91ddd2f4b2e63622c72c3d2a4e53b9be8c"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/mod.rs",
      "bytes": 3258,
      "sha256": "61d604d2d9a488c56794a9da8cebc860fd908ebb0de1f02b3e7eadbcdcd5d799"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs",
      "bytes": 3103,
      "sha256": "0f61b9b88fb48a86e85949a223574293d24cd6c30228b51213f8a621bcfe1611"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/probes.rs",
      "bytes": 2671,
      "sha256": "03c54ef98a235ac56072a723310ddbb5717f38167bbc705f79d71893aa0d985f"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
      "bytes": 2497,
      "sha256": "2be352dec295c3a4ebc20f0871f23ac5784666e4e7e0d6bbc8cf1ae657a5ede7"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/retirement.rs",
      "bytes": 8131,
      "sha256": "87b620f152a18118d61946781274f8723344e400f22a763f90ef6df7bac46c59"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
      "bytes": 1610,
      "sha256": "a6353c88edb251314c2c019bd6cd609438dd9ca10af2ff46a6b61900511c83e2"
    },
    {
      "path": "crates/zryna-driver/src/server_runtime/tests/startup.rs",
      "bytes": 3219,
      "sha256": "a909d066788b0c8a1d11d95c5e4c4f7151e3b644a3d2d98fcf7a90993be5ebb2"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
      "bytes": 7468,
      "sha256": "7352a2fcc012da3b2c96ccc49225affdb4cfd6ad0370d06de9e41e22008580b1"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/cli.rs",
      "bytes": 1958,
      "sha256": "e24b1a097791c337b37cd0733d10cbed1f1829e4175fcb152fb388d128d2de82"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/config.rs",
      "bytes": 3068,
      "sha256": "a8491e9dd1b54426baf6275f3510dead3844c6840c9a591eb8495affa4416aa2"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/corpus/config.json",
      "bytes": 110,
      "sha256": "2cf2a9b65423e1111725ae2a93ce6ae2163f94de02fe111c7a87042e3f3e61a5"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/corpus/status.zry",
      "bytes": 52,
      "sha256": "473d88e4bcb02639ece1020ed6f9594e8b90275371c1d3acd3ec56cee74eeb9c"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/framing.rs",
      "bytes": 4340,
      "sha256": "14d359cb2cc02bfe25c8d08945294f04c18e1bc865e341b0f7ecfbec297b09d3"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/io.rs",
      "bytes": 5141,
      "sha256": "b9e3fbacea56f813e3b194410cb48c4d21bf1ae9e27e8b5fadafb13d01d3c5e0"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/lifecycle_publication_tests.rs",
      "bytes": 1858,
      "sha256": "bfea9182d8d3e5f7aa0f2309f024469a973c05df7dab066d1fa3702dc88fffe0"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/listener.rs",
      "bytes": 6488,
      "sha256": "4ad1e58e79fb1dc45c792069a11dddbf08ad1f132651da9fec04c102ad3420d5"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/listener_identity.rs",
      "bytes": 10688,
      "sha256": "aed1e2a0c065748e5d91f3de95b2995f6ac0916ff7c44d5898019fdf8cd0cda7"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/mod.rs",
      "bytes": 1144,
      "sha256": "30ad13fd70c9873af81f64cdc4828a79fdc7826f12aa915165ecfd22813d3c55"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/negative_tests.rs",
      "bytes": 9608,
      "sha256": "d00dbc162c745a56907156acf305e498f4a7e70ff9fc740f572df3c9ac529b5a"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/process_fixture.rs",
      "bytes": 3566,
      "sha256": "3b15e7e6b32fd34a10149a578b10e49c2f917112ffe88efca70d96382612b795"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/process_tests.rs",
      "bytes": 10047,
      "sha256": "d9ec370c44fdd3232016284ef3cd3cfeb1d88faee271c8d6597c525dc7996032"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/publication_tests.rs",
      "bytes": 3728,
      "sha256": "00e6019fb16f95767965469d2c38e84c4d1d62456d3c5b2136080a08ceb77f5a"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/source.rs",
      "bytes": 2052,
      "sha256": "9cc86e3969ab134c0be85b3d4344c7dd7e863bc8d399ee224fd017ddfcb237d8"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/source_tests.rs",
      "bytes": 9678,
      "sha256": "9b8854b316acc5727bb733764c752c9f6a5a71ab6245f6f7110ddd2f2fc9c63c"
    },
    {
      "path": "crates/zryna-driver/src/server_transport/tests.rs",
      "bytes": 9700,
      "sha256": "227cffc82757fa16e3535a681422f86d2d6afac5f520f547602f4eb386e0a0f1"
    },
    {
      "path": "crates/zryna-driver/src/server_workspace/approval.rs",
      "bytes": 3873,
      "sha256": "73f8b64bb4a3204156c7687f08d1b9f6cb2e6469784bc846494829d291ffde69"
    },
    {
      "path": "crates/zryna-driver/src/server_workspace/guest.rs",
      "bytes": 3848,
      "sha256": "e7f639f9b8b95a17519f2754545d82325c10f6a20e6cce83fe048fb01b593887"
    },
    {
      "path": "crates/zryna-driver/src/server_workspace/record.rs",
      "bytes": 5686,
      "sha256": "7d680e55c2008b44db4cdf0449890fa987a2d5281be4dcf519f27b4f2ba33b0f"
    },
    {
      "path": "crates/zryna-driver/src/server_workspace/selection.rs",
      "bytes": 2581,
      "sha256": "8108c352b75bb84189706adc800762e99782ab3e8a820a6a43ecf084ee6947d1"
    },
    {
      "path": "crates/zryna-frontend/Cargo.toml",
      "bytes": 794,
      "sha256": "ceddb21dd4e879afe32c61a361f57b112701d5cff532a9338aad07d7e7c34bb2"
    },
    {
      "path": "crates/zryna-ir/Cargo.toml",
      "bytes": 439,
      "sha256": "eb52e5024fc6fa2f2680fa34ef69cd23a3db3056bc63aab56cdf7aedcbfac51b"
    },
    {
      "path": "crates/zryna-ir/src/expression_order.rs",
      "bytes": 1848,
      "sha256": "3b66d990b8a5bd04fe425df8b5f5d64e93d5bbb7bf3677bd74eb57c282821f1a"
    },
    {
      "path": "crates/zryna-ir/src/lib.rs",
      "bytes": 42608,
      "sha256": "a01a1930004d3e45b4a3eed9183ecbf9f3717ec4c52bc767cfb422a488b582fe"
    },
    {
      "path": "crates/zryna-layout/Cargo.toml",
      "bytes": 398,
      "sha256": "0fb8c733cdfd06f51fa4da18da00bdf960ef4aba07d68b10cc32b0fe926509cd"
    },
    {
      "path": "crates/zryna-native-c-ir/Cargo.toml",
      "bytes": 599,
      "sha256": "3d50a77a685fc9ea316a9be65ef1bd5f429cd79d05e46b8381576c2b71edd3e2"
    },
    {
      "path": "crates/zryna-native-mir/Cargo.toml",
      "bytes": 708,
      "sha256": "c0234c90df0800ae5010e2ce33ad170585f42a4d52211c62040db1a48b79303f"
    },
    {
      "path": "crates/zryna-ownership-runtime-abi/Cargo.toml",
      "bytes": 403,
      "sha256": "931ac7e22f84198aec64fbeea65aa9475262b4fcd33207f3dc9bb1c31e09efd2"
    },
    {
      "path": "crates/zryna-package/Cargo.toml",
      "bytes": 281,
      "sha256": "08d2f6753a4eba516bcf4ca7972246a621c0a892aeedcee41496ed8085f26e94"
    },
    {
      "path": "crates/zryna-process/Cargo.toml",
      "bytes": 226,
      "sha256": "32236a07febec6d4fe2df1969fb5ee09b7261f851773ac0524e2eb3fb8ff7b0d"
    },
    {
      "path": "crates/zryna-semantics/Cargo.toml",
      "bytes": 554,
      "sha256": "b297ae192bd6a3522396f67f5a75611a246ea11d929cb0bbfa6ba95f85b42573"
    },
    {
      "path": "crates/zryna-source/Cargo.toml",
      "bytes": 278,
      "sha256": "68eb7dfd534fe43247fafadc1bcdc2a94044e3663c07d04e3fe3e96500e31438"
    },
    {
      "path": "crates/zryna-source/src/lib.rs",
      "bytes": 33303,
      "sha256": "86d795fd66f60ce2038855a6326856bc8f67e11d4bacd9069f5bb03b59dfbddc"
    },
    {
      "path": "crates/zryna-source/src/span_validation.rs",
      "bytes": 1658,
      "sha256": "c4221f43439c75d2c0c9bdeacbade2ea40a5b6698a3d8d8a442a5f2cef9cb030"
    },
    {
      "path": "crates/zryna-syntax/Cargo.toml",
      "bytes": 378,
      "sha256": "947f8680963c759595dd85a79ca71acb34cbd6c6d27b1ab10f4cd0d187d616ad"
    },
    {
      "path": "crates/zryna-syntax/src/command_h1_v1.rs",
      "bytes": 8656,
      "sha256": "eb9b791dcaa2ae77123938403f7bf9d8a3c999cd3f2fd890853f39456378b4dc"
    },
    {
      "path": "crates/zryna-syntax/src/v4/limits.rs",
      "bytes": 1715,
      "sha256": "ab1c2cf15817d4144f9f4524b81bb22233f1e8bf680c7c8698b77271b3f455ff"
    },
    {
      "path": "crates/zryna-windows-filesystem/Cargo.toml",
      "bytes": 1054,
      "sha256": "e117afe3890548509398c81d3b7cc01c1406b554fbfeaacab86d881367d00d1d"
    },
    {
      "path": "docs/CODE_NAVIGATION.md",
      "bytes": 36867,
      "sha256": "1423ea67b74fde8820b09506177abf49084d1fbd7dcc89ede81b436903c77934"
    },
    {
      "path": "docs/DOCUMENTATION_BUNDLES.md",
      "bytes": 5301,
      "sha256": "8886b7b632642d6ea8c9f98fb059f9c0e579194354671ee297b435f2e5c8e8e8"
    },
    {
      "path": "examples/browser-component/index.html",
      "bytes": 1195,
      "sha256": "3c1f1ee12a237c04b51dab075c53edce5155af27b40274ad3b6fcc80176f62a9"
    },
    {
      "path": "examples/browser-component/serve.mjs",
      "bytes": 1013,
      "sha256": "83f2229d155a406a0cfd7aba97474b0261dea77b6d5c20f5e8edde05b6c51ffa"
    },
    {
      "path": "examples/wasi-command/error.zry",
      "bytes": 49,
      "sha256": "be57932b5031a38a149e72d8c47859b1775b075c5270b3be78d00799fc8989bd"
    },
    {
      "path": "examples/wasi-command/lookup.zry",
      "bytes": 356,
      "sha256": "2e0a162e46fcdd05275ce8468e85121fd4f70fe38e6601be033ffebe0689cce9"
    },
    {
      "path": "examples/wasi-command/pure.zry",
      "bytes": 48,
      "sha256": "783f24c20178854ee1620ebe067136edbf43adff855606084de204f1cb074e72"
    },
    {
      "path": "examples/wasi-server/status-200.zry",
      "bytes": 46,
      "sha256": "63ff76225f2a5e8eb33553f5c6409ed68e95ec357d36f88926dc114d2c38077e"
    },
    {
      "path": "examples/wasi-server/status-599.zry",
      "bytes": 46,
      "sha256": "07173d6ae0404b515fc1ad706368230dd77c678122259c19a308041f9995c762"
    },
    {
      "path": "package.json",
      "bytes": 7060,
      "sha256": "634e2b9c3594a6645851510ec1ebc82d4f2b6a9fe184201060e9081650eb9425"
    },
    {
      "path": "pnpm-lock.yaml",
      "bytes": 45843,
      "sha256": "40ba58636b8aced200535e1b33eb4623967af5fe4c9fcd09f2b917dbe4196d47"
    },
    {
      "path": "pnpm-workspace.yaml",
      "bytes": 112,
      "sha256": "25fc9bd07f534da5ecd60721a3ca93987aacd656ceeae3511d7bf495012f5b1b"
    },
    {
      "path": "rust-toolchain.toml",
      "bytes": 86,
      "sha256": "279d77167cec5426fa80f457cd066dc74a360fbe4e2816f4f3fa01487a918fdc"
    },
    {
      "path": "schemas/zryna-distribution-build-input-v1.schema.json",
      "bytes": 9101,
      "sha256": "22cac71dcde3e52e7714ce73a15c8d46a01da649b1087c8af6a16dac9c1b103d"
    },
    {
      "path": "schemas/zryna-preassembly-gates-v1.schema.json",
      "bytes": 2078,
      "sha256": "c5e4134e57feed0b2a6e02116b75401082a61f134388ab15c39bd495dc1604d8"
    },
    {
      "path": "schemas/zryna-release-build-result-v1.schema.json",
      "bytes": 2318,
      "sha256": "5671f31cb90ac39758acf4f125aca6b488fe815a91ca3a7ede24d3a928ca3aea"
    },
    {
      "path": "schemas/zryna-release-qualification-architecture-v1.schema.json",
      "bytes": 2729,
      "sha256": "773b4e6d4ea96ac891ae9e164c0f3df72a3a853e5fadfc3ec0915a516e3a712a"
    },
    {
      "path": "schemas/zryna-release-reproduction-v1.schema.json",
      "bytes": 2713,
      "sha256": "71d46ec5df4c78f2102221b6572a530c344090089d2a792a0d5518990bee7679"
    },
    {
      "path": "schemas/zryna-source-build-receipt-v1.schema.json",
      "bytes": 2573,
      "sha256": "6cc7d16bf9163d490c04950420a5b5e11ba0bfe470372a14b97da919c14dc1ae"
    },
    {
      "path": "schemas/zryna-wit-capability-profiles-v1.schema.json",
      "bytes": 5612,
      "sha256": "a6a4c60f7b3bc6edab7ed9a3f5f4c364ba03c4115d55ac94df43160d896219d7"
    },
    {
      "path": "schemas/zryna-workspace-v1.schema.json",
      "bytes": 2243,
      "sha256": "7586fbf457abc681c0e926685bd5cc3b803fdcc7f6a69c114becec14c532a13e"
    },
    {
      "path": "scripts/check-m2-conformance.mjs",
      "bytes": 20495,
      "sha256": "035a6726aaed596352c1f00a512fe61d3a81cf3bdf909dd0edf8cb5b5c06e2b8"
    },
    {
      "path": "scripts/check-m2-contract.mjs",
      "bytes": 10223,
      "sha256": "2741172aaad354afaa82c0564ac808998670538b1eef8f8c6b40c1925ed43931"
    },
    {
      "path": "scripts/check-m3-conformance.mjs",
      "bytes": 3662,
      "sha256": "ebd0baf14f9a9804fc8ebb07ac93a4747d8e9ad01d611e732d90003d599f370c"
    },
    {
      "path": "scripts/check-m4-conformance.mjs",
      "bytes": 7592,
      "sha256": "740705896928f1794cda9702683436f3831e1f4fd1ee4820e82ff1e549e96a83"
    },
    {
      "path": "scripts/classify-workflow-paths.mjs",
      "bytes": 8772,
      "sha256": "467726b64d1566817b90d9c662bd5fd404be422927656a45c798c7b057e2d478"
    },
    {
      "path": "scripts/distribution-release/acquire-qualification-materials.mjs",
      "bytes": 13699,
      "sha256": "d7d97ca343c7c599f0748fa5b77f80306b2d2a883448c35e67e328e5790496eb"
    },
    {
      "path": "scripts/distribution-release/canonical.mjs",
      "bytes": 4109,
      "sha256": "34d74ab8a07582ec9187139a59335cd467dfe66ea4c4caccd1b3cd5690512595"
    },
    {
      "path": "scripts/distribution-release/compare-production-candidate-builds.mjs",
      "bytes": 7318,
      "sha256": "45cc71e51a6451b106b38ecc49294ff4b7c6db1ff03287663b2a1891a89f4ce6"
    },
    {
      "path": "scripts/distribution-release/compare-release-builds.mjs",
      "bytes": 6266,
      "sha256": "e5c6ccb3603aa91160c861346f8a72d218664eac8d8b2fe517e8ca146416bbe0"
    },
    {
      "path": "scripts/distribution-release/input.mjs",
      "bytes": 1881,
      "sha256": "aed7d5690998fac880974904c0b1f7dd3108502a8263a84c4e2f07c5eb4d5764"
    },
    {
      "path": "scripts/distribution-release/installed-execution.mjs",
      "bytes": 7709,
      "sha256": "f2c2f105f01f44afee9877bb46d1bd3279031858d7c7380c98ba9641e01c08ff"
    },
    {
      "path": "scripts/distribution-release/installed-lifecycle.mjs",
      "bytes": 4399,
      "sha256": "0339be515a525bf65e26752f0df174bb98870afefc798f4af885888a445f715b"
    },
    {
      "path": "scripts/distribution-release/observe-qualification-tools.mjs",
      "bytes": 9990,
      "sha256": "a33ad68c8969ffee1163d95ba07e6280c86030f8c2d5c948cc0436d9ab285342"
    },
    {
      "path": "scripts/distribution-release/provision-qualification-cargo.mjs",
      "bytes": 11796,
      "sha256": "d5d923e468297afc1ac804f3273fe1028b25b4bbedf4a22a5e2027bcef0f7e0c"
    },
    {
      "path": "scripts/distribution-release/qualification-archive-capability.mjs",
      "bytes": 12167,
      "sha256": "e9d3c929863d465e7139acb14d94b102fa5fafb46ef5d6ddaaca47b9853ca9ae"
    },
    {
      "path": "scripts/distribution-release/release-files.mjs",
      "bytes": 8253,
      "sha256": "d8ca69648dcef264b6c1fc2fc59b23ab151b669a75166b05dfbf091f967ebd0f"
    },
    {
      "path": "scripts/distribution-release/run-installed-acceptance.mjs",
      "bytes": 9312,
      "sha256": "517a78c64e013be679858a4ab4ee6aac4c9014bcd86d81c36a193c06cbaaf733"
    },
    {
      "path": "scripts/distribution-release/validate-build-input.mjs",
      "bytes": 5264,
      "sha256": "b4ab96cea60074bdf524de306ef9f306ef58834d634f418d70ae78783bf0320d"
    },
    {
      "path": "scripts/distribution-release/validate-preassembly-gates.mjs",
      "bytes": 2964,
      "sha256": "e4fdcf68761cd38452317e5795faf1f475c902f53470f725858d306646613095"
    },
    {
      "path": "scripts/distribution-release/validate-production-recipe.mjs",
      "bytes": 541,
      "sha256": "078906a72375a1d00c4a9426cfcb18385d194000a23fd658aa5402c708abe296"
    },
    {
      "path": "scripts/distribution-release/validate-release-qualification-architecture.mjs",
      "bytes": 1016,
      "sha256": "3aac3c06ef49aebb4512058dcfb36aed610e7275f55d4d9b55c4ccae7b156aca"
    },
    {
      "path": "scripts/distribution-release/validate-source-build-receipt.mjs",
      "bytes": 2968,
      "sha256": "b24ffce9d1aafa83b23574c243e55f09eb1a7442b315ba7f4df783bca30f08cd"
    },
    {
      "path": "scripts/distribution/archive-entries.mjs",
      "bytes": 1728,
      "sha256": "bea7c3ca8dc4bad4770d28d3450429e24e9fc361b5ee40c8cfc673491a865384"
    },
    {
      "path": "scripts/distribution/archive-tar.mjs",
      "bytes": 4736,
      "sha256": "9a2d906a3b085055ad02d4bf0752ba7544dd32776203fb22db193e4b40c5aaee"
    },
    {
      "path": "scripts/distribution/archive-zip.mjs",
      "bytes": 4550,
      "sha256": "721904a07539ddd2896c6a53cdbf931be5431953df7c9a963f9f556c61546ff1"
    },
    {
      "path": "scripts/distribution/assemble.mjs",
      "bytes": 4734,
      "sha256": "45127bdf07d2fe6fba30966aa05dca42434982c712a98d981a42e4945d01fe50"
    },
    {
      "path": "scripts/distribution/binary-identity.mjs",
      "bytes": 1290,
      "sha256": "a4ebfbc73225ce54fe021e196284963dedfea0cecbde706ec9a759bd4c7cda1c"
    },
    {
      "path": "scripts/distribution/canonical.mjs",
      "bytes": 3838,
      "sha256": "7bee4ec69538d074fb134dd9cc2134c8964d336f3a08cef877029a9c3b02f63d"
    },
    {
      "path": "scripts/distribution/command-h1-rust-binding-v1.json",
      "bytes": 994,
      "sha256": "41065082c185e09063aad15c9b5be7b371a853ba7b91f2c1f456dbcd55d224cd"
    },
    {
      "path": "scripts/distribution/command-h1-rust-binding.mjs",
      "bytes": 1707,
      "sha256": "ecedf63ba17c299dda3731318e6fff314e042db9fff1db3a3af80675d93ee750"
    },
    {
      "path": "scripts/distribution/documents.mjs",
      "bytes": 4612,
      "sha256": "9a663d11f3488f49378e11fabd57b208c26fe81b75e14df245e43ef9b535139d"
    },
    {
      "path": "scripts/distribution/inventory.mjs",
      "bytes": 6643,
      "sha256": "14331d89d86f610edd3122c941837a4ffb2d968e487f357317f06e130e3f7475"
    },
    {
      "path": "scripts/distribution/material-record.mjs",
      "bytes": 688,
      "sha256": "e9de7ed83bbcb3bbb45f2d6c786e95a60f2e6debe75fe528fb0d016f232320b4"
    },
    {
      "path": "scripts/distribution/materials-rust-command-candidate-v1.json",
      "bytes": 1400,
      "sha256": "499be6581463e71e54ede6d2c5807c346173f34e7505e6faeaae95944c254483"
    },
    {
      "path": "scripts/distribution/materials-rust-v1.json",
      "bytes": 128074,
      "sha256": "32b6343a6dfae19d7c3cdcc50658762c2c0ad08aca15124bdce85b28db38a84b"
    },
    {
      "path": "scripts/distribution/materials-typescript-v1.json",
      "bytes": 3185,
      "sha256": "f43639bd9decf44d95a094caad029a572851e4399c889da6071d4425203da3f6"
    },
    {
      "path": "scripts/distribution/materials.mjs",
      "bytes": 3130,
      "sha256": "552342cef514309452fd96bd0b579c9b6841d26d63dc298ee162aeea4734e46f"
    },
    {
      "path": "scripts/distribution/node-materials.mjs",
      "bytes": 2844,
      "sha256": "016733c07e07694ff7599b0283fffce29f37103c8fa1643e8a4913303f2f20e5"
    },
    {
      "path": "scripts/distribution/npm-materials.mjs",
      "bytes": 15768,
      "sha256": "4d1d72582185d0f6f9f9f23172352451fc9569bc0a05f5ae0b2c5c8664742479"
    },
    {
      "path": "scripts/distribution/payload.mjs",
      "bytes": 3140,
      "sha256": "9132cccee2d32051a040ad1466e74ccca3a5d03137a1eb3ab5b0ccdbe71286e9"
    },
    {
      "path": "scripts/distribution/prepare.mjs",
      "bytes": 4757,
      "sha256": "593be1da1a76e732cd2bdf524da416d1cd0e798bac57e76dbaacd2fa01a16b18"
    },
    {
      "path": "scripts/distribution/provision-release.mjs",
      "bytes": 12829,
      "sha256": "0fb1db41774c87679b83ee04b601b7253c82b07b9b67d138bf2920632e376344"
    },
    {
      "path": "scripts/distribution/provision/evidence.mjs",
      "bytes": 1531,
      "sha256": "3ca4925ccb096e32feac30e613e2ac78f95a06cf51cf55b84176487d16044ca1"
    },
    {
      "path": "scripts/distribution/provision/recipe.mjs",
      "bytes": 2745,
      "sha256": "5d1d4de424f17599728a027eed6cd1cbed185af8cb1961dda77cc22780606a9c"
    },
    {
      "path": "scripts/distribution/release-recipe-v1.json",
      "bytes": 2488,
      "sha256": "0f522abd343ba4e3c07a38d93c778fa7a0e30aac2295abe1f075091f6b6d7db1"
    },
    {
      "path": "scripts/distribution/runtime.mjs",
      "bytes": 717,
      "sha256": "7bec637b6e6c7c5cccb7fe6ce3a80c45274d9266e9dfe3d5f878857ac484f798"
    },
    {
      "path": "scripts/distribution/rust-capture.mjs",
      "bytes": 2205,
      "sha256": "abbb131262989b84a200b4735f07671344df758784d967e2ac307fdf92b72241"
    },
    {
      "path": "scripts/distribution/rust-materials.mjs",
      "bytes": 8110,
      "sha256": "48a5da5d323d1f6696f5acb073ddedf48f1789669e18dbf9675769a0eb27fd9a"
    },
    {
      "path": "scripts/distribution/source-receipt.mjs",
      "bytes": 2276,
      "sha256": "a9ca623f24414fb342e0253f7933a547f137eb555834c0976a7f3474d0be0316"
    },
    {
      "path": "scripts/distribution/verify.mjs",
      "bytes": 4084,
      "sha256": "ed8a9221baea3c1432ab2617e0e9bc55702925308560cfd3713809f8bfa26c32"
    },
    {
      "path": "scripts/docs/bundle.mjs",
      "bytes": 12889,
      "sha256": "b88af8658e5c47ed4b06bdb3b4c28dff0690419ddda31a95873c7674abec7f2b"
    },
    {
      "path": "scripts/lib/m3-gates.mjs",
      "bytes": 3638,
      "sha256": "3742b64cfcfdd6d50f7e6ba0b8b611ad3756addc43012925511b4f85f7c7703d"
    },
    {
      "path": "scripts/m4/collect-output.mjs",
      "bytes": 13327,
      "sha256": "50af362a052145c3d93dc20de4a1b877a05b2a242f67a8bf872253857502dfed"
    },
    {
      "path": "scripts/m4/evidence-browser-byte-boundary.mjs",
      "bytes": 3372,
      "sha256": "90d488196bc9484b738ba45a1f2ec7f9049730819580e2815de0e0106b9861eb"
    },
    {
      "path": "scripts/m4/evidence-browser.mjs",
      "bytes": 4589,
      "sha256": "42c197353ddc63778a35bb9f5e4980340ce8138f352df75c2b2b24a684e94941"
    },
    {
      "path": "scripts/m4/evidence-context.mjs",
      "bytes": 4636,
      "sha256": "55b4ffbdfdaef071bb4b6f2298f8ab26c48e50016d9dae89a6a4bd4434454c0e"
    },
    {
      "path": "scripts/m4/evidence-counts.mjs",
      "bytes": 2842,
      "sha256": "0204bd388a709ba4dcf36be4f6cfff9181b32e8d2dacdb9a7129c05060f5f0ce"
    },
    {
      "path": "scripts/m4/evidence-foundations.mjs",
      "bytes": 8687,
      "sha256": "b9dceaadb4413296a804981ccfcaef27793fb41a1dbaecc8d14dddc06a510584"
    },
    {
      "path": "scripts/m4/evidence-input.mjs",
      "bytes": 3426,
      "sha256": "5d6213b240cdce0deabf1f3cb63ac17ca643c51c60611f365aff60a607735ace"
    },
    {
      "path": "scripts/m4/evidence-installed.mjs",
      "bytes": 8729,
      "sha256": "82d9c67edf488cfb6f29d4539cdba238b157842763ab791ca4044b6d870d6e07"
    },
    {
      "path": "scripts/m4/evidence-node.mjs",
      "bytes": 4781,
      "sha256": "e1695fc788ea506e1d0a52564e1b631fead3c6f5d0766be467a38548da41cb62"
    },
    {
      "path": "scripts/m4/evidence-quantitative.mjs",
      "bytes": 7556,
      "sha256": "e2bae1596776e5613ebfb49e36b6c7951c98eed8ba2f331681911eb807bd419f"
    },
    {
      "path": "scripts/m4/evidence-source.mjs",
      "bytes": 1370,
      "sha256": "f72b98bef003b26e78cb53f8d3647118bbe3b8e753c0732c7d3b281124f03e7b"
    },
    {
      "path": "scripts/m4/evidence.mjs",
      "bytes": 11897,
      "sha256": "370cbe7affd5d1bafd8289095972505f4e1107aef3828c641416f6b78158ba14"
    },
    {
      "path": "scripts/m4/hosted-aggregate.mjs",
      "bytes": 1455,
      "sha256": "a7a0aa505fc828524fbe62e65c6118fa19b5c1defeec00dbe172045f8d9e04b5"
    },
    {
      "path": "scripts/m4/hosted-api.mjs",
      "bytes": 1601,
      "sha256": "1f9cb7ae182243e9d4798afea8e2f9eec017e0406243adc66527894a74cba428"
    },
    {
      "path": "scripts/m4/hosted-archive.mjs",
      "bytes": 6160,
      "sha256": "7d07616422c3297bf2fd5360465d6ca8a9eeaf86fae5eb5d3df718676f355ab1"
    },
    {
      "path": "scripts/m4/hosted-expectations.mjs",
      "bytes": 7190,
      "sha256": "a4a7b5288a4c5b5b0a920162bfdf0d0f6964c11b4860fd7ff8d963e3303c8dbd"
    },
    {
      "path": "scripts/m4/hosted-workflow.mjs",
      "bytes": 5826,
      "sha256": "3c2c4caa578bd4de044c9e8ab2debeeeb7c22714090815a437fba0e4aa360e93"
    },
    {
      "path": "scripts/m4/installed-byte-boundary.mjs",
      "bytes": 2418,
      "sha256": "46df1ef7b0b952207f23fb925314a4a8e1c6c17be072ac939e155817be619205"
    },
    {
      "path": "scripts/m4/installed-cases.mjs",
      "bytes": 2774,
      "sha256": "4da3fccce64d5adb8f6fa32d19580eada6395e70d48dbbc11c41da18d675b494"
    },
    {
      "path": "scripts/m4/installed-identities.mjs",
      "bytes": 3166,
      "sha256": "85b0bfe273fef2f9e90f91aa9ad6dc783b2cd36551ab99a455155164e9f47e5d"
    },
    {
      "path": "scripts/m4/installed-observation.mjs",
      "bytes": 6872,
      "sha256": "6db9d61ca3eb6efaf81319bc8cc5f20477644d139929a03ae4de09ab9ae67aed"
    },
    {
      "path": "scripts/m4/limit-contracts.mjs",
      "bytes": 12117,
      "sha256": "563051de5eb01430875af2499bb71c77f2c7fea94a91fd0024aa035600880eea"
    },
    {
      "path": "scripts/m4/named-gate-cases.mjs",
      "bytes": 3220,
      "sha256": "e9628469af3372d50821ce8b8d395336dacaf34f02ad6e79cbbb9210a49e91f2"
    },
    {
      "path": "scripts/m4/observe-context.mjs",
      "bytes": 4867,
      "sha256": "45e09ff08f259e00b1b9d0f9c355946e6f885eda9eabc35ea09e3699d85c5c0c"
    },
    {
      "path": "scripts/m4/observe-materials.mjs",
      "bytes": 5856,
      "sha256": "913efb4046e927820f26f00dd949c5cef44104b8f706c6172eee21463782f960"
    },
    {
      "path": "scripts/m4/publication-workflows.mjs",
      "bytes": 6915,
      "sha256": "2a677ee1d1fedc8991d7eeca6ecc1245d244521234153911c7a3ad83c67fc7a0"
    },
    {
      "path": "scripts/m4/registry-files.mjs",
      "bytes": 6348,
      "sha256": "0da0990b5992d206405afb751200615abd5ab886c5ed46c5bff7dd25714bd83d"
    },
    {
      "path": "scripts/m4/retain-publication.mjs",
      "bytes": 2679,
      "sha256": "b7590edfdf96cbcc9eb8ed52fc9a3ba3bbb5956543f27fabb7b4a12a70f7400b"
    },
    {
      "path": "scripts/m4/support.mjs",
      "bytes": 9278,
      "sha256": "ea1aa6db405b0044c63e97f344c9d12ac3e6728f218321d2588a0596eea5f269"
    },
    {
      "path": "scripts/m4/verify-ci-result.mjs",
      "bytes": 967,
      "sha256": "8a70226846cb1832b807caf2aae8f465a89e15b9f7f0d443b8ab9afe02d1acfe"
    },
    {
      "path": "scripts/m4/verify-docs-artifact.mjs",
      "bytes": 9922,
      "sha256": "b7e0d1809b678c63e5fb37bdbcfb029787e206025eceb1d08a282c17d3c7c97a"
    },
    {
      "path": "scripts/m4/verify-hosted.mjs",
      "bytes": 8575,
      "sha256": "09b1e4e3b35b7ca8b0079904c16cf5ce8452d41519a8bf494bd89c44b3e1e6b5"
    },
    {
      "path": "scripts/m4/verify-publication.mjs",
      "bytes": 7416,
      "sha256": "7201af17db1145e02f05ca9d632ce2b36e1808acc834e7a0d35572acc0c8f412"
    },
    {
      "path": "scripts/run-m0-conformance.mjs",
      "bytes": 14915,
      "sha256": "3338c5e194043e2055c089806057c5dc32c9828f9721bcf32c25132a4bf470a4"
    },
    {
      "path": "scripts/run-m2-conformance.mjs",
      "bytes": 2843,
      "sha256": "f182d1a08369c97763569ae827d745d6f9eb67fa253e1ec24974e67d99fd1e73"
    },
    {
      "path": "scripts/run-m2-quick.mjs",
      "bytes": 3957,
      "sha256": "a4ece8f7398e992baf035375321c39852c83e583ef25f13c6b694ff641c408f3"
    },
    {
      "path": "scripts/run-m3-conformance.mjs",
      "bytes": 183,
      "sha256": "00f9f09435c658190ab34b48ca3dba65f4d64406b5248d026c7853b4556f0844"
    },
    {
      "path": "scripts/run-m4-conformance.mjs",
      "bytes": 7094,
      "sha256": "f472209c8da1653ccf9b6aa2988103c0475e208a857014ef0ca6cebfe714734d"
    },
    {
      "path": "scripts/scalar-host/acquire-browser.py",
      "bytes": 7970,
      "sha256": "c1c2e0b473599dcc55d8413eb85b5b58b82c05479bb3834994deb28b8b473476"
    },
    {
      "path": "scripts/scalar-host/run-conformance.mjs",
      "bytes": 9400,
      "sha256": "acbdef5584b57c0531fde06889380c9a5290012ddfb9eeb62e637c6b253e2b21"
    },
    {
      "path": "spec/wit/capability-profiles-v1/worlds.wit",
      "bytes": 1129,
      "sha256": "cee8923feb7e50b545a56ecf01c14ca2fd9264e2dfdbe9f54eca6b07f0886cfe"
    },
    {
      "path": "tests/browser-component/byte-boundary.mjs",
      "bytes": 8529,
      "sha256": "09f53fb3625a6f424df5d54078139fee860da050275f8ddd7d94b7a116f0a194"
    },
    {
      "path": "tests/browser-component/run.mjs",
      "bytes": 9330,
      "sha256": "529e24ef12a0db462a239acbeaaed220886311fb22f21f75859fd790d370f9b1"
    },
    {
      "path": "tests/installed-command-qualification/byte-boundary.mjs",
      "bytes": 1352,
      "sha256": "c91732716cd2d6d5a30db704d38c6e98a1faa9c2333bd4f27ee6e4da41896b8e"
    },
    {
      "path": "tests/installed-command-qualification/fixtures/byte-boundary/exact.zry",
      "bytes": 87161,
      "sha256": "e42343006d7e7dea34eb5a27f9034dbecce8f6d7ba10267443f050021e7d495d"
    },
    {
      "path": "tests/installed-command-qualification/fixtures/byte-boundary/extra.zry",
      "bytes": 87161,
      "sha256": "d15526b21a384033b77467c959c8fae1769b2a000cf444643ce29271c8b6e8b7"
    },
    {
      "path": "tests/installed-command-qualification/inputs.mjs",
      "bytes": 1844,
      "sha256": "b87d3b649357235bcc95c76e9ea4bbdca921f91b903dd64a0f7d53431c3ca550"
    },
    {
      "path": "tests/installed-command-qualification/prepare.mjs",
      "bytes": 12467,
      "sha256": "41ace74d0c394ddf86d23889bcf8303e14500602570ace5887b8a742feed3b68"
    },
    {
      "path": "tests/installed-command-qualification/run.mjs",
      "bytes": 9454,
      "sha256": "c843dc542a33333b4b229ebda93fcb22a0133d8bf44aab16ca14ceb5df8f31e0"
    },
    {
      "path": "tests/m0-conformance-v1.json",
      "bytes": 11027,
      "sha256": "4882737250ef9449e5a53e5fa1f7c4176fab8fb49ed6426fe5d4f9d99a050afb"
    },
    {
      "path": "tests/m0-conformance.test.mjs",
      "bytes": 12937,
      "sha256": "2dc3ec3caa2e107ea4d3f87077349a8260fffc5434b1e64b94e04aae35aa3471"
    },
    {
      "path": "tests/m1-conformance-v1.json",
      "bytes": 1076,
      "sha256": "4fd99b68b1f4275c02389a5fd525ed238cf61138e2108e0a129b3714f85e94a3"
    },
    {
      "path": "tests/m2-conformance-v1.json",
      "bytes": 28273,
      "sha256": "cf07d765c26364cd127b8fdba7d6cefec535876b71916402624c6d77c1140c18"
    },
    {
      "path": "tests/m2-contract.test.mjs",
      "bytes": 13732,
      "sha256": "e63082c2637936ba50d68c056e2ca4c627138dc86df57ce1d54b5a0aabfcdc1f"
    },
    {
      "path": "tests/m2-manifest-contract.test.mjs",
      "bytes": 4032,
      "sha256": "fee44795ce6cd891a4f0c8aeeb1777cefd1744d65134c049d09c72233e94da41"
    },
    {
      "path": "tests/m3-conformance-v1.json",
      "bytes": 32696,
      "sha256": "1d29f8d5c24cf1215d64f340576934f674f5cfb89a51d4f82a3882fe75cb36b9"
    },
    {
      "path": "tests/m4-allocation-core.test.mjs",
      "bytes": 7445,
      "sha256": "19809531ed2101bfd6475e07a4c801568b689b29ab8766ba3b7acc489cc6c9ad"
    },
    {
      "path": "tests/m4-browser-byte-evidence.test.mjs",
      "bytes": 6067,
      "sha256": "ab9a728763d1c1c55b45a689661c9516d279b5c3063140c15dc1bf88420ae2a3"
    },
    {
      "path": "tests/m4-conformance.test.mjs",
      "bytes": 15200,
      "sha256": "a1406c2767a3c15034ec57a1f52f80add8cf46d9fcee871190acc729e786ee06"
    },
    {
      "path": "tests/m4-evidence.test.mjs",
      "bytes": 52726,
      "sha256": "1cb39154128a885553f69136a5fbfcefb8467a5ae6186a689b2fd5c7d8ed80ec"
    },
    {
      "path": "tests/m4-fixtures/allocation-core/capacity-inspect.mjs",
      "bytes": 1953,
      "sha256": "cbfd8a73605bf5757966663263240806d641b1ade927503a70912dc475018a85"
    },
    {
      "path": "tests/m4-fixtures/allocation-core/inspect.mjs",
      "bytes": 7120,
      "sha256": "6cfb623b5e6af136ef8881ee6798d8d79be192ecdca24fb18380c5a528ea6ebe"
    },
    {
      "path": "tests/m4-fixtures/allocation-core/wasm-inspection.mjs",
      "bytes": 6198,
      "sha256": "86401a8d648496410d575d98e925f82f30d5ed6c1dc1a22ac678b1645a78a672"
    },
    {
      "path": "tests/m4-hosted.test.mjs",
      "bytes": 19910,
      "sha256": "d1c8de932c47257dde2bd7048a3e9ac3278f1027127edf305af3d81026e687e2"
    },
    {
      "path": "tests/m4-limit-contract-fixtures.mjs",
      "bytes": 5034,
      "sha256": "bf0530e09642163289fe2cb20f222da6e4d9f9417a38ba27e80922671e19f123"
    },
    {
      "path": "tests/m4-limit-contracts-v1.json",
      "bytes": 88928,
      "sha256": "b35368682735c16683f45929f52c5343b8e28c23e0927309a462d2a66526da6f"
    },
    {
      "path": "tests/m4-limit-contracts.test.mjs",
      "bytes": 12002,
      "sha256": "1b1a21c2c22370b892da72e8b1aa0558fddf7450e7f17dc6f0887fe839050e4f"
    },
    {
      "path": "tests/m4-public-docs.test.mjs",
      "bytes": 9865,
      "sha256": "ec552a741d16f0020ce6047cebf82786a9e5addd34f66edf98f7b4c27d0810b0"
    },
    {
      "path": "tests/m4-publication.test.mjs",
      "bytes": 13229,
      "sha256": "4be7de6a0ecc95f6974c71b95619369fac7ded2d69afffce9d70fc7353c7d3e3"
    },
    {
      "path": "tests/m4-quantitative.test.mjs",
      "bytes": 14208,
      "sha256": "96bc9a148d2d887eed4db7a7a5ddb7c0d300c416dc6d0405266350b2e5c0cbb4"
    },
    {
      "path": "tests/m4-scalar-core.test.mjs",
      "bytes": 9058,
      "sha256": "a9ee888307bab7ef61d859e844edbf59401f97c38699e4264ed935ff84f36ae1"
    },
    {
      "path": "tests/m4-workflow-cases.mjs",
      "bytes": 4785,
      "sha256": "e249b6b28393f1801adb73308a0f9c4ac2e8ea471537cfbc828f67832fb965a3"
    },
    {
      "path": "tests/m4-workflow.test.mjs",
      "bytes": 8954,
      "sha256": "3936df9217216c6ea66152893a138b635c81e163b01914ec3d05560b6d035036"
    },
    {
      "path": "tests/scalar-host-conformance.test.mjs",
      "bytes": 9336,
      "sha256": "9d75ed5784ea5d5ffdb3b3ffd8e6b801cdb0b21de79e8cc002c0cc01d3acb947"
    },
    {
      "path": "tests/scalar-host/binding.mjs",
      "bytes": 2553,
      "sha256": "392bf8eeeed3c246f3c4fd567f0f0b139a468b89526a40f57458de50d1a58987"
    },
    {
      "path": "tests/scalar-host/browser-fixture.mjs",
      "bytes": 7060,
      "sha256": "b2aee6a2a52708b59a71da655500129b6ab232727fbbb8ccb103904f06c6641c"
    },
    {
      "path": "tests/scalar-host/browser-pin.json",
      "bytes": 1272,
      "sha256": "de1c55800622989c1f3e450deb3d1c742079494f4abf7a5ee13650966d572ea9"
    },
    {
      "path": "tests/scalar-host/corpus.js",
      "bytes": 3107,
      "sha256": "19c263c99856c8add3a9ca4327f0a75e85f5a78bb35429dce8371dc9c7908d5e"
    },
    {
      "path": "tests/wasi-command-source-fixtures/README.md",
      "bytes": 1597,
      "sha256": "913bba63e97f3c779404e8747de41028e5739f95c5d07d9c058370e700efdba4"
    },
    {
      "path": "tests/wasi-command-source-fixtures/clone-aggregate.json",
      "bytes": 13521,
      "sha256": "8d8081ea596b70f57427c8cc68b8451219cd29c2fe6170c99a0d32264377fac6"
    },
    {
      "path": "tests/wasi-command-source-fixtures/clone-aggregate.zry",
      "bytes": 219,
      "sha256": "7016deab0f3029b63750476cf9262def2cea7b2aac32640e552a61486a4d3150"
    },
    {
      "path": "tests/wasi-command-source-fixtures/clone-generic.json",
      "bytes": 16320,
      "sha256": "81cbbef51d5a830bc65520baf9d9acee8a94cdccc157b8aee1da2b245b28e436"
    },
    {
      "path": "tests/wasi-command-source-fixtures/clone-generic.zry",
      "bytes": 248,
      "sha256": "a6259a7e44318320375445fc1c82827d295593f2720bb804b85040386ecc2635"
    },
    {
      "path": "tests/wasi-command-source-fixtures/clone-vector-growth.json",
      "bytes": 25263,
      "sha256": "1e42cb2b5569e4224ab9760bfb00840e5151602cb9f93332d609914915482e0c"
    },
    {
      "path": "tests/wasi-command-source-fixtures/clone-vector-growth.zry",
      "bytes": 276,
      "sha256": "1a6a67a0e2062a6ee2d1c160f8c52771fe3996c935004db23f26b5ec55fd33d0"
    },
    {
      "path": "tests/wasi-command-source-fixtures/clone-vector.json",
      "bytes": 10535,
      "sha256": "e9fde2f0e6872cfc338066334752483f50ea0c4c2378d5498f3fecfd06be456c"
    },
    {
      "path": "tests/wasi-command-source-fixtures/clone-vector.zry",
      "bytes": 153,
      "sha256": "1a50787b811111c6bec627e46ce40d271bdbc4d0ebe22b9c8c8d8e3d2854bad7"
    },
    {
      "path": "tests/wasi-command-source-fixtures/consumed-environment-shared.zry",
      "bytes": 425,
      "sha256": "6032c36781dad4cd8ea15640eb735bcaadd94f587868d90f32a5de31c8cb22c7"
    },
    {
      "path": "tests/wasi-command-source-fixtures/consumed-environment-vector.zry",
      "bytes": 433,
      "sha256": "09b0b2ed1e29404abd6c3519d6ed0ffe278bdc4b82847a29d985e70544695868"
    },
    {
      "path": "tests/wasi-command-source-fixtures/control-flow.json",
      "bytes": 15693,
      "sha256": "42323d72d81cc1968e5dfc164f043abf0ed3c35e214010a7b68f0901a98a877c"
    },
    {
      "path": "tests/wasi-command-source-fixtures/control-flow.zry",
      "bytes": 210,
      "sha256": "24f8ea86b4582b95eccb4135997943923f11c25e623c06d42aacf64c381a1a3c"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-extra-export.json",
      "bytes": 5495,
      "sha256": "48d6fa69a6fbfc2608a46285c68317bb93cf0a586a8501f606ce7539bc09ef0e"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-extra-export.zry",
      "bytes": 94,
      "sha256": "1ac172e777ac4c717e2b7d792eede1158b96145e7adea0ced8c6146c4697bfab"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-missing-export.json",
      "bytes": 2782,
      "sha256": "29aeb488b93f33452cf3a30857257f795e2690519ad2ed4e8d8f9c39b90ebeff"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-missing-export.zry",
      "bytes": 39,
      "sha256": "34360ed14c45c2c713736fd931f582662931a565f8222d064c5459a66e9b4d62"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-name.json",
      "bytes": 2862,
      "sha256": "05706cef74038bf94922130ce798f4b4c64080ffb392252655c530dd178ec141"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-name.zry",
      "bytes": 47,
      "sha256": "f959da9b146c5b8dcb08dbc4dfd0ee8569ff2f5f5b7a58d8498ebd8dd6d3a1da"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-parameter.json",
      "bytes": 3835,
      "sha256": "5ba0f97837a8579a91858e8adf03133b311179e0eb3d675a9f3e879f94ed483a"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-parameter.zry",
      "bytes": 58,
      "sha256": "5a77e3b996768d6b2d2ec4952f5dd2132f12b8269b7d33216a046a3752017efc"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-result.json",
      "bytes": 2861,
      "sha256": "aea642ad7f8739cb3c753f5ea61dc88204abdf9926c71bc97182c41d31fa9bc8"
    },
    {
      "path": "tests/wasi-command-source-fixtures/entry-result.zry",
      "bytes": 42,
      "sha256": "1da8a3c23243e1cf147bd30a9a9f3fcb04e50ec26ec4e2d0651deec5f1deed36"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-consume.json",
      "bytes": 20065,
      "sha256": "8d73b2e0e285cb29eee6a0f68ec84813fae0693b7e6f1e92791dcc70c9fef7bd"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-consume.zry",
      "bytes": 355,
      "sha256": "da8928637d61a834639f71aa364fd650c302bd873153aca4005a1816a9eb998e"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-helper.json",
      "bytes": 11155,
      "sha256": "2e983f4b7c08d2632e5982f90ed642fdeebe78d52ef23ed9dcc57574edfab881"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-helper.zry",
      "bytes": 217,
      "sha256": "673b11a0d21ce1b826fc49d9a93958555951e865542098da6641f8087d64798f"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-incomplete-match.json",
      "bytes": 8516,
      "sha256": "2cea52801b84d408189c7ec77ebf6cf16927006baa6e609c97aac2da2f01dc7e"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-incomplete-match.zry",
      "bytes": 163,
      "sha256": "33fb1db6b0a825fb142cfa2fa11ab44cb095a4e049372bdac930dd11c8f9d321"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-invalid-binding.json",
      "bytes": 9749,
      "sha256": "5f5f656e294d4f5dcb85c981f9af0746970bcd6b5a22f7c9d33cdd98dc6c096a"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-invalid-binding.zry",
      "bytes": 198,
      "sha256": "62d592161721575f0e1a567ccfdf251c4e655725a7a8e9e965ad901ee417e185"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-live-prefix.json",
      "bytes": 13367,
      "sha256": "06f06984371166b548050c33f8284b93907854ec3c6bae734e206984e12cce51"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-live-prefix.zry",
      "bytes": 266,
      "sha256": "6a906a5dcf902157080c9230d88a6f72b8d1dbb013239b932bc7855c4b33cafa"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-match.json",
      "bytes": 9985,
      "sha256": "6e7c8e6fb6ced4ac749728225c86e2586635a9f5180e0c0d6d844228ef10dab0"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-match.zry",
      "bytes": 203,
      "sha256": "7a4b802d78a70e90b3bd815af8e0576aa8cbd5645dadbdd440a3a936c278cfa4"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-trailing.json",
      "bytes": 9985,
      "sha256": "908cf22d2f9f8144bc89124ea63b2c9f75817a72be250c98fe0f661797bbb5ac"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-trailing.zry",
      "bytes": 204,
      "sha256": "56ecce7b6631e2cfd4b0df189e0cf5a50624571f6f638fdc7e0ac60e34703b8a"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-wrong-type.json",
      "bytes": 9973,
      "sha256": "ae8bfe18ae2b1c79b063d65b79106ac47bb5df2b75b269bcae8ab22e7df18567"
    },
    {
      "path": "tests/wasi-command-source-fixtures/environment-wrong-type.zry",
      "bytes": 196,
      "sha256": "15f2045d47509f67aa573b5fd37939adc82b0f9c21f351b32d5cdb590f670712"
    },
    {
      "path": "tests/wasi-command-source-fixtures/forwarded-environment-match-helper.zry",
      "bytes": 278,
      "sha256": "c8f20e628196235b951486e524fc330098cf9d26daef6a7de2e88b39aef4b35a"
    },
    {
      "path": "tests/wasi-command-source-fixtures/fuel-exhaustion.json",
      "bytes": 7796,
      "sha256": "cc52b69137de0ec8331e7ff291e75e4a2b647f1d98331e888ec02ff33f1c6af5"
    },
    {
      "path": "tests/wasi-command-source-fixtures/fuel-exhaustion.zry",
      "bytes": 111,
      "sha256": "bd82eec647bb2128b7b00187109fa854c7621ecc5907436a9ae3a5a4c8572f9c"
    },
    {
      "path": "tests/wasi-command-source-fixtures/language-bounds.json",
      "bytes": 13704,
      "sha256": "804d3e383353f23a43285bce780facbb5ddb26f15a8283c395821f0b8b85d534"
    },
    {
      "path": "tests/wasi-command-source-fixtures/language-bounds.zry",
      "bytes": 159,
      "sha256": "406e0a3c15c88a7a81a8d39dde04fc56e5d3d15f110789bd9f0945983c6dfe1d"
    },
    {
      "path": "tests/wasi-command-source-fixtures/owned-aggregates.json",
      "bytes": 30701,
      "sha256": "ff24542acbab34dafc0076222296464df3072d99166d006e40f365560b7b59eb"
    },
    {
      "path": "tests/wasi-command-source-fixtures/owned-aggregates.zry",
      "bytes": 461,
      "sha256": "3b51cc5bb53965a72eb6b24d252525bbf587b57a1128db971d0efa8e97b898d0"
    },
    {
      "path": "tests/wasi-command-source-fixtures/pure-entry.json",
      "bytes": 2861,
      "sha256": "5c7b5836488dae0caca42bb67199d407077d6f57bc0fbd28342e930d98ae9be5"
    },
    {
      "path": "tests/wasi-command-source-fixtures/pure-entry.zry",
      "bytes": 46,
      "sha256": "6c4f289618cadcf9e78664c66c2443ae36c1ae7fda8c501a17ecc790ea2ba6ec"
    },
    {
      "path": "tests/wasi-command-source-fixtures/pure-false.json",
      "bytes": 2862,
      "sha256": "54b313cd5af27ab7c6c3fb0bcc3c4083939f4193b0151825b43c4576654f967c"
    },
    {
      "path": "tests/wasi-command-source-fixtures/pure-false.zry",
      "bytes": 47,
      "sha256": "061f166a633ee47a8338536c06fc321f7dcd53556ad5cf629d832230decb009c"
    },
    {
      "path": "tests/wasi-command-source-fixtures/unused-environment-match-helper.zry",
      "bytes": 268,
      "sha256": "7145f38a1b5b74f72678e5466dab03ed7be7425c75e1d87dac2b0ff99ef87bf3"
    },
    {
      "path": "tests/wasi-command-source-fixtures/unused-environment-shared.zry",
      "bytes": 322,
      "sha256": "714671e892d5320693742858ed663efbbb6ea862b0474bab49042a49d60addd1"
    },
    {
      "path": "tests/wasi-command-source-fixtures/unused-environment-vector-push.zry",
      "bytes": 348,
      "sha256": "53ec932b2702eee20cce8d05081cc09bce4a93ad0fa38a9b9453d80f5a5f9ec7"
    },
    {
      "path": "tests/wasi-command-source-fixtures/unused-environment-vector.zry",
      "bytes": 332,
      "sha256": "f6ff7426d6625f50e7a43a47ea4c91a3cf0086c3a88f44c3a67f1042ad9113ea"
    },
    {
      "path": "tests/wasi-command-source-fixtures/weak-upgrade.json",
      "bytes": 12665,
      "sha256": "d627c8e05f3dc837e3a53d26d9eb8f20c1dbff15ae9ad252eb7bb0f107a61918"
    },
    {
      "path": "tests/wasi-command-source-fixtures/weak-upgrade.zry",
      "bytes": 192,
      "sha256": "e76945c047f85fa492827072a29bf0812b44d1a7c793c5568669f604a22b6257"
    },
    {
      "path": "tests/wit-capability-profiles-v1.json",
      "bytes": 5627,
      "sha256": "20e5f07b8b64ee3daf61ca4c341bc23ef9bb76bcec6973047562147b41706398"
    },
    {
      "path": "tests/workflow-routing/inventory-contracts.mjs",
      "bytes": 7860,
      "sha256": "d05936eb19c0325a48a3134042243c3005a1501df443202e95dadd7f55b10bae"
    },
    {
      "path": "zryna.workspace.json",
      "bytes": 6745,
      "sha256": "c2f01314c1596bc3da2b5b22e73e62642cb51d740111d6553acf7c25a6881593"
    }
  ],
  "requiredExternalEvidence": [
    "exact-candidate-local-and-hosted-M0-M4",
    "linux-and-windows-browser",
    "linux-and-windows-fresh-install",
    "same-host-artifact-reproducibility",
    "documentation-export-and-live-publication",
    "tool-binary-and-node-typescript-browser-material-identities",
    "complete-candidate-Git-tree-outside-selected-static-source-inventory"
  ],
  "lifecycleExemptions": [
    "component:build-only-no-guest",
    "browser:post-hoc-deadline-cannot-preempt-synchronous-guest"
  ],
  "rows": [
    {
      "id": "component-linux",
      "profile": "default-m1",
      "mode": "checkout-build",
      "host": {
        "os": "linux",
        "architecture": "x64",
        "baseline": "ubuntu-latest:observed-baseline-required",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "build-only"
      },
      "ownerIssues": [
        398
      ],
      "contract": "crates/zryna-backend-webassembly/src/scalar_component/mod.rs",
      "selector": {
        "command": "build",
        "profile": null,
        "target": "component",
        "signature": "i32 parameters -> i32"
      },
      "identities": {
        "world": "zryna:capability-profiles/browser@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.scalar-component.v1",
        "manifest": "manifest-v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "coreSha256",
          "interfaceSha256"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [],
        "effective": [],
        "rootApprovals": [],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "environment",
          "monotonic-clock"
        ],
        "operationLimits": "no guest imports or host grants"
      },
      "artifacts": [
        {
          "path": "component/{stem}.wasm",
          "kind": "webassembly-component"
        },
        {
          "path": "zryna-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "component-component-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "component-bytes-exact-limit",
              "firstExtraCase": "component-bytes-first-extra"
            },
            "public": {
              "kind": "not-applicable-emission"
            },
            "bindingCaseIds": [
              "component-public-build"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "component-public-build",
          "rowIds": [
            "component-linux",
            "component-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "case-component-public-build",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "component_build_publishes_one_audited_build_only_artifact",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "one component artifact; manifest v1 targets component; component run and M2 selector exit2 C1012 with no bundle"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-build"
        },
        {
          "id": "component-replay",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::component_is_reproducible_and_retains_the_exact_core_and_world",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two emitted byte arrays equal; unchanged core; exact browser world"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-denied-import",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-denied-import",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::undeclared_component_import_is_rejected",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "undeclared import rejected before authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-substitution",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-substitution",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::substituted_wit_and_component_identity_fail_before_authority_is_returned",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "changed WIT and component identity reject"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-bytes-exact-limit",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete encoded component at inclusive byte ceiling accepted; first extra byte W4021"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-component-audit"
        },
        {
          "id": "component-bytes-first-extra",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete encoded component at inclusive byte ceiling accepted; first extra byte W4021"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-component-audit"
        },
        {
          "id": "component-interface-consistency",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-interface-consistency",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_exact_interfaces_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two expected exports accepted; same component with expected slice of one rejected; no maximum-exports claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "expected-interface-consistency"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-component-public-build",
        "case-component-replay",
        "case-component-denied-import",
        "case-component-substitution",
        "case-component-bytes-exact-limit",
        "case-component-interface-consistency",
        "case-wit-closure"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces",
        "guest-execution-and-guest-lifecycle"
      ]
    },
    {
      "id": "component-windows",
      "profile": "default-m1",
      "mode": "checkout-build",
      "host": {
        "os": "win32",
        "architecture": "x64",
        "baseline": "windows-latest:observed-baseline-required",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "build-only"
      },
      "ownerIssues": [
        398
      ],
      "contract": "crates/zryna-backend-webassembly/src/scalar_component/mod.rs",
      "selector": {
        "command": "build",
        "profile": null,
        "target": "component",
        "signature": "i32 parameters -> i32"
      },
      "identities": {
        "world": "zryna:capability-profiles/browser@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.scalar-component.v1",
        "manifest": "manifest-v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "coreSha256",
          "interfaceSha256"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [],
        "effective": [],
        "rootApprovals": [],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "environment",
          "monotonic-clock"
        ],
        "operationLimits": "no guest imports or host grants"
      },
      "artifacts": [
        {
          "path": "component/{stem}.wasm",
          "kind": "webassembly-component"
        },
        {
          "path": "zryna-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "component-component-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "component-bytes-exact-limit",
              "firstExtraCase": "component-bytes-first-extra"
            },
            "public": {
              "kind": "not-applicable-emission"
            },
            "bindingCaseIds": [
              "component-public-build"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "component-public-build",
          "rowIds": [
            "component-linux",
            "component-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "case-component-public-build",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "component_build_publishes_one_audited_build_only_artifact",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "one component artifact; manifest v1 targets component; component run and M2 selector exit2 C1012 with no bundle"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-build"
        },
        {
          "id": "component-replay",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::component_is_reproducible_and_retains_the_exact_core_and_world",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two emitted byte arrays equal; unchanged core; exact browser world"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-denied-import",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-denied-import",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::undeclared_component_import_is_rejected",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "undeclared import rejected before authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-substitution",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-substitution",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::substituted_wit_and_component_identity_fail_before_authority_is_returned",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "changed WIT and component identity reject"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-bytes-exact-limit",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete encoded component at inclusive byte ceiling accepted; first extra byte W4021"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-component-audit"
        },
        {
          "id": "component-bytes-first-extra",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete encoded component at inclusive byte ceiling accepted; first extra byte W4021"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-component-audit"
        },
        {
          "id": "component-interface-consistency",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-interface-consistency",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_exact_interfaces_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two expected exports accepted; same component with expected slice of one rejected; no maximum-exports claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "expected-interface-consistency"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-component-public-build",
        "case-component-replay",
        "case-component-denied-import",
        "case-component-substitution",
        "case-component-bytes-exact-limit",
        "case-component-interface-consistency",
        "case-wit-closure"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces",
        "guest-execution-and-guest-lifecycle"
      ]
    },
    {
      "id": "browser-linux",
      "profile": "browser-component-v1",
      "mode": "checkout-browser",
      "host": {
        "os": "linux",
        "architecture": "x64",
        "baseline": "ubuntu-22.04",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Chrome for Testing 153.0.8010.12"
      },
      "ownerIssues": [
        399
      ],
      "contract": "docs/BROWSER_COMPONENT_V1.md",
      "selector": {
        "command": "build",
        "profile": "browser-component-v1",
        "target": "component",
        "signature": "i32 parameters -> i32"
      },
      "identities": {
        "world": "zryna:capability-profiles/browser@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.browser-component-bindings.v1",
        "manifest": "zryna-browser-component-v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "coreSha256",
          "interfaceSha256",
          "loaderSha256",
          "declarationSha256"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [],
        "effective": [],
        "rootApprovals": [],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "environment",
          "monotonic-clock"
        ],
        "operationLimits": "no guest imports or host grants"
      },
      "artifacts": [
        {
          "path": "component/{stem}.wasm",
          "kind": "webassembly-component"
        },
        {
          "path": "component/{stem}.mjs",
          "kind": "ecmascript-module"
        },
        {
          "path": "component/{stem}.d.mts",
          "kind": "typescript-declarations"
        },
        {
          "path": "zryna-browser-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "browser-component-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "component-bytes-exact-limit",
              "firstExtraCase": "component-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "browser-public-canonical-component-exact-limit",
              "firstExtraCase": "browser-public-canonical-component-first-extra"
            },
            "bindingCaseIds": [
              "browser-node-public",
              "browser-chrome-public"
            ]
          }
        },
        {
          "id": "deadline",
          "minimum": 1,
          "maximum": 30000,
          "unit": "milliseconds",
          "contractId": "browser-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "browser-node-deadline-exact-limit",
              "firstExtraCase": "browser-node-deadline-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "browser-chrome-deadline-exact-limit",
              "firstExtraCase": "browser-chrome-deadline-first-extra"
            },
            "bindingCaseIds": [
              "browser-node-public",
              "browser-chrome-public"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "component-replay",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::component_is_reproducible_and_retains_the_exact_core_and_world",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two emitted byte arrays equal; unchanged core; exact browser world"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-denied-import",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-denied-import",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::undeclared_component_import_is_rejected",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "undeclared import rejected before authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-substitution",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-substitution",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::substituted_wit_and_component_identity_fail_before_authority_is_returned",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "changed WIT and component identity reject"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-bytes-exact-limit",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete encoded component at inclusive byte ceiling accepted; first extra byte W4021"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-component-audit"
        },
        {
          "id": "component-bytes-first-extra",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete encoded component at inclusive byte ceiling accepted; first extra byte W4021"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-component-audit"
        },
        {
          "id": "component-interface-consistency",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-interface-consistency",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_exact_interfaces_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two expected exports accepted; same component with expected slice of one rejected; no maximum-exports claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "expected-interface-consistency"
        },
        {
          "id": "browser-node-public",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "case-browser-node-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_bundle_and_node_bindings_are_sealed",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "sealed generated bundle; scalar signed-i32 and typed rejection corpus"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-node-companion"
        },
        {
          "id": "browser-selector-denial",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "case-browser-selector-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_selector_rejects_other_targets_and_run_without_a_bundle",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "other targets and component run reject with no bundle"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-build"
        },
        {
          "id": "browser-replay",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "case-browser-replay",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_replays_deterministically_and_rejects_stale_binding",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "component/loader/declaration bytes equal across fresh stems; stale binding B3992"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-build"
        },
        {
          "id": "browser-chrome-public",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "checked-in page and generated loader; browser and Node corpus equality; no unexpected routes"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "pinned-chrome-and-node"
        },
        {
          "id": "browser-sealed-scalar-corpus",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "scalar_adapter_interface::tests::host_consumers::pinned_real_browser_and_node_execute_the_same_sealed_scalar_corpus",
            "source": "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "exact retained ESM source/interface/policy; pinned browser and Node same fixed corpus; cleanup confirmed"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "pinned-private-scalar-browser-and-node"
        },
        {
          "id": "browser-full-esm-transport",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "scalar_adapter_interface::tests::host_consumers::retained_esm_transport_accepts_exact_ceiling_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual full32MiB retained ESM transport accepted; first extra rejected; separate required ignored proof; not new public component/browser byte allowance"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-esm-transport-ceiling"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "browser-node-deadline-exact-limit",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "case-browser-node-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_bundle_and_node_bindings_are_sealed",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual retained component instantiate at30000; add20,22=42;30001 validation TypeErrorB3993; new draft corpus assertions executed by owning named CLI test"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "deadline",
            "value": 30000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-node-loader-option-admission"
        },
        {
          "id": "browser-node-deadline-first-extra",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "case-browser-node-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_bundle_and_node_bindings_are_sealed",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual retained component instantiate at30000; add20,22=42;30001 validation TypeErrorB3993; new draft corpus assertions executed by owning named CLI test"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "deadline",
            "value": 30001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-node-loader-option-admission"
        },
        {
          "id": "browser-node-minimum-admission",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "case-browser-node-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_bundle_and_node_bindings_are_sealed",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "deadline1 option admitted: either real completed instance or plain Error elapsedB3993; TypeError validation never counts; no1ms compilation-completion promise"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-node-loader-option-admission"
        },
        {
          "id": "browser-chrome-deadline-exact-limit",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual retained component instantiate at30000; add20,22=42;30001 validation TypeErrorB3993; new draft corpus assertions executed by owning named CLI test"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "deadline",
            "value": 30000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "pinned-chrome-loader-option-admission"
        },
        {
          "id": "browser-chrome-deadline-first-extra",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual retained component instantiate at30000; add20,22=42;30001 validation TypeErrorB3993; new draft corpus assertions executed by owning named CLI test"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "deadline",
            "value": 30001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "pinned-chrome-loader-option-admission"
        },
        {
          "id": "browser-chrome-minimum-admission",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "deadline1 option admitted: either real completed instance or plain Error elapsedB3993; TypeError validation never counts; no1ms compilation-completion promise"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "pinned-chrome-loader-option-admission"
        },
        {
          "id": "browser-public-canonical-component-exact-limit",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/fixtures/browser_byte_boundary.rs",
            "tests/browser-component/byte-boundary.mjs",
            "crates/zryna-backend-webassembly/src/scalar_component/tests/byte_boundary.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual public frontend source309364 bytes emits source-authenticated canonical1048576 with1863 ordinary exports; complete source/WIT/core/interface/component/manifest identity checks and fresh replay; changed literal source309365 bytes emits canonical1048577 and publicfactory W4021 exit4 before bundle publication; Node and pinnedChrome execute every exact export, reject the independent canonical first-extra B3991, and recover ordinary42; closed retained browserByteBoundary observation required"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-browser-source-authenticated-canonical-component-byte-boundary"
        },
        {
          "id": "browser-public-canonical-component-first-extra",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/fixtures/browser_byte_boundary.rs",
            "tests/browser-component/byte-boundary.mjs",
            "crates/zryna-backend-webassembly/src/scalar_component/tests/byte_boundary.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual public frontend source309364 bytes emits source-authenticated canonical1048576 with1863 ordinary exports; complete source/WIT/core/interface/component/manifest identity checks and fresh replay; changed literal source309365 bytes emits canonical1048577 and publicfactory W4021 exit4 before bundle publication; Node and pinnedChrome execute every exact export, reject the independent canonical first-extra B3991, and recover ordinary42; closed retained browserByteBoundary observation required"
            ],
            "diagnosticCodes": [
              "ZRYNA-W4021",
              "ZRYNA-B3991"
            ]
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-browser-source-authenticated-canonical-component-byte-boundary"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-component-replay",
        "case-component-denied-import",
        "case-component-substitution",
        "case-component-bytes-exact-limit",
        "case-component-interface-consistency",
        "case-browser-node-public",
        "case-browser-selector-denial",
        "case-browser-replay",
        "pinned-browser-three-proof",
        "case-wit-closure"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces",
        "synchronous-guest-preemption"
      ]
    },
    {
      "id": "browser-windows",
      "profile": "browser-component-v1",
      "mode": "checkout-browser",
      "host": {
        "os": "win32",
        "architecture": "x64",
        "baseline": "windows-2022",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Chrome for Testing 153.0.8010.12"
      },
      "ownerIssues": [
        399
      ],
      "contract": "docs/BROWSER_COMPONENT_V1.md",
      "selector": {
        "command": "build",
        "profile": "browser-component-v1",
        "target": "component",
        "signature": "i32 parameters -> i32"
      },
      "identities": {
        "world": "zryna:capability-profiles/browser@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.browser-component-bindings.v1",
        "manifest": "zryna-browser-component-v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "coreSha256",
          "interfaceSha256",
          "loaderSha256",
          "declarationSha256"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [],
        "effective": [],
        "rootApprovals": [],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "environment",
          "monotonic-clock"
        ],
        "operationLimits": "no guest imports or host grants"
      },
      "artifacts": [
        {
          "path": "component/{stem}.wasm",
          "kind": "webassembly-component"
        },
        {
          "path": "component/{stem}.mjs",
          "kind": "ecmascript-module"
        },
        {
          "path": "component/{stem}.d.mts",
          "kind": "typescript-declarations"
        },
        {
          "path": "zryna-browser-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "browser-component-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "component-bytes-exact-limit",
              "firstExtraCase": "component-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "browser-public-canonical-component-exact-limit",
              "firstExtraCase": "browser-public-canonical-component-first-extra"
            },
            "bindingCaseIds": [
              "browser-node-public",
              "browser-chrome-public"
            ]
          }
        },
        {
          "id": "deadline",
          "minimum": 1,
          "maximum": 30000,
          "unit": "milliseconds",
          "contractId": "browser-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "browser-node-deadline-exact-limit",
              "firstExtraCase": "browser-node-deadline-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "browser-chrome-deadline-exact-limit",
              "firstExtraCase": "browser-chrome-deadline-first-extra"
            },
            "bindingCaseIds": [
              "browser-node-public",
              "browser-chrome-public"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "component-replay",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::component_is_reproducible_and_retains_the_exact_core_and_world",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two emitted byte arrays equal; unchanged core; exact browser world"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-denied-import",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-denied-import",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::undeclared_component_import_is_rejected",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "undeclared import rejected before authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-substitution",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-substitution",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::substituted_wit_and_component_identity_fail_before_authority_is_returned",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "changed WIT and component identity reject"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "component-bytes-exact-limit",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete encoded component at inclusive byte ceiling accepted; first extra byte W4021"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-component-audit"
        },
        {
          "id": "component-bytes-first-extra",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete encoded component at inclusive byte ceiling accepted; first extra byte W4021"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-component-audit"
        },
        {
          "id": "component-interface-consistency",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-backend-webassembly/src/scalar_component/tests.rs"
          ],
          "gateId": "case-component-interface-consistency",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "scalar_component::tests::complete_component_bytes_accept_exact_interfaces_and_reject_the_first_extra",
            "source": "crates/zryna-backend-webassembly/src/scalar_component/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two expected exports accepted; same component with expected slice of one rejected; no maximum-exports claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "expected-interface-consistency"
        },
        {
          "id": "browser-node-public",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "case-browser-node-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_bundle_and_node_bindings_are_sealed",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "sealed generated bundle; scalar signed-i32 and typed rejection corpus"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-node-companion"
        },
        {
          "id": "browser-selector-denial",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "case-browser-selector-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_selector_rejects_other_targets_and_run_without_a_bundle",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "other targets and component run reject with no bundle"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-build"
        },
        {
          "id": "browser-replay",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "case-browser-replay",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_replays_deterministically_and_rejects_stale_binding",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "component/loader/declaration bytes equal across fresh stems; stale binding B3992"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-build"
        },
        {
          "id": "browser-chrome-public",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "checked-in page and generated loader; browser and Node corpus equality; no unexpected routes"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "pinned-chrome-and-node"
        },
        {
          "id": "browser-sealed-scalar-corpus",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "scalar_adapter_interface::tests::host_consumers::pinned_real_browser_and_node_execute_the_same_sealed_scalar_corpus",
            "source": "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "exact retained ESM source/interface/policy; pinned browser and Node same fixed corpus; cleanup confirmed"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "pinned-private-scalar-browser-and-node"
        },
        {
          "id": "browser-full-esm-transport",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "scalar_adapter_interface::tests::host_consumers::retained_esm_transport_accepts_exact_ceiling_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual full32MiB retained ESM transport accepted; first extra rejected; separate required ignored proof; not new public component/browser byte allowance"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-esm-transport-ceiling"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "browser-node-deadline-exact-limit",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "case-browser-node-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_bundle_and_node_bindings_are_sealed",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual retained component instantiate at30000; add20,22=42;30001 validation TypeErrorB3993; new draft corpus assertions executed by owning named CLI test"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "deadline",
            "value": 30000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-node-loader-option-admission"
        },
        {
          "id": "browser-node-deadline-first-extra",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "case-browser-node-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_bundle_and_node_bindings_are_sealed",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual retained component instantiate at30000; add20,22=42;30001 validation TypeErrorB3993; new draft corpus assertions executed by owning named CLI test"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "deadline",
            "value": 30001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-node-loader-option-admission"
        },
        {
          "id": "browser-node-minimum-admission",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "case-browser-node-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_bundle_and_node_bindings_are_sealed",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "deadline1 option admitted: either real completed instance or plain Error elapsedB3993; TypeError validation never counts; no1ms compilation-completion promise"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-node-loader-option-admission"
        },
        {
          "id": "browser-chrome-deadline-exact-limit",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual retained component instantiate at30000; add20,22=42;30001 validation TypeErrorB3993; new draft corpus assertions executed by owning named CLI test"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "deadline",
            "value": 30000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "pinned-chrome-loader-option-admission"
        },
        {
          "id": "browser-chrome-deadline-first-extra",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual retained component instantiate at30000; add20,22=42;30001 validation TypeErrorB3993; new draft corpus assertions executed by owning named CLI test"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "deadline",
            "value": 30001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "pinned-chrome-loader-option-admission"
        },
        {
          "id": "browser-chrome-minimum-admission",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/cli.rs",
            "tests/browser-component/run.mjs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "deadline1 option admitted: either real completed instance or plain Error elapsedB3993; TypeError validation never counts; no1ms compilation-completion promise"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "pinned-chrome-loader-option-admission"
        },
        {
          "id": "browser-public-canonical-component-exact-limit",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/fixtures/browser_byte_boundary.rs",
            "tests/browser-component/byte-boundary.mjs",
            "crates/zryna-backend-webassembly/src/scalar_component/tests/byte_boundary.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual public frontend source309364 bytes emits source-authenticated canonical1048576 with1863 ordinary exports; complete source/WIT/core/interface/component/manifest identity checks and fresh replay; changed literal source309365 bytes emits canonical1048577 and publicfactory W4021 exit4 before bundle publication; Node and pinnedChrome execute every exact export, reject the independent canonical first-extra B3991, and recover ordinary42; closed retained browserByteBoundary observation required"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-browser-source-authenticated-canonical-component-byte-boundary"
        },
        {
          "id": "browser-public-canonical-component-first-extra",
          "rowIds": [
            "browser-linux",
            "browser-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/fixtures/browser_byte_boundary.rs",
            "tests/browser-component/byte-boundary.mjs",
            "crates/zryna-backend-webassembly/src/scalar_component/tests/byte_boundary.rs"
          ],
          "gateId": "pinned-browser-three-proof",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "cli",
            "selector": "browser_component_pinned_real_browser_and_node_agree",
            "source": "apps/zryna/tests/cli.rs",
            "ignoredByDefault": true
          },
          "oracle": {
            "assertions": [
              "actual public frontend source309364 bytes emits source-authenticated canonical1048576 with1863 ordinary exports; complete source/WIT/core/interface/component/manifest identity checks and fresh replay; changed literal source309365 bytes emits canonical1048577 and publicfactory W4021 exit4 before bundle publication; Node and pinnedChrome execute every exact export, reject the independent canonical first-extra B3991, and recover ordinary42; closed retained browserByteBoundary observation required"
            ],
            "diagnosticCodes": [
              "ZRYNA-W4021",
              "ZRYNA-B3991"
            ]
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-browser-source-authenticated-canonical-component-byte-boundary"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-component-replay",
        "case-component-denied-import",
        "case-component-substitution",
        "case-component-bytes-exact-limit",
        "case-component-interface-consistency",
        "case-browser-node-public",
        "case-browser-selector-denial",
        "case-browser-replay",
        "pinned-browser-three-proof",
        "case-wit-closure"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces",
        "synchronous-guest-preemption"
      ]
    },
    {
      "id": "command-linux",
      "profile": "command-h1-v1",
      "mode": "checkout-run",
      "host": {
        "os": "linux",
        "architecture": "x64",
        "baseline": "ubuntu-latest:observed-baseline-required",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Wasmtime48.0.1"
      },
      "ownerIssues": [
        400
      ],
      "contract": "docs/WASI_COMMAND_MANIFEST_V1.md",
      "selector": {
        "command": "run",
        "profile": "command-h1-v1",
        "target": "wasi-command",
        "signature": "main():bool -> wasi:cli/run result<(),()>"
      },
      "identities": {
        "world": "zryna:capability-profiles/command@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.command-h1.host.v1",
        "manifest": "zryna.wasi-command-manifest.v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "languageSha256",
          "storageSha256",
          "linear32Sha256",
          "linuxX8664Sha256",
          "programBinding",
          "worldSha256"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [
          "empty-or-environment:get-environment:one-literal-key"
        ],
        "effective": [
          "empty-or-environment:get-environment:one-literal-key"
        ],
        "rootApprovals": [
          "private-literal-key-grant"
        ],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "monotonic-clock"
        ],
        "operationLimits": "one approved literal environment key; repeated reads do not deplete a quota; remaining world imports denied"
      },
      "artifacts": [
        {
          "path": "wasi-command/{stem}.wasm",
          "kind": "wasi-command-component-v1"
        },
        {
          "path": "zryna-wasi-command-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "command-component-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-component-policy-exact-limit",
              "firstExtraCase": "h1-private-component-policy-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "h1-public-cli-component-exact-limit",
              "firstExtraCase": "h1-public-cli-component-first-extra"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "request-bytes",
          "minimum": 0,
          "maximum": 4096,
          "unit": "bytes",
          "contractId": "command-request-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-request-bytes-exact-limit",
              "firstExtraCase": "command-request-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "key-bytes",
          "minimum": 1,
          "maximum": 64,
          "unit": "bytes",
          "contractId": "command-key-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-key-bytes-exact-limit",
              "firstExtraCase": "command-key-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "value-bytes",
          "minimum": 0,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "command-value-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-value-bytes-exact-limit",
              "firstExtraCase": "command-value-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "manifest-bytes",
          "minimum": 0,
          "maximum": 16384,
          "unit": "bytes",
          "contractId": "command-manifest-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-manifest-exact-limit",
              "firstExtraCase": "command-manifest-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "transfer-live",
          "minimum": 0,
          "maximum": 16,
          "unit": "count",
          "contractId": "command-transfer-live",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-live-exact-limit",
              "firstExtraCase": "command-transfer-live-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "transfer-allocations",
          "minimum": 0,
          "maximum": 4096,
          "unit": "count",
          "contractId": "command-transfer-allocations",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-allocations-exact-limit",
              "firstExtraCase": "command-transfer-allocations-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "transfer-allocation-bytes",
          "minimum": 0,
          "maximum": 4096,
          "unit": "bytes",
          "contractId": "command-transfer-allocation-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-allocation-bytes-exact-limit",
              "firstExtraCase": "command-transfer-allocation-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "transfer-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "command-transfer-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-bytes-exact-limit",
              "firstExtraCase": "command-transfer-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "fuel",
          "minimum": 0,
          "maximum": 100000,
          "unit": "fuel-units",
          "contractId": "command-fuel",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-fixed-fuel-enforcement"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "command-fuel-public-input-exclusion"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "deadline",
          "minimum": 1,
          "maximum": 5000,
          "unit": "milliseconds",
          "contractId": "command-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "command-deadline-cleanup"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "command-deadline-public-input-exclusion"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "wasm-stack",
          "minimum": 0,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "command-wasm-stack",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-private-stack-enforcement"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "command-wasm-stack-public-input-exclusion"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "backtrace-frames",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "command-backtrace-frames",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-backtrace-output-cap"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "command-backtrace-frames-public-input-exclusion"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "instances",
          "minimum": 0,
          "maximum": 2,
          "unit": "count",
          "contractId": "command-instances",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-instance-count-exact-limit",
              "firstExtraCase": "h1-private-instance-count-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "memories",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "command-memories",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-memory-objects-exact-limit",
              "firstExtraCase": "h1-memory-objects-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "tables",
          "minimum": 0,
          "maximum": 0,
          "unit": "count",
          "contractId": "command-tables",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-table-objects-exact-limit",
              "firstExtraCase": "h1-table-objects-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "memory-bytes",
          "minimum": 0,
          "maximum": 16777216,
          "unit": "bytes",
          "contractId": "command-memory-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-memory-byte-policy-exact-limit",
              "firstExtraCase": "h1-private-memory-byte-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "memory-pages",
          "minimum": 0,
          "maximum": 256,
          "unit": "pages",
          "contractId": "command-memory-pages",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-memory-pages-exact-limit",
              "firstExtraCase": "h1-private-memory-pages-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "command-public-pure",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-pure",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_pure_and_declared_error_commit_distinct_complete_command_bundles",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "pure ok and declared err complete bundles"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-public-input",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-input",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_found_missing_empty_and_utf8_values_are_private_and_released",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "found/missing/present-empty and UTF8; private input absent in published records; handles released"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-public-denials",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-denials",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_wrong_omitted_extra_grants_and_wrong_export_reject_without_bundle",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "wrong/omitted/extra grant and wrong export reject with no bundle"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-create-only",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "publication",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-create-only",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_create_only_retry_preserves_component_and_manifest_inventory",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "collision preserves complete prior component/manifest inventory"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-request-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::exact_file_limit_accepts_and_first_extra_byte_rejects",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 4096 UTF8 bytes accepted and 4097 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-bytes",
            "value": 4096,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-request-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::exact_file_limit_accepts_and_first_extra_byte_rejects",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 4096 UTF8 bytes accepted and 4097 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-bytes",
            "value": 4097,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-key-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-key-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::key_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 64 UTF8 bytes accepted and 65 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "key-bytes",
            "value": 64,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-key-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-key-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::key_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 64 UTF8 bytes accepted and 65 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "key-bytes",
            "value": 65,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-value-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::present_value_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 1024 UTF8 bytes accepted and 1025 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "value-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-value-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::present_value_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 1024 UTF8 bytes accepted and 1025 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "value-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-execution",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs"
          ],
          "gateId": "case-command-value-execution",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::execution::actual_found_empty_ascii_multibyte_and_exact_1024_bytes_run_ok",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "present-empty/ASCII/multibyte/exact1024 runs WIT ok; teardown confirmed; private handles released"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-first-extra-pre-engine",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs"
          ],
          "gateId": "case-command-first-extra-pre-engine",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::preparation::first_extra_value_byte_and_nonprivate_input_reject_before_engine",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "1025-byte value rejected D4101 before consuming run; fresh valid recovery"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-admission"
        },
        {
          "id": "command-denied-callbacks",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs"
          ],
          "gateId": "case-command-denied-callbacks",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::denied_probes::actual_denied_capability_matrix_seals_first_callback_and_consumes_each_store",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "each admitted-world denied callback seals first denial and consumes actual Store"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "command-grant-does-not-broaden",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs"
          ],
          "gateId": "case-command-grant-does-not-broaden",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::denied_probes::actual_environment_grant_does_not_approve_other_capability_callbacks",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "environment approval does not admit clocks/filesystem/network/randomness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "command-deadline-cleanup",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs"
          ],
          "gateId": "case-command-deadline-cleanup",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::deadline::actual_epoch_deadline_traps_with_no_identity_and_confirmed_teardown",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "real epoch expiry traps; no fabricated trap identity; Store consumed and deadline worker joined; fresh recovery"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-unwind-no-certificate",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs"
          ],
          "gateId": "case-command-unwind-no-certificate",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::deadline::actual_host_unwind_has_no_run_or_identity_and_never_confirms_teardown",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "host unwind reports unconfirmed teardown; no successful run or controlled identity"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-transfer-live-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-live-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_live_limit_counts_old_and_prospective_resize",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 16 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-live",
            "value": 16,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-live-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-live-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_live_limit_counts_old_and_prospective_resize",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 16 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-live",
            "value": 17,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocations-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocations-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_count_survives_full_drain_and_rejects_first_extra",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocations",
            "value": 4096,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocations-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocations-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_count_survives_full_drain_and_rejects_first_extra",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocations",
            "value": 4097,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocation-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocation-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_byte_limit_and_wrapping_sizes_trap",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocation-bytes",
            "value": 4096,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocation-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocation-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_byte_limit_and_wrapping_sizes_trap",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocation-bytes",
            "value": 4097,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_arena_counts_holes_and_exact_last_byte",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 1048576 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_arena_counts_holes_and_exact_last_byte",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 1048576 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-manifest-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs"
          ],
          "gateId": "case-command-manifest-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::manifest::tests::rejection::strict_utf8_surrogates_trailing_records_and_exact_manifest_byte_bound_reject_and_recover",
            "source": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "strict decoder inclusive whitespace-padded valid record16384 accepted;16385 rejected; decoding grants no execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "manifest-bytes",
            "value": 16384,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-manifest-decoder"
        },
        {
          "id": "command-manifest-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs"
          ],
          "gateId": "case-command-manifest-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::manifest::tests::rejection::strict_utf8_surrogates_trailing_records_and_exact_manifest_byte_bound_reject_and_recover",
            "source": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "strict decoder inclusive whitespace-padded valid record16384 accepted;16385 rejected; decoding grants no execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "manifest-bytes",
            "value": 16385,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-manifest-decoder"
        },
        {
          "id": "command-component-replay",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs"
          ],
          "gateId": "case-command-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::tests::immutable_artifact_binding_is_deterministic_across_fresh_issuers_and_wit_input_order",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "fresh issuer identity and reversed WIT input order preserve exact artifact bytes and bindings"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "h1-private-memory-pages-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-memory-pages-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualrawcore256pages;declaredmax257permitsstructurally; host Memory::grow firstextra page rejected by unchanged16MiBlimiter;freshStore;not guestmemory.grow orbyte+1"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-pages",
            "value": 256,
            "unit": "pages",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-host-Memory-grow"
        },
        {
          "id": "h1-private-memory-pages-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-memory-pages-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualrawcore256pages;declaredmax257permitsstructurally; host Memory::grow firstextra page rejected by unchanged16MiBlimiter;freshStore;not guestmemory.grow orbyte+1"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-pages",
            "value": 257,
            "unit": "pages",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-host-Memory-grow"
        },
        {
          "id": "h1-private-memory-byte-policy-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-h1-private-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::memory_policy::private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualproduction StoreLimits memory_growing directpolicy callback accepts16777216/rejects16777217;freshlimiter; no guestallocation ormemory.grow execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-bytes",
            "value": 16777216,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-production-ResourceLimiter-memory-policy"
        },
        {
          "id": "h1-private-memory-byte-policy-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-h1-private-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::memory_policy::private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualproduction StoreLimits memory_growing directpolicy callback accepts16777216/rejects16777217;freshlimiter; no guestallocation ormemory.grow execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-bytes",
            "value": 16777217,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-production-ResourceLimiter-memory-policy"
        },
        {
          "id": "h1-private-instance-count-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-instance-count-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_instance_limit_accepts_two_and_rejects_third",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two actual emptycoreinstances under unchanged StoreLimits;thirdrejects;freshStore;not publicH1artifact"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "instances",
            "value": 2,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-instances"
        },
        {
          "id": "h1-private-instance-count-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-instance-count-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_instance_limit_accepts_two_and_rejects_third",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two actual emptycoreinstances under unchanged StoreLimits;thirdrejects;freshStore;not publicH1artifact"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "instances",
            "value": 3,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-instances"
        },
        {
          "id": "h1-private-stack-enforcement",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs"
          ],
          "gateId": "case-h1-private-stack-enforcement",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::stack_limits::private_h1_fixed_stack_traps_recursive_core_with_fuel_remaining_and_recovers",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "rawrecursivecore actual StackOverflow with fuelremaining under fixedstack65536; consumedStore/freshsealedH1recovery; NO nativebyteoccupancy65536/65537 oracle; publicrecursionexcluded"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-stack-trap"
        },
        {
          "id": "h1-memory-objects-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-memory-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_one_memory_object_admits_one_and_rejects_second_across_instances",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memories",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-memory-objects-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-memory-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_one_memory_object_admits_one_and_rejects_second_across_instances",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memories",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-table-objects-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-table-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "tables",
            "value": 0,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-table-objects-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-table-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "tables",
            "value": 1,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-backtrace-output-cap",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs"
          ],
          "gateId": "case-h1-backtrace-output-cap",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::backtrace_limits::private_h1_one_frame_backtrace_retains_inner_trap_and_truncates_extra_caller",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inner Unreachable retains one diagnostic frame under productioncap1; extra source caller still traps with one diagnostic frame; not second-call rejection or physical native frame occupancy"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-diagnostic-frame-truncation"
        },
        {
          "id": "h1-fixed-fuel-enforcement",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-h1-fixed-fuel-enforcement",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::fuel_limits::private_h1_fixed_fuel100000_measured_exhaustion_and_next_call_rejection",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged initialFUEL100000; actual get_fuel cost1;99999 completed calls leave1; final charged call OutOfFuel at0; next call OutOfFuel at0; fresh public profile recovery; no100000successfulcalls or caller budget100001 admission claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-measured-fixed-fuel-enforcement"
        },
        {
          "id": "h1-private-component-policy-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs"
          ],
          "gateId": "case-h1-private-component-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::component_audit::tests::byte_policy::private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private actual full validgraph1048576 audits owning byte guard;1048577 rejects; independent language_audit rejects corepadding W4103; not public authenticated H1 canonical accepted1MiB"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-H1-whole-graph-component-byte-policy-language-denies-padding"
        },
        {
          "id": "h1-private-component-policy-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs"
          ],
          "gateId": "case-h1-private-component-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::component_audit::tests::byte_policy::private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private actual full validgraph1048576 audits owning byte guard;1048577 rejects; independent language_audit rejects corepadding W4103; not public authenticated H1 canonical accepted1MiB"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-H1-whole-graph-component-byte-policy-language-denies-padding"
        },
        {
          "id": "h1-public-component-factory-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-component-factory-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::byte_boundary::public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "authenticated actual publicH1 source/verifiedIR/factory emits canonical1048576; issuer/sourceSHA/completegraph/languageaudits and canonicalrevalidation; one sourcebyte signedLEB change emits independentcanonical1048577; ownerguardW4104/factoryW4102 rejects; recovery; shared factorycapacity proof, not installed1MiB runtime claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-H1-source-authenticated-canonical-factory-boundary"
        },
        {
          "id": "h1-public-component-factory-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-component-factory-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::byte_boundary::public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "authenticated actual publicH1 source/verifiedIR/factory emits canonical1048576; issuer/sourceSHA/completegraph/languageaudits and canonicalrevalidation; one sourcebyte signedLEB change emits independentcanonical1048577; ownerguardW4104/factoryW4102 rejects; recovery; shared factorycapacity proof, not installed1MiB runtime claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-H1-source-authenticated-canonical-factory-boundary"
        },
        {
          "id": "h1-public-cli-component-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_command/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-cli-component-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "byte_boundary::actual_cli_h1_accepts1048576_and_rejects_source_emitted1048577_before_execution",
            "source": "apps/zryna/tests/wasi_command/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual frontend/publicCLI authored87161byte H1source; canonicalcomponent1048576/sourcehash/program/composition/WIT34/grants[]/quotas0;runok/confirmedteardown; same-source-length onebytechange emits1048577 W4102 exit3 with no manifest/bundle/execution; ordinary same-case recovery; checkout runtime only"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-checkout-H1-source-authenticated-canonical-ceiling-execution"
        },
        {
          "id": "h1-public-cli-component-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_command/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-cli-component-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "byte_boundary::actual_cli_h1_accepts1048576_and_rejects_source_emitted1048577_before_execution",
            "source": "apps/zryna/tests/wasi_command/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual frontend/publicCLI authored87161byte H1source; canonicalcomponent1048576/sourcehash/program/composition/WIT34/grants[]/quotas0;runok/confirmedteardown; same-source-length onebytechange emits1048577 W4102 exit3 with no manifest/bundle/execution; ordinary same-case recovery; checkout runtime only"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-checkout-H1-source-authenticated-canonical-ceiling-execution"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-command-public-pure",
        "case-command-public-input",
        "case-command-public-denials",
        "case-command-create-only",
        "case-command-request-bytes-exact-limit",
        "case-command-key-bytes-exact-limit",
        "case-command-value-bytes-exact-limit",
        "case-command-value-execution",
        "case-command-first-extra-pre-engine",
        "case-command-denied-callbacks",
        "case-command-grant-does-not-broaden",
        "case-command-deadline-cleanup",
        "case-command-unwind-no-certificate",
        "case-command-transfer-live-exact-limit",
        "case-command-transfer-allocations-exact-limit",
        "case-command-transfer-allocation-bytes-exact-limit",
        "case-command-transfer-bytes-exact-limit",
        "case-command-manifest-exact-limit",
        "case-command-component-replay",
        "case-wit-closure",
        "case-h1-private-memory-pages-exact-limit",
        "case-h1-private-memory-byte-policy-exact-limit",
        "case-h1-private-instance-count-exact-limit",
        "case-h1-private-stack-enforcement",
        "case-h1-memory-objects-exact-limit",
        "case-h1-table-objects-exact-limit",
        "case-h1-backtrace-output-cap",
        "case-h1-fixed-fuel-enforcement",
        "case-h1-private-component-policy-exact-limit",
        "case-h1-public-component-factory-exact-limit",
        "case-h1-public-cli-component-exact-limit"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces"
      ]
    },
    {
      "id": "command-windows",
      "profile": "command-h1-v1",
      "mode": "checkout-run",
      "host": {
        "os": "win32",
        "architecture": "x64",
        "baseline": "windows-latest:observed-baseline-required",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Wasmtime48.0.1"
      },
      "ownerIssues": [
        400
      ],
      "contract": "docs/WASI_COMMAND_MANIFEST_V1.md",
      "selector": {
        "command": "run",
        "profile": "command-h1-v1",
        "target": "wasi-command",
        "signature": "main():bool -> wasi:cli/run result<(),()>"
      },
      "identities": {
        "world": "zryna:capability-profiles/command@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.command-h1.host.v1",
        "manifest": "zryna.wasi-command-manifest.v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "languageSha256",
          "storageSha256",
          "linear32Sha256",
          "linuxX8664Sha256",
          "programBinding",
          "worldSha256"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [
          "empty-or-environment:get-environment:one-literal-key"
        ],
        "effective": [
          "empty-or-environment:get-environment:one-literal-key"
        ],
        "rootApprovals": [
          "private-literal-key-grant"
        ],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "monotonic-clock"
        ],
        "operationLimits": "one approved literal environment key; repeated reads do not deplete a quota; remaining world imports denied"
      },
      "artifacts": [
        {
          "path": "wasi-command/{stem}.wasm",
          "kind": "wasi-command-component-v1"
        },
        {
          "path": "zryna-wasi-command-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "command-component-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-component-policy-exact-limit",
              "firstExtraCase": "h1-private-component-policy-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "h1-public-cli-component-exact-limit",
              "firstExtraCase": "h1-public-cli-component-first-extra"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "request-bytes",
          "minimum": 0,
          "maximum": 4096,
          "unit": "bytes",
          "contractId": "command-request-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-request-bytes-exact-limit",
              "firstExtraCase": "command-request-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "key-bytes",
          "minimum": 1,
          "maximum": 64,
          "unit": "bytes",
          "contractId": "command-key-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-key-bytes-exact-limit",
              "firstExtraCase": "command-key-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "value-bytes",
          "minimum": 0,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "command-value-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-value-bytes-exact-limit",
              "firstExtraCase": "command-value-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "manifest-bytes",
          "minimum": 0,
          "maximum": 16384,
          "unit": "bytes",
          "contractId": "command-manifest-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-manifest-exact-limit",
              "firstExtraCase": "command-manifest-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "transfer-live",
          "minimum": 0,
          "maximum": 16,
          "unit": "count",
          "contractId": "command-transfer-live",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-live-exact-limit",
              "firstExtraCase": "command-transfer-live-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "transfer-allocations",
          "minimum": 0,
          "maximum": 4096,
          "unit": "count",
          "contractId": "command-transfer-allocations",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-allocations-exact-limit",
              "firstExtraCase": "command-transfer-allocations-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "transfer-allocation-bytes",
          "minimum": 0,
          "maximum": 4096,
          "unit": "bytes",
          "contractId": "command-transfer-allocation-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-allocation-bytes-exact-limit",
              "firstExtraCase": "command-transfer-allocation-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "transfer-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "command-transfer-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-bytes-exact-limit",
              "firstExtraCase": "command-transfer-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "fuel",
          "minimum": 0,
          "maximum": 100000,
          "unit": "fuel-units",
          "contractId": "command-fuel",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-fixed-fuel-enforcement"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "command-fuel-public-input-exclusion"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "deadline",
          "minimum": 1,
          "maximum": 5000,
          "unit": "milliseconds",
          "contractId": "command-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "command-deadline-cleanup"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "command-deadline-public-input-exclusion"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "wasm-stack",
          "minimum": 0,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "command-wasm-stack",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-private-stack-enforcement"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "command-wasm-stack-public-input-exclusion"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "backtrace-frames",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "command-backtrace-frames",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-backtrace-output-cap"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "command-backtrace-frames-public-input-exclusion"
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "instances",
          "minimum": 0,
          "maximum": 2,
          "unit": "count",
          "contractId": "command-instances",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-instance-count-exact-limit",
              "firstExtraCase": "h1-private-instance-count-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "memories",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "command-memories",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-memory-objects-exact-limit",
              "firstExtraCase": "h1-memory-objects-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "tables",
          "minimum": 0,
          "maximum": 0,
          "unit": "count",
          "contractId": "command-tables",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-table-objects-exact-limit",
              "firstExtraCase": "h1-table-objects-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "memory-bytes",
          "minimum": 0,
          "maximum": 16777216,
          "unit": "bytes",
          "contractId": "command-memory-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-memory-byte-policy-exact-limit",
              "firstExtraCase": "h1-private-memory-byte-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        },
        {
          "id": "memory-pages",
          "minimum": 0,
          "maximum": 256,
          "unit": "pages",
          "contractId": "command-memory-pages",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-memory-pages-exact-limit",
              "firstExtraCase": "h1-private-memory-pages-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "command-public-pure",
                "command-public-input"
              ]
            },
            "bindingCaseIds": [
              "command-public-pure",
              "command-public-input"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "command-public-pure",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-pure",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_pure_and_declared_error_commit_distinct_complete_command_bundles",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "pure ok and declared err complete bundles"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-public-input",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-input",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_found_missing_empty_and_utf8_values_are_private_and_released",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "found/missing/present-empty and UTF8; private input absent in published records; handles released"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-public-denials",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-denials",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_wrong_omitted_extra_grants_and_wrong_export_reject_without_bundle",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "wrong/omitted/extra grant and wrong export reject with no bundle"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-create-only",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "publication",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-create-only",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_create_only_retry_preserves_component_and_manifest_inventory",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "collision preserves complete prior component/manifest inventory"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-request-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::exact_file_limit_accepts_and_first_extra_byte_rejects",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 4096 UTF8 bytes accepted and 4097 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-bytes",
            "value": 4096,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-request-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::exact_file_limit_accepts_and_first_extra_byte_rejects",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 4096 UTF8 bytes accepted and 4097 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-bytes",
            "value": 4097,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-key-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-key-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::key_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 64 UTF8 bytes accepted and 65 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "key-bytes",
            "value": 64,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-key-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-key-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::key_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 64 UTF8 bytes accepted and 65 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "key-bytes",
            "value": 65,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-value-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::present_value_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 1024 UTF8 bytes accepted and 1025 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "value-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-value-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::present_value_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 1024 UTF8 bytes accepted and 1025 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "value-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-execution",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs"
          ],
          "gateId": "case-command-value-execution",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::execution::actual_found_empty_ascii_multibyte_and_exact_1024_bytes_run_ok",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "present-empty/ASCII/multibyte/exact1024 runs WIT ok; teardown confirmed; private handles released"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-first-extra-pre-engine",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs"
          ],
          "gateId": "case-command-first-extra-pre-engine",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::preparation::first_extra_value_byte_and_nonprivate_input_reject_before_engine",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "1025-byte value rejected D4101 before consuming run; fresh valid recovery"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-admission"
        },
        {
          "id": "command-denied-callbacks",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs"
          ],
          "gateId": "case-command-denied-callbacks",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::denied_probes::actual_denied_capability_matrix_seals_first_callback_and_consumes_each_store",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "each admitted-world denied callback seals first denial and consumes actual Store"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "command-grant-does-not-broaden",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs"
          ],
          "gateId": "case-command-grant-does-not-broaden",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::denied_probes::actual_environment_grant_does_not_approve_other_capability_callbacks",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "environment approval does not admit clocks/filesystem/network/randomness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "command-deadline-cleanup",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs"
          ],
          "gateId": "case-command-deadline-cleanup",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::deadline::actual_epoch_deadline_traps_with_no_identity_and_confirmed_teardown",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "real epoch expiry traps; no fabricated trap identity; Store consumed and deadline worker joined; fresh recovery"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-unwind-no-certificate",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs"
          ],
          "gateId": "case-command-unwind-no-certificate",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::deadline::actual_host_unwind_has_no_run_or_identity_and_never_confirms_teardown",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "host unwind reports unconfirmed teardown; no successful run or controlled identity"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-transfer-live-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-live-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_live_limit_counts_old_and_prospective_resize",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 16 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-live",
            "value": 16,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-live-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-live-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_live_limit_counts_old_and_prospective_resize",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 16 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-live",
            "value": 17,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocations-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocations-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_count_survives_full_drain_and_rejects_first_extra",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocations",
            "value": 4096,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocations-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocations-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_count_survives_full_drain_and_rejects_first_extra",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocations",
            "value": 4097,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocation-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocation-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_byte_limit_and_wrapping_sizes_trap",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocation-bytes",
            "value": 4096,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocation-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocation-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_byte_limit_and_wrapping_sizes_trap",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocation-bytes",
            "value": 4097,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_arena_counts_holes_and_exact_last_byte",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 1048576 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_arena_counts_holes_and_exact_last_byte",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 1048576 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-manifest-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs"
          ],
          "gateId": "case-command-manifest-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::manifest::tests::rejection::strict_utf8_surrogates_trailing_records_and_exact_manifest_byte_bound_reject_and_recover",
            "source": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "strict decoder inclusive whitespace-padded valid record16384 accepted;16385 rejected; decoding grants no execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "manifest-bytes",
            "value": 16384,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-manifest-decoder"
        },
        {
          "id": "command-manifest-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs"
          ],
          "gateId": "case-command-manifest-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::manifest::tests::rejection::strict_utf8_surrogates_trailing_records_and_exact_manifest_byte_bound_reject_and_recover",
            "source": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "strict decoder inclusive whitespace-padded valid record16384 accepted;16385 rejected; decoding grants no execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "manifest-bytes",
            "value": 16385,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-manifest-decoder"
        },
        {
          "id": "command-component-replay",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs"
          ],
          "gateId": "case-command-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::tests::immutable_artifact_binding_is_deterministic_across_fresh_issuers_and_wit_input_order",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "fresh issuer identity and reversed WIT input order preserve exact artifact bytes and bindings"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "h1-private-memory-pages-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-memory-pages-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualrawcore256pages;declaredmax257permitsstructurally; host Memory::grow firstextra page rejected by unchanged16MiBlimiter;freshStore;not guestmemory.grow orbyte+1"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-pages",
            "value": 256,
            "unit": "pages",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-host-Memory-grow"
        },
        {
          "id": "h1-private-memory-pages-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-memory-pages-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualrawcore256pages;declaredmax257permitsstructurally; host Memory::grow firstextra page rejected by unchanged16MiBlimiter;freshStore;not guestmemory.grow orbyte+1"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-pages",
            "value": 257,
            "unit": "pages",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-host-Memory-grow"
        },
        {
          "id": "h1-private-memory-byte-policy-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-h1-private-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::memory_policy::private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualproduction StoreLimits memory_growing directpolicy callback accepts16777216/rejects16777217;freshlimiter; no guestallocation ormemory.grow execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-bytes",
            "value": 16777216,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-production-ResourceLimiter-memory-policy"
        },
        {
          "id": "h1-private-memory-byte-policy-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-h1-private-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::memory_policy::private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualproduction StoreLimits memory_growing directpolicy callback accepts16777216/rejects16777217;freshlimiter; no guestallocation ormemory.grow execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-bytes",
            "value": 16777217,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-production-ResourceLimiter-memory-policy"
        },
        {
          "id": "h1-private-instance-count-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-instance-count-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_instance_limit_accepts_two_and_rejects_third",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two actual emptycoreinstances under unchanged StoreLimits;thirdrejects;freshStore;not publicH1artifact"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "instances",
            "value": 2,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-instances"
        },
        {
          "id": "h1-private-instance-count-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-instance-count-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_instance_limit_accepts_two_and_rejects_third",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two actual emptycoreinstances under unchanged StoreLimits;thirdrejects;freshStore;not publicH1artifact"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "instances",
            "value": 3,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-instances"
        },
        {
          "id": "h1-private-stack-enforcement",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs"
          ],
          "gateId": "case-h1-private-stack-enforcement",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::stack_limits::private_h1_fixed_stack_traps_recursive_core_with_fuel_remaining_and_recovers",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "rawrecursivecore actual StackOverflow with fuelremaining under fixedstack65536; consumedStore/freshsealedH1recovery; NO nativebyteoccupancy65536/65537 oracle; publicrecursionexcluded"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-stack-trap"
        },
        {
          "id": "h1-memory-objects-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-memory-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_one_memory_object_admits_one_and_rejects_second_across_instances",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memories",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-memory-objects-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-memory-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_one_memory_object_admits_one_and_rejects_second_across_instances",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memories",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-table-objects-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-table-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "tables",
            "value": 0,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-table-objects-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-table-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "tables",
            "value": 1,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-backtrace-output-cap",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs"
          ],
          "gateId": "case-h1-backtrace-output-cap",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::backtrace_limits::private_h1_one_frame_backtrace_retains_inner_trap_and_truncates_extra_caller",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inner Unreachable retains one diagnostic frame under productioncap1; extra source caller still traps with one diagnostic frame; not second-call rejection or physical native frame occupancy"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-diagnostic-frame-truncation"
        },
        {
          "id": "h1-fixed-fuel-enforcement",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-h1-fixed-fuel-enforcement",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::fuel_limits::private_h1_fixed_fuel100000_measured_exhaustion_and_next_call_rejection",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged initialFUEL100000; actual get_fuel cost1;99999 completed calls leave1; final charged call OutOfFuel at0; next call OutOfFuel at0; fresh public profile recovery; no100000successfulcalls or caller budget100001 admission claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-measured-fixed-fuel-enforcement"
        },
        {
          "id": "h1-private-component-policy-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs"
          ],
          "gateId": "case-h1-private-component-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::component_audit::tests::byte_policy::private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private actual full validgraph1048576 audits owning byte guard;1048577 rejects; independent language_audit rejects corepadding W4103; not public authenticated H1 canonical accepted1MiB"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-H1-whole-graph-component-byte-policy-language-denies-padding"
        },
        {
          "id": "h1-private-component-policy-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs"
          ],
          "gateId": "case-h1-private-component-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::component_audit::tests::byte_policy::private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private actual full validgraph1048576 audits owning byte guard;1048577 rejects; independent language_audit rejects corepadding W4103; not public authenticated H1 canonical accepted1MiB"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-H1-whole-graph-component-byte-policy-language-denies-padding"
        },
        {
          "id": "h1-public-component-factory-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-component-factory-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::byte_boundary::public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "authenticated actual publicH1 source/verifiedIR/factory emits canonical1048576; issuer/sourceSHA/completegraph/languageaudits and canonicalrevalidation; one sourcebyte signedLEB change emits independentcanonical1048577; ownerguardW4104/factoryW4102 rejects; recovery; shared factorycapacity proof, not installed1MiB runtime claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-H1-source-authenticated-canonical-factory-boundary"
        },
        {
          "id": "h1-public-component-factory-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-component-factory-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::byte_boundary::public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "authenticated actual publicH1 source/verifiedIR/factory emits canonical1048576; issuer/sourceSHA/completegraph/languageaudits and canonicalrevalidation; one sourcebyte signedLEB change emits independentcanonical1048577; ownerguardW4104/factoryW4102 rejects; recovery; shared factorycapacity proof, not installed1MiB runtime claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-H1-source-authenticated-canonical-factory-boundary"
        },
        {
          "id": "h1-public-cli-component-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_command/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-cli-component-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "byte_boundary::actual_cli_h1_accepts1048576_and_rejects_source_emitted1048577_before_execution",
            "source": "apps/zryna/tests/wasi_command/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual frontend/publicCLI authored87161byte H1source; canonicalcomponent1048576/sourcehash/program/composition/WIT34/grants[]/quotas0;runok/confirmedteardown; same-source-length onebytechange emits1048577 W4102 exit3 with no manifest/bundle/execution; ordinary same-case recovery; checkout runtime only"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-checkout-H1-source-authenticated-canonical-ceiling-execution"
        },
        {
          "id": "h1-public-cli-component-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_command/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-cli-component-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "byte_boundary::actual_cli_h1_accepts1048576_and_rejects_source_emitted1048577_before_execution",
            "source": "apps/zryna/tests/wasi_command/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual frontend/publicCLI authored87161byte H1source; canonicalcomponent1048576/sourcehash/program/composition/WIT34/grants[]/quotas0;runok/confirmedteardown; same-source-length onebytechange emits1048577 W4102 exit3 with no manifest/bundle/execution; ordinary same-case recovery; checkout runtime only"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-checkout-H1-source-authenticated-canonical-ceiling-execution"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-command-public-pure",
        "case-command-public-input",
        "case-command-public-denials",
        "case-command-create-only",
        "case-command-request-bytes-exact-limit",
        "case-command-key-bytes-exact-limit",
        "case-command-value-bytes-exact-limit",
        "case-command-value-execution",
        "case-command-first-extra-pre-engine",
        "case-command-denied-callbacks",
        "case-command-grant-does-not-broaden",
        "case-command-deadline-cleanup",
        "case-command-unwind-no-certificate",
        "case-command-transfer-live-exact-limit",
        "case-command-transfer-allocations-exact-limit",
        "case-command-transfer-allocation-bytes-exact-limit",
        "case-command-transfer-bytes-exact-limit",
        "case-command-manifest-exact-limit",
        "case-command-component-replay",
        "case-wit-closure",
        "case-h1-private-memory-pages-exact-limit",
        "case-h1-private-memory-byte-policy-exact-limit",
        "case-h1-private-instance-count-exact-limit",
        "case-h1-private-stack-enforcement",
        "case-h1-memory-objects-exact-limit",
        "case-h1-table-objects-exact-limit",
        "case-h1-backtrace-output-cap",
        "case-h1-fixed-fuel-enforcement",
        "case-h1-private-component-policy-exact-limit",
        "case-h1-public-component-factory-exact-limit",
        "case-h1-public-cli-component-exact-limit"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces"
      ]
    },
    {
      "id": "installed-command-linux",
      "profile": "command-h1-v1",
      "mode": "fresh-content-verified-installation",
      "host": {
        "os": "linux",
        "architecture": "x64",
        "baseline": "ubuntu-24.04",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Wasmtime48.0.1"
      },
      "ownerIssues": [
        400
      ],
      "contract": "docs/WASI_COMMAND_MANIFEST_V1.md",
      "selector": {
        "command": "run",
        "profile": "command-h1-v1",
        "target": "wasi-command",
        "signature": "main():bool -> wasi:cli/run result<(),()>"
      },
      "identities": {
        "world": "zryna:capability-profiles/command@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.command-h1.host.v1",
        "manifest": "zryna.wasi-command-manifest.v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "languageSha256",
          "storageSha256",
          "linear32Sha256",
          "linuxX8664Sha256",
          "programBinding",
          "worldSha256"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [
          "empty-or-environment:get-environment:one-literal-key"
        ],
        "effective": [
          "empty-or-environment:get-environment:one-literal-key"
        ],
        "rootApprovals": [
          "private-literal-key-grant"
        ],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "monotonic-clock"
        ],
        "operationLimits": "one approved literal environment key; repeated reads do not deplete a quota; remaining world imports denied"
      },
      "artifacts": [
        {
          "path": "wasi-command/{stem}.wasm",
          "kind": "wasi-command-component-v1"
        },
        {
          "path": "zryna-wasi-command-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "installed-command-component-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-component-policy-exact-limit",
              "firstExtraCase": "h1-private-component-policy-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "installed-component-bytes-exact-limit",
              "firstExtraCase": "installed-component-bytes-first-extra"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "request-bytes",
          "minimum": 0,
          "maximum": 4096,
          "unit": "bytes",
          "contractId": "installed-command-request-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-request-bytes-exact-limit",
              "firstExtraCase": "command-request-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "key-bytes",
          "minimum": 1,
          "maximum": 64,
          "unit": "bytes",
          "contractId": "installed-command-key-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-key-bytes-exact-limit",
              "firstExtraCase": "command-key-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "value-bytes",
          "minimum": 0,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "installed-command-value-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-value-bytes-exact-limit",
              "firstExtraCase": "command-value-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "manifest-bytes",
          "minimum": 0,
          "maximum": 16384,
          "unit": "bytes",
          "contractId": "installed-command-manifest-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-manifest-exact-limit",
              "firstExtraCase": "command-manifest-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "transfer-live",
          "minimum": 0,
          "maximum": 16,
          "unit": "count",
          "contractId": "installed-command-transfer-live",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-live-exact-limit",
              "firstExtraCase": "command-transfer-live-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "transfer-allocations",
          "minimum": 0,
          "maximum": 4096,
          "unit": "count",
          "contractId": "installed-command-transfer-allocations",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-allocations-exact-limit",
              "firstExtraCase": "command-transfer-allocations-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "transfer-allocation-bytes",
          "minimum": 0,
          "maximum": 4096,
          "unit": "bytes",
          "contractId": "installed-command-transfer-allocation-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-allocation-bytes-exact-limit",
              "firstExtraCase": "command-transfer-allocation-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "transfer-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "installed-command-transfer-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-bytes-exact-limit",
              "firstExtraCase": "command-transfer-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "fuel",
          "minimum": 0,
          "maximum": 100000,
          "unit": "fuel-units",
          "contractId": "installed-command-fuel",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-fixed-fuel-enforcement"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "installed-command-fuel-public-input-exclusion"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "deadline",
          "minimum": 1,
          "maximum": 5000,
          "unit": "milliseconds",
          "contractId": "installed-command-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "command-deadline-cleanup"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "installed-command-deadline-public-input-exclusion"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "wasm-stack",
          "minimum": 0,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "installed-command-wasm-stack",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-private-stack-enforcement"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "installed-command-wasm-stack-public-input-exclusion"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "backtrace-frames",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "installed-command-backtrace-frames",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-backtrace-output-cap"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "installed-command-backtrace-frames-public-input-exclusion"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "instances",
          "minimum": 0,
          "maximum": 2,
          "unit": "count",
          "contractId": "installed-command-instances",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-instance-count-exact-limit",
              "firstExtraCase": "h1-private-instance-count-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "memories",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "installed-command-memories",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-memory-objects-exact-limit",
              "firstExtraCase": "h1-memory-objects-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "tables",
          "minimum": 0,
          "maximum": 0,
          "unit": "count",
          "contractId": "installed-command-tables",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-table-objects-exact-limit",
              "firstExtraCase": "h1-table-objects-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "memory-bytes",
          "minimum": 0,
          "maximum": 16777216,
          "unit": "bytes",
          "contractId": "installed-command-memory-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-memory-byte-policy-exact-limit",
              "firstExtraCase": "h1-private-memory-byte-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "memory-pages",
          "minimum": 0,
          "maximum": 256,
          "unit": "pages",
          "contractId": "installed-command-memory-pages",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-memory-pages-exact-limit",
              "firstExtraCase": "h1-private-memory-pages-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "command-public-pure",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-pure",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_pure_and_declared_error_commit_distinct_complete_command_bundles",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "pure ok and declared err complete bundles"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-public-input",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-input",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_found_missing_empty_and_utf8_values_are_private_and_released",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "found/missing/present-empty and UTF8; private input absent in published records; handles released"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-public-denials",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-denials",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_wrong_omitted_extra_grants_and_wrong_export_reject_without_bundle",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "wrong/omitted/extra grant and wrong export reject with no bundle"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-create-only",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "publication",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-create-only",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_create_only_retry_preserves_component_and_manifest_inventory",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "collision preserves complete prior component/manifest inventory"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-request-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::exact_file_limit_accepts_and_first_extra_byte_rejects",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 4096 UTF8 bytes accepted and 4097 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-bytes",
            "value": 4096,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-request-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::exact_file_limit_accepts_and_first_extra_byte_rejects",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 4096 UTF8 bytes accepted and 4097 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-bytes",
            "value": 4097,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-key-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-key-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::key_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 64 UTF8 bytes accepted and 65 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "key-bytes",
            "value": 64,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-key-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-key-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::key_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 64 UTF8 bytes accepted and 65 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "key-bytes",
            "value": 65,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-value-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::present_value_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 1024 UTF8 bytes accepted and 1025 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "value-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-value-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::present_value_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 1024 UTF8 bytes accepted and 1025 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "value-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-execution",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs"
          ],
          "gateId": "case-command-value-execution",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::execution::actual_found_empty_ascii_multibyte_and_exact_1024_bytes_run_ok",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "present-empty/ASCII/multibyte/exact1024 runs WIT ok; teardown confirmed; private handles released"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-first-extra-pre-engine",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs"
          ],
          "gateId": "case-command-first-extra-pre-engine",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::preparation::first_extra_value_byte_and_nonprivate_input_reject_before_engine",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "1025-byte value rejected D4101 before consuming run; fresh valid recovery"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-admission"
        },
        {
          "id": "command-denied-callbacks",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs"
          ],
          "gateId": "case-command-denied-callbacks",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::denied_probes::actual_denied_capability_matrix_seals_first_callback_and_consumes_each_store",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "each admitted-world denied callback seals first denial and consumes actual Store"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "command-grant-does-not-broaden",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs"
          ],
          "gateId": "case-command-grant-does-not-broaden",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::denied_probes::actual_environment_grant_does_not_approve_other_capability_callbacks",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "environment approval does not admit clocks/filesystem/network/randomness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "command-deadline-cleanup",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs"
          ],
          "gateId": "case-command-deadline-cleanup",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::deadline::actual_epoch_deadline_traps_with_no_identity_and_confirmed_teardown",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "real epoch expiry traps; no fabricated trap identity; Store consumed and deadline worker joined; fresh recovery"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-unwind-no-certificate",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs"
          ],
          "gateId": "case-command-unwind-no-certificate",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::deadline::actual_host_unwind_has_no_run_or_identity_and_never_confirms_teardown",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "host unwind reports unconfirmed teardown; no successful run or controlled identity"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-transfer-live-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-live-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_live_limit_counts_old_and_prospective_resize",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 16 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-live",
            "value": 16,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-live-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-live-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_live_limit_counts_old_and_prospective_resize",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 16 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-live",
            "value": 17,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocations-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocations-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_count_survives_full_drain_and_rejects_first_extra",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocations",
            "value": 4096,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocations-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocations-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_count_survives_full_drain_and_rejects_first_extra",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocations",
            "value": 4097,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocation-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocation-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_byte_limit_and_wrapping_sizes_trap",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocation-bytes",
            "value": 4096,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocation-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocation-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_byte_limit_and_wrapping_sizes_trap",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocation-bytes",
            "value": 4097,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_arena_counts_holes_and_exact_last_byte",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 1048576 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_arena_counts_holes_and_exact_last_byte",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 1048576 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-manifest-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs"
          ],
          "gateId": "case-command-manifest-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::manifest::tests::rejection::strict_utf8_surrogates_trailing_records_and_exact_manifest_byte_bound_reject_and_recover",
            "source": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "strict decoder inclusive whitespace-padded valid record16384 accepted;16385 rejected; decoding grants no execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "manifest-bytes",
            "value": 16384,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-manifest-decoder"
        },
        {
          "id": "command-manifest-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs"
          ],
          "gateId": "case-command-manifest-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::manifest::tests::rejection::strict_utf8_surrogates_trailing_records_and_exact_manifest_byte_bound_reject_and_recover",
            "source": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "strict decoder inclusive whitespace-padded valid record16384 accepted;16385 rejected; decoding grants no execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "manifest-bytes",
            "value": 16385,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-manifest-decoder"
        },
        {
          "id": "command-component-replay",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs"
          ],
          "gateId": "case-command-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::tests::immutable_artifact_binding_is_deterministic_across_fresh_issuers_and_wit_input_order",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "fresh issuer identity and reversed WIT input order preserve exact artifact bytes and bindings"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "installed-pure",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "pure",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-declared-error",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "declared-error",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-found",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "found",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-missing",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "missing",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-empty",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "empty",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-utf8",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "utf8",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-omitted",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "omitted",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-wrong-key",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "wrong-key",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-unsupported",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "unsupported",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-malformed",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "malformed",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-duplicate",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "duplicate",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-conflicting",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "conflicting",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-extra-grant",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "extra-grant",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-relative-grant",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "relative-grant",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-node-override",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "node-override",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-root-override",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "root-override",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-scalar-argument",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "scalar-argument",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-other-export",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "other-export",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-dependency",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "dependency",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-build-only",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "build-only",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-inside-installation",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "inside-installation",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-contains-installation",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "contains-installation",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-default-directory",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "default-directory",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-restored-installation",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "restored-installation",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-existing-output-preserved",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "publication",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "existing-output-preserved",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-provider-tamper",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "provider-tamper",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-runtime-tamper",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "runtime-tamper",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-metadata-tamper",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "metadata-tamper",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-extra-installation-file",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "extra-installation-file",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "h1-private-memory-pages-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-memory-pages-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualrawcore256pages;declaredmax257permitsstructurally; host Memory::grow firstextra page rejected by unchanged16MiBlimiter;freshStore;not guestmemory.grow orbyte+1"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-pages",
            "value": 256,
            "unit": "pages",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-host-Memory-grow"
        },
        {
          "id": "h1-private-memory-pages-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-memory-pages-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualrawcore256pages;declaredmax257permitsstructurally; host Memory::grow firstextra page rejected by unchanged16MiBlimiter;freshStore;not guestmemory.grow orbyte+1"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-pages",
            "value": 257,
            "unit": "pages",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-host-Memory-grow"
        },
        {
          "id": "h1-private-memory-byte-policy-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-h1-private-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::memory_policy::private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualproduction StoreLimits memory_growing directpolicy callback accepts16777216/rejects16777217;freshlimiter; no guestallocation ormemory.grow execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-bytes",
            "value": 16777216,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-production-ResourceLimiter-memory-policy"
        },
        {
          "id": "h1-private-memory-byte-policy-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-h1-private-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::memory_policy::private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualproduction StoreLimits memory_growing directpolicy callback accepts16777216/rejects16777217;freshlimiter; no guestallocation ormemory.grow execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-bytes",
            "value": 16777217,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-production-ResourceLimiter-memory-policy"
        },
        {
          "id": "h1-private-instance-count-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-instance-count-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_instance_limit_accepts_two_and_rejects_third",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two actual emptycoreinstances under unchanged StoreLimits;thirdrejects;freshStore;not publicH1artifact"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "instances",
            "value": 2,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-instances"
        },
        {
          "id": "h1-private-instance-count-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-instance-count-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_instance_limit_accepts_two_and_rejects_third",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two actual emptycoreinstances under unchanged StoreLimits;thirdrejects;freshStore;not publicH1artifact"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "instances",
            "value": 3,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-instances"
        },
        {
          "id": "h1-private-stack-enforcement",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs"
          ],
          "gateId": "case-h1-private-stack-enforcement",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::stack_limits::private_h1_fixed_stack_traps_recursive_core_with_fuel_remaining_and_recovers",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "rawrecursivecore actual StackOverflow with fuelremaining under fixedstack65536; consumedStore/freshsealedH1recovery; NO nativebyteoccupancy65536/65537 oracle; publicrecursionexcluded"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-stack-trap"
        },
        {
          "id": "h1-memory-objects-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-memory-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_one_memory_object_admits_one_and_rejects_second_across_instances",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memories",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-memory-objects-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-memory-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_one_memory_object_admits_one_and_rejects_second_across_instances",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memories",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-table-objects-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-table-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "tables",
            "value": 0,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-table-objects-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-table-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "tables",
            "value": 1,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-backtrace-output-cap",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs"
          ],
          "gateId": "case-h1-backtrace-output-cap",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::backtrace_limits::private_h1_one_frame_backtrace_retains_inner_trap_and_truncates_extra_caller",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inner Unreachable retains one diagnostic frame under productioncap1; extra source caller still traps with one diagnostic frame; not second-call rejection or physical native frame occupancy"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-diagnostic-frame-truncation"
        },
        {
          "id": "h1-fixed-fuel-enforcement",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-h1-fixed-fuel-enforcement",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::fuel_limits::private_h1_fixed_fuel100000_measured_exhaustion_and_next_call_rejection",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged initialFUEL100000; actual get_fuel cost1;99999 completed calls leave1; final charged call OutOfFuel at0; next call OutOfFuel at0; fresh public profile recovery; no100000successfulcalls or caller budget100001 admission claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-measured-fixed-fuel-enforcement"
        },
        {
          "id": "h1-private-component-policy-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs"
          ],
          "gateId": "case-h1-private-component-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::component_audit::tests::byte_policy::private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private actual full validgraph1048576 audits owning byte guard;1048577 rejects; independent language_audit rejects corepadding W4103; not public authenticated H1 canonical accepted1MiB"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-H1-whole-graph-component-byte-policy-language-denies-padding"
        },
        {
          "id": "h1-private-component-policy-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs"
          ],
          "gateId": "case-h1-private-component-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::component_audit::tests::byte_policy::private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private actual full validgraph1048576 audits owning byte guard;1048577 rejects; independent language_audit rejects corepadding W4103; not public authenticated H1 canonical accepted1MiB"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-H1-whole-graph-component-byte-policy-language-denies-padding"
        },
        {
          "id": "h1-public-component-factory-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-component-factory-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::byte_boundary::public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "authenticated actual publicH1 source/verifiedIR/factory emits canonical1048576; issuer/sourceSHA/completegraph/languageaudits and canonicalrevalidation; one sourcebyte signedLEB change emits independentcanonical1048577; ownerguardW4104/factoryW4102 rejects; recovery; shared factorycapacity proof, not installed1MiB runtime claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-H1-source-authenticated-canonical-factory-boundary"
        },
        {
          "id": "h1-public-component-factory-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-component-factory-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::byte_boundary::public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "authenticated actual publicH1 source/verifiedIR/factory emits canonical1048576; issuer/sourceSHA/completegraph/languageaudits and canonicalrevalidation; one sourcebyte signedLEB change emits independentcanonical1048577; ownerguardW4104/factoryW4102 rejects; recovery; shared factorycapacity proof, not installed1MiB runtime claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-H1-source-authenticated-canonical-factory-boundary"
        },
        {
          "id": "installed-component-bytes-exact-limit",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "tests/installed-command-qualification/run.mjs",
            "tests/installed-command-qualification/byte-boundary.mjs",
            "tests/installed-command-qualification/fixtures/byte-boundary/exact.zry"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "component-bytes-exact",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "fresh content-verified relocated installed binary invokes frozen authenticated87161byte source; actual observed source identity bound to retained source bytes and fixed fixture; exact canonicalcomponent1048576 with full WIT34/program/composition/grantless execution and confirmed cleanup; first extra source factory W4102 exit3 has no manifest/results/bundle; final restored installation recovery; requires separate actual installed receipt"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-component-bytes-first-extra",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "tests/installed-command-qualification/run.mjs",
            "tests/installed-command-qualification/byte-boundary.mjs",
            "tests/installed-command-qualification/fixtures/byte-boundary/extra.zry"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "component-bytes-first-extra",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "fresh content-verified relocated installed binary invokes frozen authenticated87161byte source; actual observed source identity bound to retained source bytes and fixed fixture; exact canonicalcomponent1048576 with full WIT34/program/composition/grantless execution and confirmed cleanup; first extra source factory W4102 exit3 has no manifest/results/bundle; final restored installation recovery; requires separate actual installed receipt"
            ],
            "diagnosticCodes": [
              "ZRYNA-W4102"
            ]
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "fresh-content-verified-installed-command"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-command-public-pure",
        "case-command-public-input",
        "case-command-public-denials",
        "case-command-create-only",
        "case-command-request-bytes-exact-limit",
        "case-command-key-bytes-exact-limit",
        "case-command-value-bytes-exact-limit",
        "case-command-value-execution",
        "case-command-first-extra-pre-engine",
        "case-command-denied-callbacks",
        "case-command-grant-does-not-broaden",
        "case-command-deadline-cleanup",
        "case-command-unwind-no-certificate",
        "case-command-transfer-live-exact-limit",
        "case-command-transfer-allocations-exact-limit",
        "case-command-transfer-allocation-bytes-exact-limit",
        "case-command-transfer-bytes-exact-limit",
        "case-command-manifest-exact-limit",
        "case-command-component-replay",
        "case-wit-closure",
        "installed-qualification",
        "case-h1-private-memory-pages-exact-limit",
        "case-h1-private-memory-byte-policy-exact-limit",
        "case-h1-private-instance-count-exact-limit",
        "case-h1-private-stack-enforcement",
        "case-h1-memory-objects-exact-limit",
        "case-h1-table-objects-exact-limit",
        "case-h1-backtrace-output-cap",
        "case-h1-fixed-fuel-enforcement",
        "case-h1-private-component-policy-exact-limit",
        "case-h1-public-component-factory-exact-limit"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces"
      ]
    },
    {
      "id": "installed-command-windows",
      "profile": "command-h1-v1",
      "mode": "fresh-content-verified-installation",
      "host": {
        "os": "win32",
        "architecture": "x64",
        "baseline": "windows-2022",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Wasmtime48.0.1"
      },
      "ownerIssues": [
        400
      ],
      "contract": "docs/WASI_COMMAND_MANIFEST_V1.md",
      "selector": {
        "command": "run",
        "profile": "command-h1-v1",
        "target": "wasi-command",
        "signature": "main():bool -> wasi:cli/run result<(),()>"
      },
      "identities": {
        "world": "zryna:capability-profiles/command@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.command-h1.host.v1",
        "manifest": "zryna.wasi-command-manifest.v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "languageSha256",
          "storageSha256",
          "linear32Sha256",
          "linuxX8664Sha256",
          "programBinding",
          "worldSha256"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [
          "empty-or-environment:get-environment:one-literal-key"
        ],
        "effective": [
          "empty-or-environment:get-environment:one-literal-key"
        ],
        "rootApprovals": [
          "private-literal-key-grant"
        ],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "monotonic-clock"
        ],
        "operationLimits": "one approved literal environment key; repeated reads do not deplete a quota; remaining world imports denied"
      },
      "artifacts": [
        {
          "path": "wasi-command/{stem}.wasm",
          "kind": "wasi-command-component-v1"
        },
        {
          "path": "zryna-wasi-command-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "installed-command-component-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-component-policy-exact-limit",
              "firstExtraCase": "h1-private-component-policy-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "installed-component-bytes-exact-limit",
              "firstExtraCase": "installed-component-bytes-first-extra"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "request-bytes",
          "minimum": 0,
          "maximum": 4096,
          "unit": "bytes",
          "contractId": "installed-command-request-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-request-bytes-exact-limit",
              "firstExtraCase": "command-request-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "key-bytes",
          "minimum": 1,
          "maximum": 64,
          "unit": "bytes",
          "contractId": "installed-command-key-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-key-bytes-exact-limit",
              "firstExtraCase": "command-key-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "value-bytes",
          "minimum": 0,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "installed-command-value-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-value-bytes-exact-limit",
              "firstExtraCase": "command-value-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "manifest-bytes",
          "minimum": 0,
          "maximum": 16384,
          "unit": "bytes",
          "contractId": "installed-command-manifest-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-manifest-exact-limit",
              "firstExtraCase": "command-manifest-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "transfer-live",
          "minimum": 0,
          "maximum": 16,
          "unit": "count",
          "contractId": "installed-command-transfer-live",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-live-exact-limit",
              "firstExtraCase": "command-transfer-live-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "transfer-allocations",
          "minimum": 0,
          "maximum": 4096,
          "unit": "count",
          "contractId": "installed-command-transfer-allocations",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-allocations-exact-limit",
              "firstExtraCase": "command-transfer-allocations-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "transfer-allocation-bytes",
          "minimum": 0,
          "maximum": 4096,
          "unit": "bytes",
          "contractId": "installed-command-transfer-allocation-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-allocation-bytes-exact-limit",
              "firstExtraCase": "command-transfer-allocation-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "transfer-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "installed-command-transfer-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "command-transfer-bytes-exact-limit",
              "firstExtraCase": "command-transfer-bytes-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "fuel",
          "minimum": 0,
          "maximum": 100000,
          "unit": "fuel-units",
          "contractId": "installed-command-fuel",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-fixed-fuel-enforcement"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "installed-command-fuel-public-input-exclusion"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "deadline",
          "minimum": 1,
          "maximum": 5000,
          "unit": "milliseconds",
          "contractId": "installed-command-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "command-deadline-cleanup"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "installed-command-deadline-public-input-exclusion"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "wasm-stack",
          "minimum": 0,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "installed-command-wasm-stack",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-private-stack-enforcement"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "installed-command-wasm-stack-public-input-exclusion"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "backtrace-frames",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "installed-command-backtrace-frames",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "enforcement",
              "caseIds": [
                "h1-backtrace-output-cap"
              ]
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "installed-command-backtrace-frames-public-input-exclusion"
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "instances",
          "minimum": 0,
          "maximum": 2,
          "unit": "count",
          "contractId": "installed-command-instances",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-instance-count-exact-limit",
              "firstExtraCase": "h1-private-instance-count-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "memories",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "installed-command-memories",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-memory-objects-exact-limit",
              "firstExtraCase": "h1-memory-objects-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "tables",
          "minimum": 0,
          "maximum": 0,
          "unit": "count",
          "contractId": "installed-command-tables",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-table-objects-exact-limit",
              "firstExtraCase": "h1-table-objects-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "memory-bytes",
          "minimum": 0,
          "maximum": 16777216,
          "unit": "bytes",
          "contractId": "installed-command-memory-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-memory-byte-policy-exact-limit",
              "firstExtraCase": "h1-private-memory-byte-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        },
        {
          "id": "memory-pages",
          "minimum": 0,
          "maximum": 256,
          "unit": "pages",
          "contractId": "installed-command-memory-pages",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "h1-private-memory-pages-exact-limit",
              "firstExtraCase": "h1-private-memory-pages-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "installed-pure",
                "installed-found"
              ]
            },
            "bindingCaseIds": [
              "installed-pure",
              "installed-found"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "command-public-pure",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-pure",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_pure_and_declared_error_commit_distinct_complete_command_bundles",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "pure ok and declared err complete bundles"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-public-input",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-input",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_found_missing_empty_and_utf8_values_are_private_and_released",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "found/missing/present-empty and UTF8; private input absent in published records; handles released"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-public-denials",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-public-denials",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_wrong_omitted_extra_grants_and_wrong_export_reject_without_bundle",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "wrong/omitted/extra grant and wrong export reject with no bundle"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-create-only",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "publication",
          "fixture": [
            "apps/zryna/tests/wasi_command.rs"
          ],
          "gateId": "case-command-create-only",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_command",
            "selector": "actual_cli_create_only_retry_preserves_component_and_manifest_inventory",
            "source": "apps/zryna/tests/wasi_command.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "collision preserves complete prior component/manifest inventory"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-command"
        },
        {
          "id": "command-request-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::exact_file_limit_accepts_and_first_extra_byte_rejects",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 4096 UTF8 bytes accepted and 4097 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-bytes",
            "value": 4096,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-request-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::exact_file_limit_accepts_and_first_extra_byte_rejects",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 4096 UTF8 bytes accepted and 4097 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-bytes",
            "value": 4097,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-key-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-key-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::key_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 64 UTF8 bytes accepted and 65 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "key-bytes",
            "value": 64,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-key-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-key-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::key_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 64 UTF8 bytes accepted and 65 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "key-bytes",
            "value": 65,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-value-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::present_value_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 1024 UTF8 bytes accepted and 1025 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "value-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_request/tests/bounds.rs"
          ],
          "gateId": "case-command-value-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_request::tests::bounds::present_value_limit_counts_utf8_bytes_and_rejects_first_extra",
            "source": "crates/zryna-driver/src/command_request/tests/bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inclusive 1024 UTF8 bytes accepted and 1025 rejected D4101"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "value-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-request-parser"
        },
        {
          "id": "command-value-execution",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs"
          ],
          "gateId": "case-command-value-execution",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::execution::actual_found_empty_ascii_multibyte_and_exact_1024_bytes_run_ok",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "present-empty/ASCII/multibyte/exact1024 runs WIT ok; teardown confirmed; private handles released"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-first-extra-pre-engine",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs"
          ],
          "gateId": "case-command-first-extra-pre-engine",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::preparation::first_extra_value_byte_and_nonprivate_input_reject_before_engine",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/preparation.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "1025-byte value rejected D4101 before consuming run; fresh valid recovery"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-admission"
        },
        {
          "id": "command-denied-callbacks",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs"
          ],
          "gateId": "case-command-denied-callbacks",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::denied_probes::actual_denied_capability_matrix_seals_first_callback_and_consumes_each_store",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "each admitted-world denied callback seals first denial and consumes actual Store"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "command-grant-does-not-broaden",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs"
          ],
          "gateId": "case-command-grant-does-not-broaden",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::denied_probes::actual_environment_grant_does_not_approve_other_capability_callbacks",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/denied_probes.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "environment approval does not admit clocks/filesystem/network/randomness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "command-deadline-cleanup",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs"
          ],
          "gateId": "case-command-deadline-cleanup",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::deadline::actual_epoch_deadline_traps_with_no_identity_and_confirmed_teardown",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "real epoch expiry traps; no fabricated trap identity; Store consumed and deadline worker joined; fresh recovery"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-unwind-no-certificate",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs"
          ],
          "gateId": "case-command-unwind-no-certificate",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::deadline::actual_host_unwind_has_no_run_or_identity_and_never_confirms_teardown",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/deadline.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "host unwind reports unconfirmed teardown; no successful run or controlled identity"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "sealed-command-execution"
        },
        {
          "id": "command-transfer-live-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-live-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_live_limit_counts_old_and_prospective_resize",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 16 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-live",
            "value": 16,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-live-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-live-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_live_limit_counts_old_and_prospective_resize",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 16 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-live",
            "value": 17,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocations-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocations-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_count_survives_full_drain_and_rejects_first_extra",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocations",
            "value": 4096,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocations-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocations-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_count_survives_full_drain_and_rejects_first_extra",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocations",
            "value": 4097,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocation-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocation-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_byte_limit_and_wrapping_sizes_trap",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocation-bytes",
            "value": 4096,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-allocation-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-allocation-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_allocation_byte_limit_and_wrapping_sizes_trap",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 4096 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-allocation-bytes",
            "value": 4097,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-bytes-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_arena_counts_holes_and_exact_last_byte",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 1048576 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-transfer-bytes-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs"
          ],
          "gateId": "case-command-transfer-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::storage::tests::resources::command_storage_arena_counts_holes_and_exact_last_byte",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/storage/tests/resources.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "production storage core inclusive 1048576 accepted; first extra fatal; no public source invocation claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "transfer-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "sealed-storage-core-execution"
        },
        {
          "id": "command-manifest-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs"
          ],
          "gateId": "case-command-manifest-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::manifest::tests::rejection::strict_utf8_surrogates_trailing_records_and_exact_manifest_byte_bound_reject_and_recover",
            "source": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "strict decoder inclusive whitespace-padded valid record16384 accepted;16385 rejected; decoding grants no execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "manifest-bytes",
            "value": 16384,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-manifest-decoder"
        },
        {
          "id": "command-manifest-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs"
          ],
          "gateId": "case-command-manifest-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::manifest::tests::rejection::strict_utf8_surrogates_trailing_records_and_exact_manifest_byte_bound_reject_and_recover",
            "source": "crates/zryna-driver/src/command_h1_runtime/manifest/tests/rejection.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "strict decoder inclusive whitespace-padded valid record16384 accepted;16385 rejected; decoding grants no execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "manifest-bytes",
            "value": 16385,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-manifest-decoder"
        },
        {
          "id": "command-component-replay",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs"
          ],
          "gateId": "case-command-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::tests::immutable_artifact_binding_is_deterministic_across_fresh_issuers_and_wit_input_order",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "fresh issuer identity and reversed WIT input order preserve exact artifact bytes and bindings"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "installed-pure",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "pure",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-declared-error",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "declared-error",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-found",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "found",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-missing",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "missing",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-empty",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "empty",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-utf8",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "utf8",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-omitted",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "omitted",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-wrong-key",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "wrong-key",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-unsupported",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "unsupported",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-malformed",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "malformed",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-duplicate",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "duplicate",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-conflicting",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "conflicting",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-extra-grant",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "extra-grant",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-relative-grant",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "relative-grant",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-node-override",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "node-override",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-root-override",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "root-override",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-scalar-argument",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "scalar-argument",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-other-export",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "other-export",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-dependency",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "dependency",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-build-only",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "build-only",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-inside-installation",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "inside-installation",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-contains-installation",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "contains-installation",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-default-directory",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "default-directory",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-restored-installation",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "positive",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "restored-installation",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-existing-output-preserved",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "publication",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "existing-output-preserved",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-provider-tamper",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "provider-tamper",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-runtime-tamper",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "runtime-tamper",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-metadata-tamper",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "metadata-tamper",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-extra-installation-file",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "negative",
          "fixture": [
            "tests/installed-command-qualification/run.mjs"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "extra-installation-file",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "exact named invocation arguments, exit and no-extra bundle checked in invocations.json; complete content-verified fresh candidate; relocation/outside-checkout/emptyPATH; no signature or beta-archive claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "h1-private-memory-pages-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-memory-pages-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualrawcore256pages;declaredmax257permitsstructurally; host Memory::grow firstextra page rejected by unchanged16MiBlimiter;freshStore;not guestmemory.grow orbyte+1"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-pages",
            "value": 256,
            "unit": "pages",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-host-Memory-grow"
        },
        {
          "id": "h1-private-memory-pages-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-memory-pages-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_memory_accepts256_pages_and_rejects_first_extra_page",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualrawcore256pages;declaredmax257permitsstructurally; host Memory::grow firstextra page rejected by unchanged16MiBlimiter;freshStore;not guestmemory.grow orbyte+1"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-pages",
            "value": 257,
            "unit": "pages",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-host-Memory-grow"
        },
        {
          "id": "h1-private-memory-byte-policy-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-h1-private-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::memory_policy::private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualproduction StoreLimits memory_growing directpolicy callback accepts16777216/rejects16777217;freshlimiter; no guestallocation ormemory.grow execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-bytes",
            "value": 16777216,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-production-ResourceLimiter-memory-policy"
        },
        {
          "id": "h1-private-memory-byte-policy-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-h1-private-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::memory_policy::private_h1_memory_policy_accepts16777216_bytes_and_rejects16777217",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actualproduction StoreLimits memory_growing directpolicy callback accepts16777216/rejects16777217;freshlimiter; no guestallocation ormemory.grow execution"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memory-bytes",
            "value": 16777217,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-production-ResourceLimiter-memory-policy"
        },
        {
          "id": "h1-private-instance-count-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-instance-count-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_instance_limit_accepts_two_and_rejects_third",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two actual emptycoreinstances under unchanged StoreLimits;thirdrejects;freshStore;not publicH1artifact"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "instances",
            "value": 2,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-instances"
        },
        {
          "id": "h1-private-instance-count-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs"
          ],
          "gateId": "case-h1-private-instance-count-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::envelope_limits::private_h1_fixed_instance_limit_accepts_two_and_rejects_third",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/envelope_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "two actual emptycoreinstances under unchanged StoreLimits;thirdrejects;freshStore;not publicH1artifact"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "instances",
            "value": 3,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-instances"
        },
        {
          "id": "h1-private-stack-enforcement",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs"
          ],
          "gateId": "case-h1-private-stack-enforcement",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::stack_limits::private_h1_fixed_stack_traps_recursive_core_with_fuel_remaining_and_recovers",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/stack_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "rawrecursivecore actual StackOverflow with fuelremaining under fixedstack65536; consumedStore/freshsealedH1recovery; NO nativebyteoccupancy65536/65537 oracle; publicrecursionexcluded"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-stack-trap"
        },
        {
          "id": "h1-memory-objects-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-memory-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_one_memory_object_admits_one_and_rejects_second_across_instances",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memories",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-memory-objects-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-memory-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_one_memory_object_admits_one_and_rejects_second_across_instances",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "memories",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-table-objects-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-table-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "tables",
            "value": 0,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-table-objects-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs"
          ],
          "gateId": "case-h1-table-objects-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::object_limits::private_h1_zero_table_objects_admits_empty_instance_and_rejects_first_table",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/object_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-production-StoreLimits-raw-core-object-count; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "tables",
            "value": 1,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-StoreLimits-raw-core-object-count"
        },
        {
          "id": "h1-backtrace-output-cap",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs"
          ],
          "gateId": "case-h1-backtrace-output-cap",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::backtrace_limits::private_h1_one_frame_backtrace_retains_inner_trap_and_truncates_extra_caller",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/backtrace_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual inner Unreachable retains one diagnostic frame under productioncap1; extra source caller still traps with one diagnostic frame; not second-call rejection or physical native frame occupancy"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-diagnostic-frame-truncation"
        },
        {
          "id": "h1-fixed-fuel-enforcement",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-h1-fixed-fuel-enforcement",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "command_h1_runtime::tests::fuel_limits::private_h1_fixed_fuel100000_measured_exhaustion_and_next_call_rejection",
            "source": "crates/zryna-driver/src/command_h1_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged initialFUEL100000; actual get_fuel cost1;99999 completed calls leave1; final charged call OutOfFuel at0; next call OutOfFuel at0; fresh public profile recovery; no100000successfulcalls or caller budget100001 admission claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "private-production-engine-measured-fixed-fuel-enforcement"
        },
        {
          "id": "h1-private-component-policy-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs"
          ],
          "gateId": "case-h1-private-component-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::component_audit::tests::byte_policy::private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private actual full validgraph1048576 audits owning byte guard;1048577 rejects; independent language_audit rejects corepadding W4103; not public authenticated H1 canonical accepted1MiB"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-H1-whole-graph-component-byte-policy-language-denies-padding"
        },
        {
          "id": "h1-private-component-policy-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs"
          ],
          "gateId": "case-h1-private-component-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::component_audit::tests::byte_policy::private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/component_audit/tests/byte_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private actual full validgraph1048576 audits owning byte guard;1048577 rejects; independent language_audit rejects corepadding W4103; not public authenticated H1 canonical accepted1MiB"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-H1-whole-graph-component-byte-policy-language-denies-padding"
        },
        {
          "id": "h1-public-component-factory-exact-limit",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-component-factory-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::byte_boundary::public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "authenticated actual publicH1 source/verifiedIR/factory emits canonical1048576; issuer/sourceSHA/completegraph/languageaudits and canonicalrevalidation; one sourcebyte signedLEB change emits independentcanonical1048577; ownerguardW4104/factoryW4102 rejects; recovery; shared factorycapacity proof, not installed1MiB runtime claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-H1-source-authenticated-canonical-factory-boundary"
        },
        {
          "id": "h1-public-component-factory-first-extra",
          "rowIds": [
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs"
          ],
          "gateId": "case-h1-public-component-factory-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "command_h1_v1::artifact::byte_boundary::public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers",
            "source": "crates/zryna-backend-webassembly/src/command_h1_v1/artifact/tests/byte_boundary.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "authenticated actual publicH1 source/verifiedIR/factory emits canonical1048576; issuer/sourceSHA/completegraph/languageaudits and canonicalrevalidation; one sourcebyte signedLEB change emits independentcanonical1048577; ownerguardW4104/factoryW4102 rejects; recovery; shared factorycapacity proof, not installed1MiB runtime claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-H1-source-authenticated-canonical-factory-boundary"
        },
        {
          "id": "installed-component-bytes-exact-limit",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "tests/installed-command-qualification/run.mjs",
            "tests/installed-command-qualification/byte-boundary.mjs",
            "tests/installed-command-qualification/fixtures/byte-boundary/exact.zry"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "component-bytes-exact",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "fresh content-verified relocated installed binary invokes frozen authenticated87161byte source; actual observed source identity bound to retained source bytes and fixed fixture; exact canonicalcomponent1048576 with full WIT34/program/composition/grantless execution and confirmed cleanup; first extra source factory W4102 exit3 has no manifest/results/bundle; final restored installation recovery; requires separate actual installed receipt"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048576,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "fresh-content-verified-installed-command"
        },
        {
          "id": "installed-component-bytes-first-extra",
          "rowIds": [
            "installed-command-linux",
            "installed-command-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "tests/installed-command-qualification/run.mjs",
            "tests/installed-command-qualification/byte-boundary.mjs",
            "tests/installed-command-qualification/fixtures/byte-boundary/extra.zry"
          ],
          "gateId": "installed-qualification",
          "test": {
            "runner": "installed-invocation",
            "package": "zryna",
            "target": "fresh-relocated-installation",
            "selector": "component-bytes-first-extra",
            "source": "tests/installed-command-qualification/run.mjs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "fresh content-verified relocated installed binary invokes frozen authenticated87161byte source; actual observed source identity bound to retained source bytes and fixed fixture; exact canonicalcomponent1048576 with full WIT34/program/composition/grantless execution and confirmed cleanup; first extra source factory W4102 exit3 has no manifest/results/bundle; final restored installation recovery; requires separate actual installed receipt"
            ],
            "diagnosticCodes": [
              "ZRYNA-W4102"
            ]
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "fresh-content-verified-installed-command"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-command-public-pure",
        "case-command-public-input",
        "case-command-public-denials",
        "case-command-create-only",
        "case-command-request-bytes-exact-limit",
        "case-command-key-bytes-exact-limit",
        "case-command-value-bytes-exact-limit",
        "case-command-value-execution",
        "case-command-first-extra-pre-engine",
        "case-command-denied-callbacks",
        "case-command-grant-does-not-broaden",
        "case-command-deadline-cleanup",
        "case-command-unwind-no-certificate",
        "case-command-transfer-live-exact-limit",
        "case-command-transfer-allocations-exact-limit",
        "case-command-transfer-allocation-bytes-exact-limit",
        "case-command-transfer-bytes-exact-limit",
        "case-command-manifest-exact-limit",
        "case-command-component-replay",
        "case-wit-closure",
        "installed-qualification",
        "case-h1-private-memory-pages-exact-limit",
        "case-h1-private-memory-byte-policy-exact-limit",
        "case-h1-private-instance-count-exact-limit",
        "case-h1-private-stack-enforcement",
        "case-h1-memory-objects-exact-limit",
        "case-h1-table-objects-exact-limit",
        "case-h1-backtrace-output-cap",
        "case-h1-fixed-fuel-enforcement",
        "case-h1-private-component-policy-exact-limit",
        "case-h1-public-component-factory-exact-limit"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces"
      ]
    },
    {
      "id": "server-status-linux",
      "profile": "server-status-v1",
      "mode": "checkout-serve",
      "host": {
        "os": "linux",
        "architecture": "x64",
        "baseline": "ubuntu-latest:observed-baseline-required",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Wasmtime48.0.1"
      },
      "ownerIssues": [
        401
      ],
      "contract": "docs/WASI_SERVER_STATUS_V1.md",
      "selector": {
        "command": "serve",
        "profile": "server-status-v1",
        "target": null,
        "signature": "sole no-argument i32 export; status200..599; empty response"
      },
      "identities": {
        "world": "zryna:capability-profiles/server@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.server-empty-response.v1",
        "manifest": "zryna.wasi-server-manifest.v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "coreSha256",
          "bridgeRevision",
          "preparationIdentity",
          "configurationSha256",
          "listenerApprovalIdentity"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [],
        "effective": [],
        "rootApprovals": [
          "listener-configuration-digest"
        ],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "environment",
          "monotonic-clock"
        ],
        "operationLimits": "no guest capability operations; listener approval is root host authority only"
      },
      "artifacts": [
        {
          "path": "component/{stem}.wasm",
          "kind": "webassembly-component"
        },
        {
          "path": "zryna-wasi-server-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "body-bytes",
          "minimum": 1,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "server-status-body-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-body-bytes-exact-limit",
              "firstExtraCase": "server-body-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-body-bytes-exact-limit",
              "firstExtraCase": "server-body-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "header-bytes",
          "minimum": 64,
          "maximum": 8192,
          "unit": "bytes",
          "contractId": "server-status-header-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-header-bytes-exact-limit",
              "firstExtraCase": "server-header-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-header-bytes-exact-limit",
              "firstExtraCase": "server-header-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "header-fields",
          "minimum": 0,
          "maximum": 32,
          "unit": "count",
          "contractId": "server-status-header-fields",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-header-fields-exact-limit",
              "firstExtraCase": "server-header-fields-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-header-fields-exact-limit",
              "firstExtraCase": "server-header-fields-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "path-bytes",
          "minimum": 0,
          "maximum": 256,
          "unit": "bytes",
          "contractId": "server-status-path-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-path-bytes-exact-limit",
              "firstExtraCase": "server-path-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-path-bytes-exact-limit",
              "firstExtraCase": "server-path-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "attempts",
          "minimum": 1,
          "maximum": 64,
          "unit": "count",
          "contractId": "server-status-attempts",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-attempts-exact",
              "firstExtraCase": "server-attempts-first-extra-config"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-attempts-exact",
              "firstExtraCase": "server-attempts-first-extra-config"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "configuration-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-status-configuration-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-config-capture-exact-limit",
              "firstExtraCase": "server-config-capture-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-config-capture-exact-limit",
              "firstExtraCase": "server-config-capture-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "listener-approval-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-status-listener-approval-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-listener-capture-exact-limit",
              "firstExtraCase": "server-listener-capture-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-listener-capture-exact-limit",
              "firstExtraCase": "server-listener-capture-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "request-deadline",
          "minimum": 1,
          "maximum": 5000,
          "unit": "milliseconds",
          "contractId": "server-status-request-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-request-deadline-exact-limit",
              "firstExtraCase": "server-request-deadline-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-request-deadline-exact-limit",
              "firstExtraCase": "server-request-deadline-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "service-lifetime",
          "minimum": 1,
          "maximum": 30000,
          "unit": "milliseconds",
          "contractId": "server-status-service-lifetime",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-service-lifetime-exact-limit",
              "firstExtraCase": "server-service-lifetime-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-service-lifetime-exact-limit",
              "firstExtraCase": "server-service-lifetime-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "concurrent-requests",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "server-status-concurrent-requests",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-serial-request-policy-exact-limit",
              "firstExtraCase": "server-serial-request-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-status-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "guest-memory",
          "minimum": 0,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "server-status-guest-memory",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-memory-byte-policy-exact-limit",
              "firstExtraCase": "server-memory-byte-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-status-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "guest-fuel",
          "minimum": 0,
          "maximum": 100000,
          "unit": "fuel-units",
          "contractId": "server-status-guest-fuel",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-fuel-policy-exact-limit",
              "firstExtraCase": "server-fuel-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-status-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "guest-resources",
          "minimum": 0,
          "maximum": 5,
          "unit": "count",
          "contractId": "server-status-guest-resources",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-private-resource-quota-exact-limit",
              "firstExtraCase": "server-private-resource-quota-first-extra"
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "server-status-guest-resources-public-input-exclusion"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "guest-callbacks",
          "minimum": 0,
          "maximum": 8,
          "unit": "count",
          "contractId": "server-status-guest-callbacks",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-private-callback-quota-exact-limit",
              "firstExtraCase": "server-private-callback-quota-first-extra"
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "server-status-guest-callbacks-public-input-exclusion"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "server-status-component-bytes",
          "mapping": {
            "kind": "excluded",
            "exclusionId": "server-status-component-bytes-public-input-exclusion",
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "server-status-public",
          "rowIds": [
            "server-status-linux",
            "server-status-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-status-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "actual_server_status_boundaries_malformed_attempts_and_repeated_start",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "status200/599 empty bodies; malformed request rejected; served2/rejected1; empty grants; actual teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-status-denial",
          "rowIds": [
            "server-status-linux",
            "server-status-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-status-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "rejected_configuration_and_unapproved_listener_never_start_or_publish",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unapproved listener/input denied before readiness and publication"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-source-denial",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_server/source_tests.rs"
          ],
          "gateId": "case-server-source-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "source_tests::actual_source_effects_wrong_signatures_extra_exports_and_dependencies_reject_before_ready",
            "source": "apps/zryna/tests/wasi_server/source_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "source effects/signatures/extra exports/dependencies deny before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-cancel",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-cancel",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "readiness_cancellation_closes_actual_listener_and_commits_typed_teardown",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "cancel readiness; accepted0; actual listener closes; confirmed typed teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-forced-stop",
          "rowIds": [
            "server-status-linux",
            "server-status-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-forced-stop",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "forced_cli_termination_closes_listener_without_graceful_record",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "OS listener closes; no graceful record or cleanup certificate"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-body-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-body-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_body_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 65536 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "body-bytes",
            "value": 65536,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-body-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-body-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_body_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 65536 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "body-bytes",
            "value": 65537,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_complete_header_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 8192 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-bytes",
            "value": 8192,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_complete_header_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 8192 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-bytes",
            "value": 8193,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-fields-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-fields-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_header_fields_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 32 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-fields",
            "value": 32,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-fields-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-fields-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_header_fields_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 32 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-fields",
            "value": 33,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-path-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-path-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_path_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 256 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "path-bytes",
            "value": 256,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-path-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-path-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_path_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 256 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "path-bytes",
            "value": 257,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-attempts-exact",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-attempts-exact",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_attempt_budget_serves64_limit_intersections_and_closes_before_service_expiry",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "all64 intersecting maxima served;64 Stores and320 resources destroyed; actual joined listener closes before service expiry"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "attempts",
            "value": 64,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-lowered-envelopes",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/execution.rs"
          ],
          "gateId": "case-server-lowered-envelopes",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_runtime::tests::execution::actual_guest_resource_fuel_memory_and_callback_limits_trap_and_recover",
            "source": "crates/zryna-driver/src/server_runtime/tests/execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "resources3/fuel1/memory65535/callbacks5 lower fixture traps; no production-ceiling certificate"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "lowered-runtime-envelope"
        },
        {
          "id": "server-memory-growth",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs"
          ],
          "gateId": "case-server-memory-growth",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_runtime::tests::probe_execution::actual_guest_memory_growth_traps_at_one_page_and_destroys_store",
            "source": "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "real memory growth beyond one page traps; Store destroyed"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "server-configuration-negative",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-configuration-negative",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::negative::strict_configuration_rejects_unknown_duplicate_unbounded_and_nonloopback_values",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "duplicate/unknown/nonloopback and extra attempt/header/body/service limits rejected; no implicit exact request5000/service30000 acceptance"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "production-configuration-admission"
        },
        {
          "id": "server-component-replay",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests.rs"
          ],
          "gateId": "case-server-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::exact_world_empty_response_and_clock_arrangements_validate_and_replay",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "both exact arrangements replay byte-identically; original scalar core; distinct status/clock identities"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "server-request-deadline-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-request-deadline-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_request_deadline5000_and_first_extra5001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-deadline",
            "value": 5000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-request-deadline-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-request-deadline-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_request_deadline5000_and_first_extra5001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-deadline",
            "value": 5001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-service-lifetime-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-service-lifetime-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_service_deadline30000_and_first_extra30001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "service-lifetime",
            "value": 30000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-service-lifetime-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-service-lifetime-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_service_deadline30000_and_first_extra30001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "service-lifetime",
            "value": 30001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-public-envelope-binding",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-public-envelope-binding",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::public_production_resource_callback_and_fuel_envelope_executes_both_arrangements",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::envelope resources5/callbacks8/fuel100000; actual Reply/ClockRead202 emptyresponse and expected0/1reads; five resourcescreated/destroyed; reservationszero;joined shutdown; this proves using settings, not exhausting quotas"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "production-server-arrangement-under-public-envelope"
        },
        {
          "id": "server-private-resource-quota-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-private-resource-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "five actual registry owners; sixth rejected before allocation; releaseone/recover;actualdestruction; public fixed handler peaks4 so no publicguestexhaustion claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-resources",
            "value": 5,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-resource-registry"
        },
        {
          "id": "server-private-resource-quota-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-private-resource-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "five actual registry owners; sixth rejected before allocation; releaseone/recover;actualdestruction; public fixed handler peaks4 so no publicguestexhaustion claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-resources",
            "value": 6,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-resource-registry"
        },
        {
          "id": "server-private-callback-quota-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs"
          ],
          "gateId": "case-server-private-callback-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::callback_limits::private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth",
            "source": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "eight actual Host::enter policy invocations accept; ninth/further reject; unchanged Config::envelopecallbacks8; freshHostrecovery; zeroStore/worker/provider/response; no actualguestcallbacks"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-callbacks",
            "value": 8,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-Host-enter-policy"
        },
        {
          "id": "server-private-callback-quota-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs"
          ],
          "gateId": "case-server-private-callback-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::callback_limits::private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth",
            "source": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "eight actual Host::enter policy invocations accept; ninth/further reject; unchanged Config::envelopecallbacks8; freshHostrecovery; zeroStore/worker/provider/response; no actualguestcallbacks"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-callbacks",
            "value": 9,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-Host-enter-policy"
        },
        {
          "id": "server-public-shape-size-exclusion",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs"
          ],
          "gateId": "case-server-public-shape-size-exclusion",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::public_size_bound::public_single_scalar_size_upper_bound_is_strictly_below_component_ceiling",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "single no-arg i32 solefunction sourceboundshape;actual16383node/128bytename canonical artifact emitted/audited; bothReply/ClockRead;symbolic bound baseline+57343+256+256 strictlybelow1048576; source/IR/ABI/encoder/topology proofneeds independentreview+execution; NO exact1048576acceptance or authenticatedworkerpublicsource claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "conditional-public-shape-reachability-exclusion"
        },
        {
          "id": "server-attempts-first-extra-config",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-status-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "rejected_configuration_and_unapproved_listener_never_start_or_publish",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "attempts65 selection rejects before readiness/bundle; joins actual64 sharedproductiontransport only through an explicit mixedscope pair"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "attempts",
            "value": 65,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-component-first-extra-length-guard",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests.rs"
          ],
          "gateId": "case-server-component-first-extra-length-guard",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::malformed_oversized_and_extra_sections_do_not_reach_world_decoder",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual audit receives1048577zero-filled bytes and rejects; unchanged audit length guard executes before worlddecoder; vector is not structurally valid or canonical component; assertion is rejection only, no exact1048576acceptance"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-server-component-audit-length-guard"
        },
        {
          "id": "server-fuel-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-server-fuel-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::fuel_limits::private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion",
            "source": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged Config::envelope fuel100000; Envelope.validate100000 accepts/100001 ErrorLimit; actual measured unchanged productionmeter99999completedcalls then lastchargedcall traps at0 and nextOutOfFuel; fresh profile recovery; no publicguest100000successfulcalls"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-fuel",
            "value": 100000,
            "unit": "fuel-units",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-Envelope-fuel-admission"
        },
        {
          "id": "server-fuel-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-server-fuel-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::fuel_limits::private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion",
            "source": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged Config::envelope fuel100000; Envelope.validate100000 accepts/100001 ErrorLimit; actual measured unchanged productionmeter99999completedcalls then lastchargedcall traps at0 and nextOutOfFuel; fresh profile recovery; no publicguest100000successfulcalls"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-fuel",
            "value": 100001,
            "unit": "fuel-units",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-Envelope-fuel-admission"
        },
        {
          "id": "server-memory-byte-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-server-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::memory_policy::public_server_memory_envelope_accepts65536_and_rejects65537_before_host",
            "source": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-memory",
            "value": 65536,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
        },
        {
          "id": "server-memory-byte-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-server-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::memory_policy::public_server_memory_envelope_accepts65536_and_rejects65537_before_host",
            "source": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-memory",
            "value": 65537,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
        },
        {
          "id": "server-serial-request-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs"
          ],
          "gateId": "case-server-serial-request-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::retirement::public_serial::production_serial_one_request_accepts_first_rejects_second_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config.requests1 unchanged; private Prepared first actual paused Store retained; second admission rejected before anotherStore; release204 teardown/recovery; not TCPbacklog or public concurrent socket rejection"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "concurrent-requests",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-production-lifecycle-real-Store-admission"
        },
        {
          "id": "server-serial-request-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs"
          ],
          "gateId": "case-server-serial-request-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::retirement::public_serial::production_serial_one_request_accepts_first_rejects_second_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config.requests1 unchanged; private Prepared first actual paused Store retained; second admission rejected before anotherStore; release204 teardown/recovery; not TCPbacklog or public concurrent socket rejection"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "concurrent-requests",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-production-lifecycle-real-Store-admission"
        },
        {
          "id": "server-config-capture-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-config-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "configuration-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-config-capture-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-config-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "configuration-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-listener-capture-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-listener-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "listener-approval-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-listener-capture-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-listener-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "listener-approval-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-server-status-public",
        "case-server-status-denial",
        "case-server-source-denial",
        "case-server-cancel",
        "case-server-forced-stop",
        "case-server-body-bytes-exact-limit",
        "case-server-header-bytes-exact-limit",
        "case-server-header-fields-exact-limit",
        "case-server-path-bytes-exact-limit",
        "case-server-attempts-exact",
        "case-server-lowered-envelopes",
        "case-server-memory-growth",
        "case-server-configuration-negative",
        "case-server-component-replay",
        "case-wit-closure",
        "case-server-request-deadline-exact-limit",
        "case-server-service-lifetime-exact-limit",
        "case-server-public-envelope-binding",
        "case-server-private-resource-quota-exact-limit",
        "case-server-private-callback-quota-exact-limit",
        "case-server-public-shape-size-exclusion",
        "case-server-component-first-extra-length-guard",
        "case-server-fuel-policy-exact-limit",
        "case-server-memory-byte-policy-exact-limit",
        "case-server-serial-request-policy-exact-limit",
        "case-server-config-capture-exact-limit",
        "case-server-listener-capture-exact-limit"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces"
      ]
    },
    {
      "id": "server-status-windows",
      "profile": "server-status-v1",
      "mode": "checkout-serve",
      "host": {
        "os": "win32",
        "architecture": "x64",
        "baseline": "windows-latest:observed-baseline-required",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Wasmtime48.0.1"
      },
      "ownerIssues": [
        401
      ],
      "contract": "docs/WASI_SERVER_STATUS_V1.md",
      "selector": {
        "command": "serve",
        "profile": "server-status-v1",
        "target": null,
        "signature": "sole no-argument i32 export; status200..599; empty response"
      },
      "identities": {
        "world": "zryna:capability-profiles/server@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.server-empty-response.v1",
        "manifest": "zryna.wasi-server-manifest.v1",
        "manifestVersion": 1,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "coreSha256",
          "bridgeRevision",
          "preparationIdentity",
          "configurationSha256",
          "listenerApprovalIdentity"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [],
        "effective": [],
        "rootApprovals": [
          "listener-configuration-digest"
        ],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "environment",
          "monotonic-clock"
        ],
        "operationLimits": "no guest capability operations; listener approval is root host authority only"
      },
      "artifacts": [
        {
          "path": "component/{stem}.wasm",
          "kind": "webassembly-component"
        },
        {
          "path": "zryna-wasi-server-manifest-v1.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "body-bytes",
          "minimum": 1,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "server-status-body-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-body-bytes-exact-limit",
              "firstExtraCase": "server-body-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-body-bytes-exact-limit",
              "firstExtraCase": "server-body-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "header-bytes",
          "minimum": 64,
          "maximum": 8192,
          "unit": "bytes",
          "contractId": "server-status-header-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-header-bytes-exact-limit",
              "firstExtraCase": "server-header-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-header-bytes-exact-limit",
              "firstExtraCase": "server-header-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "header-fields",
          "minimum": 0,
          "maximum": 32,
          "unit": "count",
          "contractId": "server-status-header-fields",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-header-fields-exact-limit",
              "firstExtraCase": "server-header-fields-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-header-fields-exact-limit",
              "firstExtraCase": "server-header-fields-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "path-bytes",
          "minimum": 0,
          "maximum": 256,
          "unit": "bytes",
          "contractId": "server-status-path-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-path-bytes-exact-limit",
              "firstExtraCase": "server-path-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-path-bytes-exact-limit",
              "firstExtraCase": "server-path-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "attempts",
          "minimum": 1,
          "maximum": 64,
          "unit": "count",
          "contractId": "server-status-attempts",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-attempts-exact",
              "firstExtraCase": "server-attempts-first-extra-config"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-attempts-exact",
              "firstExtraCase": "server-attempts-first-extra-config"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "configuration-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-status-configuration-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-config-capture-exact-limit",
              "firstExtraCase": "server-config-capture-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-config-capture-exact-limit",
              "firstExtraCase": "server-config-capture-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "listener-approval-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-status-listener-approval-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-listener-capture-exact-limit",
              "firstExtraCase": "server-listener-capture-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-listener-capture-exact-limit",
              "firstExtraCase": "server-listener-capture-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "request-deadline",
          "minimum": 1,
          "maximum": 5000,
          "unit": "milliseconds",
          "contractId": "server-status-request-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-request-deadline-exact-limit",
              "firstExtraCase": "server-request-deadline-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-request-deadline-exact-limit",
              "firstExtraCase": "server-request-deadline-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "service-lifetime",
          "minimum": 1,
          "maximum": 30000,
          "unit": "milliseconds",
          "contractId": "server-status-service-lifetime",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-service-lifetime-exact-limit",
              "firstExtraCase": "server-service-lifetime-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-service-lifetime-exact-limit",
              "firstExtraCase": "server-service-lifetime-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "concurrent-requests",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "server-status-concurrent-requests",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-serial-request-policy-exact-limit",
              "firstExtraCase": "server-serial-request-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-status-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "guest-memory",
          "minimum": 0,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "server-status-guest-memory",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-memory-byte-policy-exact-limit",
              "firstExtraCase": "server-memory-byte-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-status-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "guest-fuel",
          "minimum": 0,
          "maximum": 100000,
          "unit": "fuel-units",
          "contractId": "server-status-guest-fuel",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-fuel-policy-exact-limit",
              "firstExtraCase": "server-fuel-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-status-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "guest-resources",
          "minimum": 0,
          "maximum": 5,
          "unit": "count",
          "contractId": "server-status-guest-resources",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-private-resource-quota-exact-limit",
              "firstExtraCase": "server-private-resource-quota-first-extra"
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "server-status-guest-resources-public-input-exclusion"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "guest-callbacks",
          "minimum": 0,
          "maximum": 8,
          "unit": "count",
          "contractId": "server-status-guest-callbacks",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-private-callback-quota-exact-limit",
              "firstExtraCase": "server-private-callback-quota-first-extra"
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "server-status-guest-callbacks-public-input-exclusion"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        },
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "server-status-component-bytes",
          "mapping": {
            "kind": "excluded",
            "exclusionId": "server-status-component-bytes-public-input-exclusion",
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-status-public"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "server-status-public",
          "rowIds": [
            "server-status-linux",
            "server-status-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-status-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "actual_server_status_boundaries_malformed_attempts_and_repeated_start",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "status200/599 empty bodies; malformed request rejected; served2/rejected1; empty grants; actual teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-status-denial",
          "rowIds": [
            "server-status-linux",
            "server-status-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-status-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "rejected_configuration_and_unapproved_listener_never_start_or_publish",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unapproved listener/input denied before readiness and publication"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-source-denial",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_server/source_tests.rs"
          ],
          "gateId": "case-server-source-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "source_tests::actual_source_effects_wrong_signatures_extra_exports_and_dependencies_reject_before_ready",
            "source": "apps/zryna/tests/wasi_server/source_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "source effects/signatures/extra exports/dependencies deny before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-cancel",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-cancel",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "readiness_cancellation_closes_actual_listener_and_commits_typed_teardown",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "cancel readiness; accepted0; actual listener closes; confirmed typed teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-forced-stop",
          "rowIds": [
            "server-status-linux",
            "server-status-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-forced-stop",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "forced_cli_termination_closes_listener_without_graceful_record",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "OS listener closes; no graceful record or cleanup certificate"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-body-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-body-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_body_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 65536 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "body-bytes",
            "value": 65536,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-body-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-body-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_body_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 65536 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "body-bytes",
            "value": 65537,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_complete_header_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 8192 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-bytes",
            "value": 8192,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_complete_header_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 8192 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-bytes",
            "value": 8193,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-fields-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-fields-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_header_fields_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 32 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-fields",
            "value": 32,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-fields-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-fields-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_header_fields_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 32 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-fields",
            "value": 33,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-path-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-path-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_path_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 256 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "path-bytes",
            "value": 256,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-path-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-path-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_path_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 256 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "path-bytes",
            "value": 257,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-attempts-exact",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-attempts-exact",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_attempt_budget_serves64_limit_intersections_and_closes_before_service_expiry",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "all64 intersecting maxima served;64 Stores and320 resources destroyed; actual joined listener closes before service expiry"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "attempts",
            "value": 64,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-lowered-envelopes",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/execution.rs"
          ],
          "gateId": "case-server-lowered-envelopes",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_runtime::tests::execution::actual_guest_resource_fuel_memory_and_callback_limits_trap_and_recover",
            "source": "crates/zryna-driver/src/server_runtime/tests/execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "resources3/fuel1/memory65535/callbacks5 lower fixture traps; no production-ceiling certificate"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "lowered-runtime-envelope"
        },
        {
          "id": "server-memory-growth",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs"
          ],
          "gateId": "case-server-memory-growth",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_runtime::tests::probe_execution::actual_guest_memory_growth_traps_at_one_page_and_destroys_store",
            "source": "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "real memory growth beyond one page traps; Store destroyed"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "server-configuration-negative",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-configuration-negative",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::negative::strict_configuration_rejects_unknown_duplicate_unbounded_and_nonloopback_values",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "duplicate/unknown/nonloopback and extra attempt/header/body/service limits rejected; no implicit exact request5000/service30000 acceptance"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "production-configuration-admission"
        },
        {
          "id": "server-component-replay",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests.rs"
          ],
          "gateId": "case-server-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::exact_world_empty_response_and_clock_arrangements_validate_and_replay",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "both exact arrangements replay byte-identically; original scalar core; distinct status/clock identities"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "server-request-deadline-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-request-deadline-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_request_deadline5000_and_first_extra5001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-deadline",
            "value": 5000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-request-deadline-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-request-deadline-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_request_deadline5000_and_first_extra5001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-deadline",
            "value": 5001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-service-lifetime-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-service-lifetime-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_service_deadline30000_and_first_extra30001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "service-lifetime",
            "value": 30000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-service-lifetime-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-service-lifetime-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_service_deadline30000_and_first_extra30001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "service-lifetime",
            "value": 30001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-public-envelope-binding",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-public-envelope-binding",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::public_production_resource_callback_and_fuel_envelope_executes_both_arrangements",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::envelope resources5/callbacks8/fuel100000; actual Reply/ClockRead202 emptyresponse and expected0/1reads; five resourcescreated/destroyed; reservationszero;joined shutdown; this proves using settings, not exhausting quotas"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "production-server-arrangement-under-public-envelope"
        },
        {
          "id": "server-private-resource-quota-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-private-resource-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "five actual registry owners; sixth rejected before allocation; releaseone/recover;actualdestruction; public fixed handler peaks4 so no publicguestexhaustion claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-resources",
            "value": 5,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-resource-registry"
        },
        {
          "id": "server-private-resource-quota-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-private-resource-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "five actual registry owners; sixth rejected before allocation; releaseone/recover;actualdestruction; public fixed handler peaks4 so no publicguestexhaustion claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-resources",
            "value": 6,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-resource-registry"
        },
        {
          "id": "server-private-callback-quota-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs"
          ],
          "gateId": "case-server-private-callback-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::callback_limits::private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth",
            "source": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "eight actual Host::enter policy invocations accept; ninth/further reject; unchanged Config::envelopecallbacks8; freshHostrecovery; zeroStore/worker/provider/response; no actualguestcallbacks"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-callbacks",
            "value": 8,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-Host-enter-policy"
        },
        {
          "id": "server-private-callback-quota-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs"
          ],
          "gateId": "case-server-private-callback-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::callback_limits::private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth",
            "source": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "eight actual Host::enter policy invocations accept; ninth/further reject; unchanged Config::envelopecallbacks8; freshHostrecovery; zeroStore/worker/provider/response; no actualguestcallbacks"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-callbacks",
            "value": 9,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-Host-enter-policy"
        },
        {
          "id": "server-public-shape-size-exclusion",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs"
          ],
          "gateId": "case-server-public-shape-size-exclusion",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::public_size_bound::public_single_scalar_size_upper_bound_is_strictly_below_component_ceiling",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "single no-arg i32 solefunction sourceboundshape;actual16383node/128bytename canonical artifact emitted/audited; bothReply/ClockRead;symbolic bound baseline+57343+256+256 strictlybelow1048576; source/IR/ABI/encoder/topology proofneeds independentreview+execution; NO exact1048576acceptance or authenticatedworkerpublicsource claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "conditional-public-shape-reachability-exclusion"
        },
        {
          "id": "server-attempts-first-extra-config",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-status-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "rejected_configuration_and_unapproved_listener_never_start_or_publish",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "attempts65 selection rejects before readiness/bundle; joins actual64 sharedproductiontransport only through an explicit mixedscope pair"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "attempts",
            "value": 65,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-component-first-extra-length-guard",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests.rs"
          ],
          "gateId": "case-server-component-first-extra-length-guard",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::malformed_oversized_and_extra_sections_do_not_reach_world_decoder",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual audit receives1048577zero-filled bytes and rejects; unchanged audit length guard executes before worlddecoder; vector is not structurally valid or canonical component; assertion is rejection only, no exact1048576acceptance"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-server-component-audit-length-guard"
        },
        {
          "id": "server-fuel-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-server-fuel-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::fuel_limits::private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion",
            "source": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged Config::envelope fuel100000; Envelope.validate100000 accepts/100001 ErrorLimit; actual measured unchanged productionmeter99999completedcalls then lastchargedcall traps at0 and nextOutOfFuel; fresh profile recovery; no publicguest100000successfulcalls"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-fuel",
            "value": 100000,
            "unit": "fuel-units",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-Envelope-fuel-admission"
        },
        {
          "id": "server-fuel-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-server-fuel-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::fuel_limits::private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion",
            "source": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged Config::envelope fuel100000; Envelope.validate100000 accepts/100001 ErrorLimit; actual measured unchanged productionmeter99999completedcalls then lastchargedcall traps at0 and nextOutOfFuel; fresh profile recovery; no publicguest100000successfulcalls"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-fuel",
            "value": 100001,
            "unit": "fuel-units",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-Envelope-fuel-admission"
        },
        {
          "id": "server-memory-byte-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-server-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::memory_policy::public_server_memory_envelope_accepts65536_and_rejects65537_before_host",
            "source": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-memory",
            "value": 65536,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
        },
        {
          "id": "server-memory-byte-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-server-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::memory_policy::public_server_memory_envelope_accepts65536_and_rejects65537_before_host",
            "source": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-memory",
            "value": 65537,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
        },
        {
          "id": "server-serial-request-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs"
          ],
          "gateId": "case-server-serial-request-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::retirement::public_serial::production_serial_one_request_accepts_first_rejects_second_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config.requests1 unchanged; private Prepared first actual paused Store retained; second admission rejected before anotherStore; release204 teardown/recovery; not TCPbacklog or public concurrent socket rejection"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "concurrent-requests",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-production-lifecycle-real-Store-admission"
        },
        {
          "id": "server-serial-request-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs"
          ],
          "gateId": "case-server-serial-request-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::retirement::public_serial::production_serial_one_request_accepts_first_rejects_second_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config.requests1 unchanged; private Prepared first actual paused Store retained; second admission rejected before anotherStore; release204 teardown/recovery; not TCPbacklog or public concurrent socket rejection"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "concurrent-requests",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-production-lifecycle-real-Store-admission"
        },
        {
          "id": "server-config-capture-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-config-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "configuration-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-config-capture-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-config-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "configuration-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-listener-capture-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-listener-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "listener-approval-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-listener-capture-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-listener-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "listener-approval-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-server-status-public",
        "case-server-status-denial",
        "case-server-source-denial",
        "case-server-cancel",
        "case-server-forced-stop",
        "case-server-body-bytes-exact-limit",
        "case-server-header-bytes-exact-limit",
        "case-server-header-fields-exact-limit",
        "case-server-path-bytes-exact-limit",
        "case-server-attempts-exact",
        "case-server-lowered-envelopes",
        "case-server-memory-growth",
        "case-server-configuration-negative",
        "case-server-component-replay",
        "case-wit-closure",
        "case-server-request-deadline-exact-limit",
        "case-server-service-lifetime-exact-limit",
        "case-server-public-envelope-binding",
        "case-server-private-resource-quota-exact-limit",
        "case-server-private-callback-quota-exact-limit",
        "case-server-public-shape-size-exclusion",
        "case-server-component-first-extra-length-guard",
        "case-server-fuel-policy-exact-limit",
        "case-server-memory-byte-policy-exact-limit",
        "case-server-serial-request-policy-exact-limit",
        "case-server-config-capture-exact-limit",
        "case-server-listener-capture-exact-limit"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces"
      ]
    },
    {
      "id": "server-clock-linux",
      "profile": "server-clock-status-v1",
      "mode": "checkout-serve",
      "host": {
        "os": "linux",
        "architecture": "x64",
        "baseline": "ubuntu-latest:observed-baseline-required",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Wasmtime48.0.1"
      },
      "ownerIssues": [
        401
      ],
      "contract": "docs/WASI_SERVER_CLOCK_STATUS_V1.md",
      "selector": {
        "command": "serve",
        "profile": "server-clock-status-v1",
        "target": null,
        "signature": "sole no-argument i32 export; status200..599; empty response"
      },
      "identities": {
        "world": "zryna:capability-profiles/server@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.server-empty-response.v1",
        "manifest": "zryna.wasi-server-manifest.v2",
        "manifestVersion": 2,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "coreSha256",
          "bridgeRevision",
          "preparationIdentity",
          "configurationSha256",
          "listenerApprovalIdentity",
          "guestRequestSha256",
          "guestApprovalIdentity",
          "actualClockReads",
          "actualStoreCount"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [
          "clock:monotonic-now:one-read-per-Store"
        ],
        "effective": [
          "clock:monotonic-now:one-read-per-Store"
        ],
        "rootApprovals": [
          "listener-configuration-digest",
          "separate-guest-request-digest"
        ],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "environment"
        ],
        "operationLimits": "one monotonic read per admitted Store; no subscriptions/timers/wall clock"
      },
      "artifacts": [
        {
          "path": "component/{stem}.wasm",
          "kind": "webassembly-component"
        },
        {
          "path": "zryna-wasi-server-manifest-v2.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "body-bytes",
          "minimum": 1,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "server-clock-body-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-body-bytes-exact-limit",
              "firstExtraCase": "server-body-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-body-bytes-exact-limit",
              "firstExtraCase": "server-body-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "header-bytes",
          "minimum": 64,
          "maximum": 8192,
          "unit": "bytes",
          "contractId": "server-clock-header-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-header-bytes-exact-limit",
              "firstExtraCase": "server-header-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-header-bytes-exact-limit",
              "firstExtraCase": "server-header-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "header-fields",
          "minimum": 0,
          "maximum": 32,
          "unit": "count",
          "contractId": "server-clock-header-fields",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-header-fields-exact-limit",
              "firstExtraCase": "server-header-fields-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-header-fields-exact-limit",
              "firstExtraCase": "server-header-fields-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "path-bytes",
          "minimum": 0,
          "maximum": 256,
          "unit": "bytes",
          "contractId": "server-clock-path-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-path-bytes-exact-limit",
              "firstExtraCase": "server-path-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-path-bytes-exact-limit",
              "firstExtraCase": "server-path-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "attempts",
          "minimum": 1,
          "maximum": 64,
          "unit": "count",
          "contractId": "server-clock-attempts",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-attempts-exact",
              "firstExtraCase": "server-attempts-first-extra-config"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-attempts-exact",
              "firstExtraCase": "server-attempts-first-extra-config"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "configuration-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-clock-configuration-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-config-capture-exact-limit",
              "firstExtraCase": "server-config-capture-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-config-capture-exact-limit",
              "firstExtraCase": "server-config-capture-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "listener-approval-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-clock-listener-approval-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-listener-capture-exact-limit",
              "firstExtraCase": "server-listener-capture-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-listener-capture-exact-limit",
              "firstExtraCase": "server-listener-capture-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "request-deadline",
          "minimum": 1,
          "maximum": 5000,
          "unit": "milliseconds",
          "contractId": "server-clock-request-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-request-deadline-exact-limit",
              "firstExtraCase": "server-request-deadline-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-request-deadline-exact-limit",
              "firstExtraCase": "server-request-deadline-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "service-lifetime",
          "minimum": 1,
          "maximum": 30000,
          "unit": "milliseconds",
          "contractId": "server-clock-service-lifetime",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-service-lifetime-exact-limit",
              "firstExtraCase": "server-service-lifetime-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-service-lifetime-exact-limit",
              "firstExtraCase": "server-service-lifetime-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "concurrent-requests",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "server-clock-concurrent-requests",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-serial-request-policy-exact-limit",
              "firstExtraCase": "server-serial-request-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-clock-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-memory",
          "minimum": 0,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "server-clock-guest-memory",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-memory-byte-policy-exact-limit",
              "firstExtraCase": "server-memory-byte-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-clock-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-fuel",
          "minimum": 0,
          "maximum": 100000,
          "unit": "fuel-units",
          "contractId": "server-clock-guest-fuel",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-fuel-policy-exact-limit",
              "firstExtraCase": "server-fuel-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-clock-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-resources",
          "minimum": 0,
          "maximum": 5,
          "unit": "count",
          "contractId": "server-clock-guest-resources",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-private-resource-quota-exact-limit",
              "firstExtraCase": "server-private-resource-quota-first-extra"
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "server-clock-guest-resources-public-input-exclusion"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-callbacks",
          "minimum": 0,
          "maximum": 8,
          "unit": "count",
          "contractId": "server-clock-guest-callbacks",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-private-callback-quota-exact-limit",
              "firstExtraCase": "server-private-callback-quota-first-extra"
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "server-clock-guest-callbacks-public-input-exclusion"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "server-clock-component-bytes",
          "mapping": {
            "kind": "excluded",
            "exclusionId": "server-clock-component-bytes-public-input-exclusion",
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-request-bytes",
          "minimum": 1,
          "maximum": 2048,
          "unit": "bytes",
          "contractId": "server-clock-guest-request-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-guest-request-bytes-exact-limit",
              "firstExtraCase": "server-guest-request-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-guest-request-bytes-exact-limit",
              "firstExtraCase": "server-guest-request-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-approval-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-clock-guest-approval-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-guest-approval-bytes-exact-limit",
              "firstExtraCase": "server-guest-approval-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-guest-approval-bytes-exact-limit",
              "firstExtraCase": "server-guest-approval-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "clock-reads",
          "minimum": 1,
          "maximum": 1,
          "unit": "count",
          "contractId": "server-clock-clock-reads",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-clock-operation-policy-exact-limit",
              "firstExtraCase": "server-clock-operation-policy-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-clock-request-quota-exact",
              "firstExtraCase": "server-clock-request-quota-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "server-source-denial",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_server/source_tests.rs"
          ],
          "gateId": "case-server-source-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "source_tests::actual_source_effects_wrong_signatures_extra_exports_and_dependencies_reject_before_ready",
            "source": "apps/zryna/tests/wasi_server/source_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "source effects/signatures/extra exports/dependencies deny before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-cancel",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-cancel",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "readiness_cancellation_closes_actual_listener_and_commits_typed_teardown",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "cancel readiness; accepted0; actual listener closes; confirmed typed teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-clock-public",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::granted_clock_profile_executes_one_read_per_store_and_records_actual_teardown",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "one clock read per admitted Store; timestamp discarded; actual v2 teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-clock-denial",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::malformed_or_unapproved_guest_inputs_and_cross_profile_flags_reject_before_ready",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "guest request/root approval separate; malformed/unapproved/cross-profile denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-clock-no-intrinsic",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-no-intrinsic",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::clock_approval_does_not_admit_a_source_language_clock_intrinsic",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "root approved fixed component arrangement adds no source clock intrinsic"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-clock-idle",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-idle",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::approved_clock_without_guest_execution_records_zero_reads_on_cancel_and_deadline",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "approved authority alone: zero admitted Stores and zero reads on cancellation/deadline"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-clock-revocation",
          "rowIds": [
            "server-clock-linux"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-revocation",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_replacement_and_approval_revocation_stop_service_without_record",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "original guest input replacement/revocation suppresses response/final record"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-guest-request-bytes-exact-limit",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-guest-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_and_approval_byte_ceilings_accept_exact_and_reject_first_extra",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "inclusive caller-private 2048-byte file accepted; first extra denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-request-bytes",
            "value": 2048,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-guest-request-bytes-first-extra",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-guest-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_and_approval_byte_ceilings_accept_exact_and_reject_first_extra",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "inclusive caller-private 2048-byte file accepted; first extra denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-request-bytes",
            "value": 2049,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-guest-approval-bytes-exact-limit",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-guest-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_and_approval_byte_ceilings_accept_exact_and_reject_first_extra",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "inclusive caller-private 1024-byte file accepted; first extra denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-approval-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-guest-approval-bytes-first-extra",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-guest-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_and_approval_byte_ceilings_accept_exact_and_reject_first_extra",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "inclusive caller-private 1024-byte file accepted; first extra denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-approval-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-body-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-body-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_body_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 65536 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "body-bytes",
            "value": 65536,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-body-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-body-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_body_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 65536 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "body-bytes",
            "value": 65537,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_complete_header_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 8192 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-bytes",
            "value": 8192,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_complete_header_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 8192 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-bytes",
            "value": 8193,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-fields-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-fields-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_header_fields_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 32 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-fields",
            "value": 32,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-fields-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-fields-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_header_fields_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 32 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-fields",
            "value": 33,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-path-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-path-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_path_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 256 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "path-bytes",
            "value": 256,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-path-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-path-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_path_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 256 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "path-bytes",
            "value": 257,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-attempts-exact",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-attempts-exact",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_attempt_budget_serves64_limit_intersections_and_closes_before_service_expiry",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "all64 intersecting maxima served;64 Stores and320 resources destroyed; actual joined listener closes before service expiry"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "attempts",
            "value": 64,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-lowered-envelopes",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/execution.rs"
          ],
          "gateId": "case-server-lowered-envelopes",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_runtime::tests::execution::actual_guest_resource_fuel_memory_and_callback_limits_trap_and_recover",
            "source": "crates/zryna-driver/src/server_runtime/tests/execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "resources3/fuel1/memory65535/callbacks5 lower fixture traps; no production-ceiling certificate"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "lowered-runtime-envelope"
        },
        {
          "id": "server-memory-growth",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs"
          ],
          "gateId": "case-server-memory-growth",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_runtime::tests::probe_execution::actual_guest_memory_growth_traps_at_one_page_and_destroys_store",
            "source": "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "real memory growth beyond one page traps; Store destroyed"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "server-configuration-negative",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-configuration-negative",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::negative::strict_configuration_rejects_unknown_duplicate_unbounded_and_nonloopback_values",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "duplicate/unknown/nonloopback and extra attempt/header/body/service limits rejected; no implicit exact request5000/service30000 acceptance"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "production-configuration-admission"
        },
        {
          "id": "server-component-replay",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests.rs"
          ],
          "gateId": "case-server-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::exact_world_empty_response_and_clock_arrangements_validate_and_replay",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "both exact arrangements replay byte-identically; original scalar core; distinct status/clock identities"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "server-request-deadline-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-request-deadline-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_request_deadline5000_and_first_extra5001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-deadline",
            "value": 5000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-request-deadline-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-request-deadline-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_request_deadline5000_and_first_extra5001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-deadline",
            "value": 5001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-service-lifetime-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-service-lifetime-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_service_deadline30000_and_first_extra30001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "service-lifetime",
            "value": 30000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-service-lifetime-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-service-lifetime-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_service_deadline30000_and_first_extra30001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "service-lifetime",
            "value": 30001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-public-envelope-binding",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-public-envelope-binding",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::public_production_resource_callback_and_fuel_envelope_executes_both_arrangements",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::envelope resources5/callbacks8/fuel100000; actual Reply/ClockRead202 emptyresponse and expected0/1reads; five resourcescreated/destroyed; reservationszero;joined shutdown; this proves using settings, not exhausting quotas"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "production-server-arrangement-under-public-envelope"
        },
        {
          "id": "server-private-resource-quota-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-private-resource-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "five actual registry owners; sixth rejected before allocation; releaseone/recover;actualdestruction; public fixed handler peaks4 so no publicguestexhaustion claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-resources",
            "value": 5,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-resource-registry"
        },
        {
          "id": "server-private-resource-quota-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-private-resource-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "five actual registry owners; sixth rejected before allocation; releaseone/recover;actualdestruction; public fixed handler peaks4 so no publicguestexhaustion claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-resources",
            "value": 6,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-resource-registry"
        },
        {
          "id": "server-private-callback-quota-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs"
          ],
          "gateId": "case-server-private-callback-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::callback_limits::private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth",
            "source": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "eight actual Host::enter policy invocations accept; ninth/further reject; unchanged Config::envelopecallbacks8; freshHostrecovery; zeroStore/worker/provider/response; no actualguestcallbacks"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-callbacks",
            "value": 8,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-Host-enter-policy"
        },
        {
          "id": "server-private-callback-quota-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs"
          ],
          "gateId": "case-server-private-callback-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::callback_limits::private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth",
            "source": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "eight actual Host::enter policy invocations accept; ninth/further reject; unchanged Config::envelopecallbacks8; freshHostrecovery; zeroStore/worker/provider/response; no actualguestcallbacks"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-callbacks",
            "value": 9,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-Host-enter-policy"
        },
        {
          "id": "server-public-shape-size-exclusion",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs"
          ],
          "gateId": "case-server-public-shape-size-exclusion",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::public_size_bound::public_single_scalar_size_upper_bound_is_strictly_below_component_ceiling",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "single no-arg i32 solefunction sourceboundshape;actual16383node/128bytename canonical artifact emitted/audited; bothReply/ClockRead;symbolic bound baseline+57343+256+256 strictlybelow1048576; source/IR/ABI/encoder/topology proofneeds independentreview+execution; NO exact1048576acceptance or authenticatedworkerpublicsource claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "conditional-public-shape-reachability-exclusion"
        },
        {
          "id": "server-attempts-first-extra-config",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-status-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "rejected_configuration_and_unapproved_listener_never_start_or_publish",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "attempts65 selection rejects before readiness/bundle; joins actual64 sharedproductiontransport only through an explicit mixedscope pair"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "attempts",
            "value": 65,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-clock-request-quota-exact",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::granted_clock_profile_executes_one_read_per_store_and_records_actual_teardown",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "approved guestrequest monotonic_reads1; actual eachStore atmost1read; observedStore/read counters; joinedteardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "clock-reads",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "public-server-clock-request-admission-and-execution"
        },
        {
          "id": "server-clock-request-quota-first-extra",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::malformed_or_unapproved_guest_inputs_and_cross_profile_flags_reject_before_ready",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "guestrequest monotonic_reads2 rejected before readiness; distinct rootapproval2 rejection also existing matching_digest case; no fabricatedsecondactualread"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "clock-reads",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "public-server-clock-request-admission"
        },
        {
          "id": "server-component-first-extra-length-guard",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests.rs"
          ],
          "gateId": "case-server-component-first-extra-length-guard",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::malformed_oversized_and_extra_sections_do_not_reach_world_decoder",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual audit receives1048577zero-filled bytes and rejects; unchanged audit length guard executes before worlddecoder; vector is not structurally valid or canonical component; assertion is rejection only, no exact1048576acceptance"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-server-component-audit-length-guard"
        },
        {
          "id": "server-fuel-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-server-fuel-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::fuel_limits::private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion",
            "source": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged Config::envelope fuel100000; Envelope.validate100000 accepts/100001 ErrorLimit; actual measured unchanged productionmeter99999completedcalls then lastchargedcall traps at0 and nextOutOfFuel; fresh profile recovery; no publicguest100000successfulcalls"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-fuel",
            "value": 100000,
            "unit": "fuel-units",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-Envelope-fuel-admission"
        },
        {
          "id": "server-fuel-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-server-fuel-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::fuel_limits::private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion",
            "source": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged Config::envelope fuel100000; Envelope.validate100000 accepts/100001 ErrorLimit; actual measured unchanged productionmeter99999completedcalls then lastchargedcall traps at0 and nextOutOfFuel; fresh profile recovery; no publicguest100000successfulcalls"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-fuel",
            "value": 100001,
            "unit": "fuel-units",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-Envelope-fuel-admission"
        },
        {
          "id": "server-memory-byte-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-server-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::memory_policy::public_server_memory_envelope_accepts65536_and_rejects65537_before_host",
            "source": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-memory",
            "value": 65536,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
        },
        {
          "id": "server-memory-byte-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-server-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::memory_policy::public_server_memory_envelope_accepts65536_and_rejects65537_before_host",
            "source": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-memory",
            "value": 65537,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
        },
        {
          "id": "server-serial-request-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs"
          ],
          "gateId": "case-server-serial-request-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::retirement::public_serial::production_serial_one_request_accepts_first_rejects_second_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config.requests1 unchanged; private Prepared first actual paused Store retained; second admission rejected before anotherStore; release204 teardown/recovery; not TCPbacklog or public concurrent socket rejection"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "concurrent-requests",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-production-lifecycle-real-Store-admission"
        },
        {
          "id": "server-serial-request-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs"
          ],
          "gateId": "case-server-serial-request-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::retirement::public_serial::production_serial_one_request_accepts_first_rejects_second_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config.requests1 unchanged; private Prepared first actual paused Store retained; second admission rejected before anotherStore; release204 teardown/recovery; not TCPbacklog or public concurrent socket rejection"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "concurrent-requests",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-production-lifecycle-real-Store-admission"
        },
        {
          "id": "server-clock-operation-policy-exact-limit",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs"
          ],
          "gateId": "case-server-clock-operation-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::clock_quota::private_clock_operation_one_read_then_second_limit_preserves_unpublished_output",
            "source": "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual operations.invoke/Host.clock under effective grant1; first actualprovider read; second Limit before outputslot/provider counter; live Store balances/destructs/fresh recovery; not second publicWasm callback"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "clock-reads",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-host-operation-real-Store-clock-quota"
        },
        {
          "id": "server-clock-operation-policy-first-extra",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs"
          ],
          "gateId": "case-server-clock-operation-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::clock_quota::private_clock_operation_one_read_then_second_limit_preserves_unpublished_output",
            "source": "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual operations.invoke/Host.clock under effective grant1; first actualprovider read; second Limit before outputslot/provider counter; live Store balances/destructs/fresh recovery; not second publicWasm callback"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "clock-reads",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-host-operation-real-Store-clock-quota"
        },
        {
          "id": "server-config-capture-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-config-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "configuration-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-config-capture-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-config-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "configuration-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-listener-capture-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-listener-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "listener-approval-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-listener-capture-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-listener-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "listener-approval-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-server-source-denial",
        "case-server-cancel",
        "case-server-clock-public",
        "case-server-clock-denial",
        "case-server-clock-no-intrinsic",
        "case-server-clock-idle",
        "case-server-clock-revocation",
        "case-server-guest-request-bytes-exact-limit",
        "case-server-body-bytes-exact-limit",
        "case-server-header-bytes-exact-limit",
        "case-server-header-fields-exact-limit",
        "case-server-path-bytes-exact-limit",
        "case-server-attempts-exact",
        "case-server-lowered-envelopes",
        "case-server-memory-growth",
        "case-server-configuration-negative",
        "case-server-component-replay",
        "case-wit-closure",
        "case-server-request-deadline-exact-limit",
        "case-server-service-lifetime-exact-limit",
        "case-server-public-envelope-binding",
        "case-server-private-resource-quota-exact-limit",
        "case-server-private-callback-quota-exact-limit",
        "case-server-public-shape-size-exclusion",
        "case-server-status-denial",
        "case-server-component-first-extra-length-guard",
        "case-server-fuel-policy-exact-limit",
        "case-server-memory-byte-policy-exact-limit",
        "case-server-serial-request-policy-exact-limit",
        "case-server-clock-operation-policy-exact-limit",
        "case-server-config-capture-exact-limit",
        "case-server-listener-capture-exact-limit"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces"
      ]
    },
    {
      "id": "server-clock-windows",
      "profile": "server-clock-status-v1",
      "mode": "checkout-serve",
      "host": {
        "os": "win32",
        "architecture": "x64",
        "baseline": "windows-latest:observed-baseline-required",
        "portabilityClaim": "exact-hosted-baseline-only",
        "consumer": "Wasmtime48.0.1"
      },
      "ownerIssues": [
        401
      ],
      "contract": "docs/WASI_SERVER_CLOCK_STATUS_V1.md",
      "selector": {
        "command": "serve",
        "profile": "server-clock-status-v1",
        "target": null,
        "signature": "sole no-argument i32 export; status200..599; empty response"
      },
      "identities": {
        "world": "zryna:capability-profiles/server@0.1.0",
        "witClosure": [
          "spec/wit/capability-profiles-v1/worlds.wit",
          "wasi/cli/command.wit",
          "wasi/cli/environment.wit",
          "wasi/cli/exit.wit",
          "wasi/cli/imports.wit",
          "wasi/cli/run.wit",
          "wasi/cli/stdio.wit",
          "wasi/cli/terminal.wit",
          "wasi/clocks/monotonic-clock.wit",
          "wasi/clocks/timezone.wit",
          "wasi/clocks/wall-clock.wit",
          "wasi/clocks/world.wit",
          "wasi/filesystem/preopens.wit",
          "wasi/filesystem/types.wit",
          "wasi/filesystem/world.wit",
          "wasi/http/handler.wit",
          "wasi/http/proxy.wit",
          "wasi/http/types.wit",
          "wasi/io/error.wit",
          "wasi/io/poll.wit",
          "wasi/io/streams.wit",
          "wasi/io/world.wit",
          "wasi/random/insecure-seed.wit",
          "wasi/random/insecure.wit",
          "wasi/random/random.wit",
          "wasi/random/world.wit",
          "wasi/sockets/instance-network.wit",
          "wasi/sockets/ip-name-lookup.wit",
          "wasi/sockets/network.wit",
          "wasi/sockets/tcp-create-socket.wit",
          "wasi/sockets/tcp.wit",
          "wasi/sockets/udp-create-socket.wit",
          "wasi/sockets/udp.wit",
          "wasi/sockets/world.wit"
        ],
        "bindingRevision": "zryna.server-empty-response.v1",
        "manifest": "zryna.wasi-server-manifest.v2",
        "manifestVersion": 2,
        "receiptRequiredFields": [
          "sourceSha256",
          "componentSha256",
          "witClosureDigest",
          "manifestSha256",
          "exactBundleInventory",
          "coreSha256",
          "bridgeRevision",
          "preparationIdentity",
          "configurationSha256",
          "listenerApprovalIdentity",
          "guestRequestSha256",
          "guestApprovalIdentity",
          "actualClockReads",
          "actualStoreCount"
        ]
      },
      "capabilities": {
        "eligibility": "specified-only-world-imports-never-effective-grants",
        "requested": [
          "clock:monotonic-now:one-read-per-Store"
        ],
        "effective": [
          "clock:monotonic-now:one-read-per-Store"
        ],
        "rootApprovals": [
          "listener-configuration-digest",
          "separate-guest-request-digest"
        ],
        "denied": [
          "filesystem",
          "outgoing-network",
          "randomness",
          "wall-clock",
          "timers",
          "subscriptions",
          "process",
          "ambient-environment",
          "guest-argv",
          "guest-cwd",
          "environment"
        ],
        "operationLimits": "one monotonic read per admitted Store; no subscriptions/timers/wall clock"
      },
      "artifacts": [
        {
          "path": "component/{stem}.wasm",
          "kind": "webassembly-component"
        },
        {
          "path": "zryna-wasi-server-manifest-v2.json",
          "kind": "manifest"
        }
      ],
      "limits": [
        {
          "id": "body-bytes",
          "minimum": 1,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "server-clock-body-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-body-bytes-exact-limit",
              "firstExtraCase": "server-body-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-body-bytes-exact-limit",
              "firstExtraCase": "server-body-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "header-bytes",
          "minimum": 64,
          "maximum": 8192,
          "unit": "bytes",
          "contractId": "server-clock-header-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-header-bytes-exact-limit",
              "firstExtraCase": "server-header-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-header-bytes-exact-limit",
              "firstExtraCase": "server-header-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "header-fields",
          "minimum": 0,
          "maximum": 32,
          "unit": "count",
          "contractId": "server-clock-header-fields",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-header-fields-exact-limit",
              "firstExtraCase": "server-header-fields-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-header-fields-exact-limit",
              "firstExtraCase": "server-header-fields-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "path-bytes",
          "minimum": 0,
          "maximum": 256,
          "unit": "bytes",
          "contractId": "server-clock-path-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-path-bytes-exact-limit",
              "firstExtraCase": "server-path-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-path-bytes-exact-limit",
              "firstExtraCase": "server-path-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "attempts",
          "minimum": 1,
          "maximum": 64,
          "unit": "count",
          "contractId": "server-clock-attempts",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-attempts-exact",
              "firstExtraCase": "server-attempts-first-extra-config"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-attempts-exact",
              "firstExtraCase": "server-attempts-first-extra-config"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "configuration-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-clock-configuration-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-config-capture-exact-limit",
              "firstExtraCase": "server-config-capture-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-config-capture-exact-limit",
              "firstExtraCase": "server-config-capture-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "listener-approval-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-clock-listener-approval-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-listener-capture-exact-limit",
              "firstExtraCase": "server-listener-capture-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-listener-capture-exact-limit",
              "firstExtraCase": "server-listener-capture-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "request-deadline",
          "minimum": 1,
          "maximum": 5000,
          "unit": "milliseconds",
          "contractId": "server-clock-request-deadline",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-request-deadline-exact-limit",
              "firstExtraCase": "server-request-deadline-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-request-deadline-exact-limit",
              "firstExtraCase": "server-request-deadline-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "service-lifetime",
          "minimum": 1,
          "maximum": 30000,
          "unit": "milliseconds",
          "contractId": "server-clock-service-lifetime",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-service-lifetime-exact-limit",
              "firstExtraCase": "server-service-lifetime-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-service-lifetime-exact-limit",
              "firstExtraCase": "server-service-lifetime-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "concurrent-requests",
          "minimum": 0,
          "maximum": 1,
          "unit": "count",
          "contractId": "server-clock-concurrent-requests",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-serial-request-policy-exact-limit",
              "firstExtraCase": "server-serial-request-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-clock-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-memory",
          "minimum": 0,
          "maximum": 65536,
          "unit": "bytes",
          "contractId": "server-clock-guest-memory",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-memory-byte-policy-exact-limit",
              "firstExtraCase": "server-memory-byte-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-clock-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-fuel",
          "minimum": 0,
          "maximum": 100000,
          "unit": "fuel-units",
          "contractId": "server-clock-guest-fuel",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-fuel-policy-exact-limit",
              "firstExtraCase": "server-fuel-policy-first-extra"
            },
            "public": {
              "kind": "guard-prerequisite",
              "caseIds": [
                "server-public-envelope-binding",
                "server-clock-public"
              ]
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-resources",
          "minimum": 0,
          "maximum": 5,
          "unit": "count",
          "contractId": "server-clock-guest-resources",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-private-resource-quota-exact-limit",
              "firstExtraCase": "server-private-resource-quota-first-extra"
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "server-clock-guest-resources-public-input-exclusion"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-callbacks",
          "minimum": 0,
          "maximum": 8,
          "unit": "count",
          "contractId": "server-clock-guest-callbacks",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-private-callback-quota-exact-limit",
              "firstExtraCase": "server-private-callback-quota-first-extra"
            },
            "public": {
              "kind": "exclusion",
              "exclusionId": "server-clock-guest-callbacks-public-input-exclusion"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "component-bytes",
          "minimum": 0,
          "maximum": 1048576,
          "unit": "bytes",
          "contractId": "server-clock-component-bytes",
          "mapping": {
            "kind": "excluded",
            "exclusionId": "server-clock-component-bytes-public-input-exclusion",
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-request-bytes",
          "minimum": 1,
          "maximum": 2048,
          "unit": "bytes",
          "contractId": "server-clock-guest-request-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-guest-request-bytes-exact-limit",
              "firstExtraCase": "server-guest-request-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-guest-request-bytes-exact-limit",
              "firstExtraCase": "server-guest-request-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "guest-approval-bytes",
          "minimum": 1,
          "maximum": 1024,
          "unit": "bytes",
          "contractId": "server-clock-guest-approval-bytes",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-guest-approval-bytes-exact-limit",
              "firstExtraCase": "server-guest-approval-bytes-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-guest-approval-bytes-exact-limit",
              "firstExtraCase": "server-guest-approval-bytes-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        },
        {
          "id": "clock-reads",
          "minimum": 1,
          "maximum": 1,
          "unit": "count",
          "contractId": "server-clock-clock-reads",
          "mapping": {
            "kind": "mapped",
            "guard": {
              "kind": "pair",
              "exactCase": "server-clock-operation-policy-exact-limit",
              "firstExtraCase": "server-clock-operation-policy-first-extra"
            },
            "public": {
              "kind": "pair",
              "exactCase": "server-clock-request-quota-exact",
              "firstExtraCase": "server-clock-request-quota-first-extra"
            },
            "bindingCaseIds": [
              "server-public-envelope-binding",
              "server-clock-public"
            ]
          }
        }
      ],
      "proofCases": [
        {
          "id": "server-source-denial",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_server/source_tests.rs"
          ],
          "gateId": "case-server-source-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "source_tests::actual_source_effects_wrong_signatures_extra_exports_and_dependencies_reject_before_ready",
            "source": "apps/zryna/tests/wasi_server/source_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "source effects/signatures/extra exports/dependencies deny before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-cancel",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-cancel",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "readiness_cancellation_closes_actual_listener_and_commits_typed_teardown",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "cancel readiness; accepted0; actual listener closes; confirmed typed teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server"
        },
        {
          "id": "server-clock-public",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "positive",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::granted_clock_profile_executes_one_read_per_store_and_records_actual_teardown",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "one clock read per admitted Store; timestamp discarded; actual v2 teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-clock-denial",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "capability-denial",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::malformed_or_unapproved_guest_inputs_and_cross_profile_flags_reject_before_ready",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "guest request/root approval separate; malformed/unapproved/cross-profile denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-clock-no-intrinsic",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-no-intrinsic",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::clock_approval_does_not_admit_a_source_language_clock_intrinsic",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "root approved fixed component arrangement adds no source clock intrinsic"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-clock-idle",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-idle",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::approved_clock_without_guest_execution_records_zero_reads_on_cancel_and_deadline",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "approved authority alone: zero admitted Stores and zero reads on cancellation/deadline"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-clock-retained-inputs-windows",
          "rowIds": [
            "server-clock-windows"
          ],
          "kind": "lifecycle",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-retained-inputs-windows",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::retained_guest_inputs_block_writes_and_replacement_then_keep_exact_permission",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "retained guest request and approval reject writes and replacement on Windows; admitted request serves HTTP 200; exactly one actual guest clock read; confirmed teardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-guest-request-bytes-exact-limit",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-guest-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_and_approval_byte_ceilings_accept_exact_and_reject_first_extra",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "inclusive caller-private 2048-byte file accepted; first extra denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-request-bytes",
            "value": 2048,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-guest-request-bytes-first-extra",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-guest-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_and_approval_byte_ceilings_accept_exact_and_reject_first_extra",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "inclusive caller-private 2048-byte file accepted; first extra denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-request-bytes",
            "value": 2049,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-guest-approval-bytes-exact-limit",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-guest-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_and_approval_byte_ceilings_accept_exact_and_reject_first_extra",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "inclusive caller-private 1024-byte file accepted; first extra denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-approval-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-guest-approval-bytes-first-extra",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-guest-request-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::guest_request_and_approval_byte_ceilings_accept_exact_and_reject_first_extra",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "inclusive caller-private 1024-byte file accepted; first extra denied before readiness"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-approval-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-checkout-server-clock"
        },
        {
          "id": "server-body-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-body-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_body_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 65536 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "body-bytes",
            "value": 65536,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-body-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-body-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_body_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 65536 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "body-bytes",
            "value": 65537,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_complete_header_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 8192 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-bytes",
            "value": 8192,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_complete_header_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 8192 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-bytes",
            "value": 8193,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-fields-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-fields-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_header_fields_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 32 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-fields",
            "value": 32,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-header-fields-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-header-fields-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_header_fields_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 32 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "header-fields",
            "value": 33,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-path-bytes-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-path-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_path_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 256 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "path-bytes",
            "value": 256,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-path-bytes-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-path-bytes-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_path_and_first_extra_reject_then_recover_with_real_store_cleanup",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "intersecting production maxima; 256 accepted with real Store cleanup; first extra creates zero Stores; maximum recovery; actual owned inputs/resources/listener closed"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "path-bytes",
            "value": 257,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-attempts-exact",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/boundary_tests.rs"
          ],
          "gateId": "case-server-attempts-exact",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::boundaries::maximum_attempt_budget_serves64_limit_intersections_and_closes_before_service_expiry",
            "source": "crates/zryna-driver/src/server_transport/boundary_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "all64 intersecting maxima served;64 Stores and320 resources destroyed; actual joined listener closes before service expiry"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "attempts",
            "value": 64,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "production-loopback-transport"
        },
        {
          "id": "server-lowered-envelopes",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/execution.rs"
          ],
          "gateId": "case-server-lowered-envelopes",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_runtime::tests::execution::actual_guest_resource_fuel_memory_and_callback_limits_trap_and_recover",
            "source": "crates/zryna-driver/src/server_runtime/tests/execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "resources3/fuel1/memory65535/callbacks5 lower fixture traps; no production-ceiling certificate"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "lowered-runtime-envelope"
        },
        {
          "id": "server-memory-growth",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs"
          ],
          "gateId": "case-server-memory-growth",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_runtime::tests::probe_execution::actual_guest_memory_growth_traps_at_one_page_and_destroys_store",
            "source": "crates/zryna-driver/src/server_runtime/tests/probe_execution.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "real memory growth beyond one page traps; Store destroyed"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "hostile-private-component"
        },
        {
          "id": "server-configuration-negative",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "negative",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-configuration-negative",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "server_runtime",
            "selector": "server_transport::tests::negative::strict_configuration_rejects_unknown_duplicate_unbounded_and_nonloopback_values",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "duplicate/unknown/nonloopback and extra attempt/header/body/service limits rejected; no implicit exact request5000/service30000 acceptance"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "production-configuration-admission"
        },
        {
          "id": "server-component-replay",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "reproducibility",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests.rs"
          ],
          "gateId": "case-server-component-replay",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::exact_world_empty_response_and_clock_arrangements_validate_and_replay",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "both exact arrangements replay byte-identically; original scalar core; distinct status/clock identities"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "component-audit"
        },
        {
          "id": "wit-closure",
          "rowIds": [
            "component-linux",
            "component-windows",
            "browser-linux",
            "browser-windows",
            "command-linux",
            "command-windows",
            "installed-command-linux",
            "installed-command-windows",
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs"
          ],
          "gateId": "case-wit-closure",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "wit_world_audit::tests::independent_baseline_resolves_and_matches_the_exact_world_closure",
            "source": "crates/zryna-backend-webassembly/src/wit_world_audit/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "complete34 source closure; exact8 packages/3 worlds; parser dependencies grant no host authority"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "specified-wit-prerequisite"
        },
        {
          "id": "server-request-deadline-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-request-deadline-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_request_deadline5000_and_first_extra5001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-deadline",
            "value": 5000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-request-deadline-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-request-deadline-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_request_deadline5000_and_first_extra5001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "request-deadline",
            "value": 5001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-service-lifetime-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-service-lifetime-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_service_deadline30000_and_first_extra30001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "service-lifetime",
            "value": 30000,
            "unit": "milliseconds",
            "decision": "accept"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-service-lifetime-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_transport/negative_tests.rs"
          ],
          "gateId": "case-server-service-lifetime-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_transport::tests::negative::public_service_deadline30000_and_first_extra30001_configuration_admission",
            "source": "crates/zryna-driver/src/server_transport/negative_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::parse exact option accepted; first extra option Error::Config; no listener/guest execution in these two probes; existing maximum productiontransport gates separately bind admitted5000/30000"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "service-lifetime",
            "value": 30001,
            "unit": "milliseconds",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-public-envelope-binding",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "positive",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-public-envelope-binding",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::public_production_resource_callback_and_fuel_envelope_executes_both_arrangements",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config::envelope resources5/callbacks8/fuel100000; actual Reply/ClockRead202 emptyresponse and expected0/1reads; five resourcescreated/destroyed; reservationszero;joined shutdown; this proves using settings, not exhausting quotas"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "production-server-arrangement-under-public-envelope"
        },
        {
          "id": "server-private-resource-quota-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-private-resource-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "five actual registry owners; sixth rejected before allocation; releaseone/recover;actualdestruction; public fixed handler peaks4 so no publicguestexhaustion claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-resources",
            "value": 5,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-resource-registry"
        },
        {
          "id": "server-private-resource-quota-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/public_limits.rs"
          ],
          "gateId": "case-server-private-resource-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::public_limits::private_resource_registry_uses_public_five_limit_rejects_sixth_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/public_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "five actual registry owners; sixth rejected before allocation; releaseone/recover;actualdestruction; public fixed handler peaks4 so no publicguestexhaustion claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-resources",
            "value": 6,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-resource-registry"
        },
        {
          "id": "server-private-callback-quota-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs"
          ],
          "gateId": "case-server-private-callback-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::callback_limits::private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth",
            "source": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "eight actual Host::enter policy invocations accept; ninth/further reject; unchanged Config::envelopecallbacks8; freshHostrecovery; zeroStore/worker/provider/response; no actualguestcallbacks"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-callbacks",
            "value": 8,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-Host-enter-policy"
        },
        {
          "id": "server-private-callback-quota-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs"
          ],
          "gateId": "case-server-private-callback-quota-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::callback_limits::private_callback_quota_uses_public_eight_accepts_eighth_and_rejects_ninth",
            "source": "crates/zryna-driver/src/server_runtime/tests/callback_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "eight actual Host::enter policy invocations accept; ninth/further reject; unchanged Config::envelopecallbacks8; freshHostrecovery; zeroStore/worker/provider/response; no actualguestcallbacks"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-callbacks",
            "value": 9,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-Host-enter-policy"
        },
        {
          "id": "server-public-shape-size-exclusion",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "identity",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs"
          ],
          "gateId": "case-server-public-shape-size-exclusion",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::public_size_bound::public_single_scalar_size_upper_bound_is_strictly_below_component_ceiling",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "single no-arg i32 solefunction sourceboundshape;actual16383node/128bytename canonical artifact emitted/audited; bothReply/ClockRead;symbolic bound baseline+57343+256+256 strictlybelow1048576; source/IR/ABI/encoder/topology proofneeds independentreview+execution; NO exact1048576acceptance or authenticatedworkerpublicsource claim"
            ],
            "diagnosticCodes": []
          },
          "boundary": null,
          "proofScope": "conditional-public-shape-reachability-exclusion"
        },
        {
          "id": "server-attempts-first-extra-config",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server.rs"
          ],
          "gateId": "case-server-status-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "rejected_configuration_and_unapproved_listener_never_start_or_publish",
            "source": "apps/zryna/tests/wasi_server.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "attempts65 selection rejects before readiness/bundle; joins actual64 sharedproductiontransport only through an explicit mixedscope pair"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "attempts",
            "value": 65,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "public-server-configuration-admission"
        },
        {
          "id": "server-clock-request-quota-exact",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-public",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::granted_clock_profile_executes_one_read_per_store_and_records_actual_teardown",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "approved guestrequest monotonic_reads1; actual eachStore atmost1read; observedStore/read counters; joinedteardown"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "clock-reads",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "public-server-clock-request-admission-and-execution"
        },
        {
          "id": "server-clock-request-quota-first-extra",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/clock_tests.rs"
          ],
          "gateId": "case-server-clock-denial",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "clock_tests::malformed_or_unapproved_guest_inputs_and_cross_profile_flags_reject_before_ready",
            "source": "apps/zryna/tests/wasi_server/clock_tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "guestrequest monotonic_reads2 rejected before readiness; distinct rootapproval2 rejection also existing matching_digest case; no fabricatedsecondactualread"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "clock-reads",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "public-server-clock-request-admission"
        },
        {
          "id": "server-component-first-extra-length-guard",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-backend-webassembly/src/component_server/tests.rs"
          ],
          "gateId": "case-server-component-first-extra-length-guard",
          "test": {
            "runner": "rust",
            "package": "zryna-backend-webassembly",
            "target": "lib",
            "selector": "component_server::tests::malformed_oversized_and_extra_sections_do_not_reach_world_decoder",
            "source": "crates/zryna-backend-webassembly/src/component_server/tests.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual audit receives1048577zero-filled bytes and rejects; unchanged audit length guard executes before worlddecoder; vector is not structurally valid or canonical component; assertion is rejection only, no exact1048576acceptance"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "component-bytes",
            "value": 1048577,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-server-component-audit-length-guard"
        },
        {
          "id": "server-fuel-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-server-fuel-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::fuel_limits::private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion",
            "source": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged Config::envelope fuel100000; Envelope.validate100000 accepts/100001 ErrorLimit; actual measured unchanged productionmeter99999completedcalls then lastchargedcall traps at0 and nextOutOfFuel; fresh profile recovery; no publicguest100000successfulcalls"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-fuel",
            "value": 100000,
            "unit": "fuel-units",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-Envelope-fuel-admission"
        },
        {
          "id": "server-fuel-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs"
          ],
          "gateId": "case-server-fuel-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::fuel_limits::private_server_fuel100000_admission_first_extra100001_and_measured_exhaustion",
            "source": "crates/zryna-driver/src/server_runtime/tests/fuel_limits.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "unchanged Config::envelope fuel100000; Envelope.validate100000 accepts/100001 ErrorLimit; actual measured unchanged productionmeter99999completedcalls then lastchargedcall traps at0 and nextOutOfFuel; fresh profile recovery; no publicguest100000successfulcalls"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-fuel",
            "value": 100001,
            "unit": "fuel-units",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-Envelope-fuel-admission"
        },
        {
          "id": "server-memory-byte-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-server-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::memory_policy::public_server_memory_envelope_accepts65536_and_rejects65537_before_host",
            "source": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-memory",
            "value": 65536,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
        },
        {
          "id": "server-memory-byte-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs"
          ],
          "gateId": "case-server-memory-byte-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::memory_policy::public_server_memory_envelope_accepts65536_and_rejects65537_before_host",
            "source": "crates/zryna-driver/src/server_runtime/tests/memory_policy.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy; actual unchanged owning guard; no claim outside named scope"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "guest-memory",
            "value": 65537,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-Envelope-and-ResourceLimiter-memory-policy"
        },
        {
          "id": "server-serial-request-policy-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs"
          ],
          "gateId": "case-server-serial-request-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::retirement::public_serial::production_serial_one_request_accepts_first_rejects_second_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config.requests1 unchanged; private Prepared first actual paused Store retained; second admission rejected before anotherStore; release204 teardown/recovery; not TCPbacklog or public concurrent socket rejection"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "concurrent-requests",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-Config-bound-production-lifecycle-real-Store-admission"
        },
        {
          "id": "server-serial-request-policy-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs"
          ],
          "gateId": "case-server-serial-request-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::tests::retirement::public_serial::production_serial_one_request_accepts_first_rejects_second_and_recovers",
            "source": "crates/zryna-driver/src/server_runtime/tests/retirement/public_serial.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "Config.requests1 unchanged; private Prepared first actual paused Store retained; second admission rejected before anotherStore; release204 teardown/recovery; not TCPbacklog or public concurrent socket rejection"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "concurrent-requests",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-Config-bound-production-lifecycle-real-Store-admission"
        },
        {
          "id": "server-clock-operation-policy-exact-limit",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs"
          ],
          "gateId": "case-server-clock-operation-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::clock_quota::private_clock_operation_one_read_then_second_limit_preserves_unpublished_output",
            "source": "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual operations.invoke/Host.clock under effective grant1; first actualprovider read; second Limit before outputslot/provider counter; live Store balances/destructs/fresh recovery; not second publicWasm callback"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "clock-reads",
            "value": 1,
            "unit": "count",
            "decision": "accept"
          },
          "proofScope": "private-production-host-operation-real-Store-clock-quota"
        },
        {
          "id": "server-clock-operation-policy-first-extra",
          "rowIds": [
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs"
          ],
          "gateId": "case-server-clock-operation-policy-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna-driver",
            "target": "lib",
            "selector": "server_runtime::execution::clock_quota::private_clock_operation_one_read_then_second_limit_preserves_unpublished_output",
            "source": "crates/zryna-driver/src/server_runtime/tests/clock_quota.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual operations.invoke/Host.clock under effective grant1; first actualprovider read; second Limit before outputslot/provider counter; live Store balances/destructs/fresh recovery; not second publicWasm callback"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "clock-reads",
            "value": 2,
            "unit": "count",
            "decision": "reject"
          },
          "proofScope": "private-production-host-operation-real-Store-clock-quota"
        },
        {
          "id": "server-config-capture-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-config-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "configuration-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-config-capture-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-config-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::configuration1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "configuration-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-listener-capture-exact-limit",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "exact-limit",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-listener-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "listener-approval-bytes",
            "value": 1024,
            "unit": "bytes",
            "decision": "accept"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        },
        {
          "id": "server-listener-capture-first-extra",
          "rowIds": [
            "server-status-linux",
            "server-status-windows",
            "server-clock-linux",
            "server-clock-windows"
          ],
          "kind": "first-extra",
          "fixture": [
            "apps/zryna/tests/wasi_server/configuration_bounds.rs"
          ],
          "gateId": "case-server-listener-capture-exact-limit",
          "test": {
            "runner": "rust",
            "package": "zryna",
            "target": "wasi_server",
            "selector": "configuration_bounds::listener_approval1024_and_first_extra1025_use_actual_capture_in_both_profiles",
            "source": "apps/zryna/tests/wasi_server/configuration_bounds.rs",
            "ignoredByDefault": false
          },
          "oracle": {
            "assertions": [
              "actual validJSON1024 accepts both publicprofiles/real listener serves200/teardown;1025 rejects before readiness/publication with C4201; exactapproval digest binds capturedcontent"
            ],
            "diagnosticCodes": []
          },
          "boundary": {
            "limitId": "listener-approval-bytes",
            "value": 1025,
            "unit": "bytes",
            "decision": "reject"
          },
          "proofScope": "public-CLI-private-file-capture-and-loopback-execution"
        }
      ],
      "requiredGates": [
        "m0-full",
        "m1-full",
        "m2-full",
        "m3-full",
        "case-server-source-denial",
        "case-server-cancel",
        "case-server-clock-public",
        "case-server-clock-denial",
        "case-server-clock-no-intrinsic",
        "case-server-clock-idle",
        "case-server-clock-retained-inputs-windows",
        "case-server-guest-request-bytes-exact-limit",
        "case-server-body-bytes-exact-limit",
        "case-server-header-bytes-exact-limit",
        "case-server-header-fields-exact-limit",
        "case-server-path-bytes-exact-limit",
        "case-server-attempts-exact",
        "case-server-lowered-envelopes",
        "case-server-memory-growth",
        "case-server-configuration-negative",
        "case-server-component-replay",
        "case-wit-closure",
        "case-server-request-deadline-exact-limit",
        "case-server-service-lifetime-exact-limit",
        "case-server-public-envelope-binding",
        "case-server-private-resource-quota-exact-limit",
        "case-server-private-callback-quota-exact-limit",
        "case-server-public-shape-size-exclusion",
        "case-server-status-denial",
        "case-server-component-first-extra-length-guard",
        "case-server-fuel-policy-exact-limit",
        "case-server-memory-byte-policy-exact-limit",
        "case-server-serial-request-policy-exact-limit",
        "case-server-clock-operation-policy-exact-limit",
        "case-server-config-capture-exact-limit",
        "case-server-listener-capture-exact-limit"
      ],
      "proofGaps": [],
      "authenticatedExecution": "not-evaluated",
      "exclusions": [
        "installed-component",
        "installed-browser",
        "installed-server-status",
        "installed-server-clock-status",
        "existing-immutable-beta-archives",
        "signed-public-release-authentication",
        "production-admission",
        "other-platforms",
        "other-browsers",
        "general-WASI-host-authority",
        "specified-only-library-resource-package-typed-JS-interfaces"
      ]
    }
  ],
  "exclusions": [
    "installed-component",
    "installed-browser",
    "installed-server-status",
    "installed-server-clock-status",
    "existing-immutable-beta-archives",
    "signed-public-release-authentication",
    "production-admission",
    "other-platforms",
    "other-browsers",
    "general-WASI-host-authority",
    "specified-only-library-resource-package-typed-JS-interfaces"
  ]
}
```

zryna.m4.support.v1:end
