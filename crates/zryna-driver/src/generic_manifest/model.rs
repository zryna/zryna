//! Plain strict metadata; decoding any of these records never creates source authority.
use serde::{Deserialize, Serialize};
use zryna_abi::{ScalarOutcome, ScalarType, ScalarValue};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GenericTarget {
    JavaScript,
    WebAssembly,
    Native,
}
impl GenericTarget {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::JavaScript => "javascript",
            Self::WebAssembly => "webassembly",
            Self::Native => "native",
        }
    }
    pub(crate) const fn material(self) -> (&'static str, &'static str) {
        match self {
            Self::JavaScript => ("ecmascript-module", "mjs"),
            Self::WebAssembly => ("core-webassembly-module", "wasm"),
            Self::Native => ("linux-x86-64-relocatable-object", "o"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GenericCommand {
    Build,
    Run,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericModuleRecord {
    pub(crate) id: u32,
    pub(crate) path: String,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericEdgeRecord {
    pub(crate) importer: String,
    pub(crate) target: String,
    pub(crate) specifier: String,
    pub(crate) imported: String,
    pub(crate) local: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericSourceRecord {
    pub(crate) entrypoint: String,
    pub(crate) entry_module: u32,
    pub(crate) modules: Vec<GenericModuleRecord>,
    pub(crate) edges: Vec<GenericEdgeRecord>,
    pub(crate) graph_sha256: String,
    pub(crate) execution_sha256: String,
    pub(crate) type_universe_sha256: String,
    pub(crate) linear32_sha256: String,
    pub(crate) linux_x86_64_sha256: String,
    pub(crate) runtime_abi_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericExportRecord {
    pub(crate) name: String,
    pub(crate) parameters: Vec<ScalarType>,
    pub(crate) result: ScalarType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericArtifactRecord {
    pub(crate) target: GenericTarget,
    pub(crate) kind: String,
    pub(crate) path: String,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericNativeRecord {
    pub(crate) target: String,
    pub(crate) template_sha256: String,
    pub(crate) header_sha256: String,
    pub(crate) rendered_sha256: String,
    pub(crate) ordinary_object_sha256: String,
    pub(crate) required_tls_bytes: u64,
    pub(crate) execution: Option<GenericNativeExecutionRecord>,
}

/// Byte provenance from an actually prepared native invocation; decoding grants no execution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericNativeExecutionRecord {
    #[serde(rename = "ordinaryObjectSha256")]
    pub(crate) ordinary_object: String,
    #[serde(rename = "runtimeObjectSha256")]
    pub(crate) runtime_object: String,
    #[serde(rename = "invocationObjectSha256")]
    pub(crate) invocation_object: String,
    #[serde(rename = "harnessSha256")]
    pub(crate) harness: String,
    #[serde(rename = "executableSha256")]
    pub(crate) executable: String,
    #[serde(rename = "dependencyReceiptSha256")]
    pub(crate) dependency_receipt: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericObservedOutcome {
    pub(crate) target: GenericTarget,
    pub(crate) outcome: ScalarOutcome,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericInvocationRecord {
    pub(crate) export: String,
    pub(crate) arguments: Vec<ScalarValue>,
    pub(crate) outcomes: Vec<GenericObservedOutcome>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct GenericManifestWire {
    pub(crate) format: String,
    pub(crate) profile: String,
    pub(crate) syntax_protocol: u32,
    pub(crate) command: GenericCommand,
    pub(crate) artifact_stem: String,
    pub(crate) source: GenericSourceRecord,
    pub(crate) exports: Vec<GenericExportRecord>,
    pub(crate) artifacts: Vec<GenericArtifactRecord>,
    pub(crate) native: Option<GenericNativeRecord>,
    pub(crate) invocation: Option<GenericInvocationRecord>,
}
