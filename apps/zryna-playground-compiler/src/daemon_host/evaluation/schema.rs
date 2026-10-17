//! Closed inert evaluation and compiler frame carriers.

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Request {
    pub(super) version: u32,
    #[serde(rename = "sourceRequest")]
    pub(super) input: Source,
    pub(super) frame_base64: String,
    pub(super) selection: Selection,
    pub(super) receipt: Receipt,
    pub(super) authority: Authority,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub(super) version: u32,
    pub(super) revision: u64,
    #[serde(rename = "source")]
    pub(super) text: String,
}
impl Source {
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }
    pub(super) fn source(&self) -> &str {
        &self.text
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Selection {
    pub(super) revision: u64,
    pub(super) source_sha256: String,
    pub(super) component_sha256: String,
    pub(super) logical: String,
    pub(super) args: Vec<i32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Receipt {
    pub(super) revision: u64,
    pub(super) source_sha256: String,
    pub(super) frame_sha256: String,
    pub(super) compiler_sha256: String,
    pub(super) toolkit_sha256: String,
    pub(super) policy_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Authority {
    #[serde(rename = "compilerSha256")]
    pub(super) compiler: String,
    #[serde(rename = "toolkitSha256")]
    pub(super) toolkit: String,
    #[serde(rename = "policySha256")]
    pub(super) policy: String,
    #[serde(rename = "bindingTemplateSha256")]
    pub(super) binding_template: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Header {
    pub(super) format: String,
    pub(super) version: u32,
    pub(super) revision: u64,
    pub(super) source_path: String,
    pub(super) source_sha256: String,
    pub(super) source_bytes: usize,
    pub(super) profile: String,
    pub(super) status: String,
    pub(super) report: serde_json::Value,
    pub(super) failure: serde_json::Value,
    pub(super) identity: Identity,
    pub(super) exports: Vec<Export>,
    pub(super) artifacts: Vec<Artifact>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Identity {
    pub(super) revision: String,
    pub(super) world: String,
    pub(super) wit_sha256: String,
    pub(super) component_sha256: String,
    pub(super) core_sha256: String,
    pub(super) interface_sha256: String,
    pub(super) core_offset: usize,
    pub(super) core_bytes: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Export {
    pub(super) logical: String,
    pub(super) component: String,
    pub(super) core: String,
    pub(super) arity: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Artifact {
    pub(super) role: String,
    pub(super) bytes: usize,
    pub(super) sha256: String,
}
