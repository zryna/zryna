//! Closed source/frame/selection correlation precedes every evaluator host operation.

use sha2::{Digest, Sha256};
use zryna_playground_compiler::read_request;

mod base64;
mod schema;
use schema::{Header, Request};
#[cfg(test)]
mod tests;

pub(super) const MAX_EVALUATION_BYTES: usize = 2_097_152;
const MAX_FRAME_BYTES: usize = 1_277_956;
const ERROR: &str = "PLAYGROUND-EVALUATION-REQUEST";

pub(super) fn hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug)]
pub(super) struct Claims {
    compiler: String,
    toolkit: String,
    policy: String,
    template: String,
    wit: String,
}
impl Claims {
    pub(super) fn authenticate(
        &self,
        approved: &zryna_driver::PlaygroundDaemonOperatorApproval,
    ) -> Result<(), &'static str> {
        if approved.version != 2
            || approved.composition_kind != "reviewed-source-v1"
            || self.compiler != approved.compiler_executable_sha256
            || self.toolkit != approved.composition_sha256
            || self.policy != approved.policy_sha256
            || self.template != approved.binding_template_sha256
            || self.wit != approved.wit_closure_sha256
        {
            return Err("PLAYGROUND-HOST-APPROVAL");
        }
        Ok(())
    }
}

pub(super) fn validate(bytes: &[u8]) -> Result<Claims, &'static str> {
    if bytes.is_empty() || bytes.len() > MAX_EVALUATION_BYTES {
        return Err(ERROR);
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| ERROR)?;
    let source = &request.input;
    // Reuse the compiler's closed inert source validator, including its revision and byte caps.
    let encoded = serde_json::to_vec(&serde_json::json!({"version":source.version,
        "revision":source.revision(), "source":source.source()}))
    .map_err(|_| ERROR)?;
    read_request(&mut encoded.as_slice()).map_err(|_| ERROR)?;
    let source_hash = digest(source.source().as_bytes());
    let selected = &request.selection;
    let receipt = &request.receipt;
    let authority = &request.authority;
    if request.version != 1
        || selected.revision != source.revision()
        || receipt.revision != source.revision()
        || selected.source_sha256 != source_hash
        || receipt.source_sha256 != source_hash
        || !hash(&selected.component_sha256)
        || !hash(&receipt.frame_sha256)
        || selected.args.len() > 256
        || selected.logical.is_empty()
        || selected.logical.len() > 1024
        || !selected.logical.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphabetic() || byte == b'_' || index > 0 && byte.is_ascii_digit()
        })
        || [&authority.compiler, &authority.toolkit, &authority.policy, &authority.binding_template]
            .iter()
            .any(|value| !hash(value))
        || receipt.compiler_sha256 != authority.compiler
        || receipt.toolkit_sha256 != authority.toolkit
        || receipt.policy_sha256 != authority.policy
    {
        return Err(ERROR);
    }
    let frame = base64::decode(&request.frame_base64, MAX_FRAME_BYTES)?;
    if digest(&frame) != receipt.frame_sha256 || frame.len() < 4 {
        return Err(ERROR);
    }
    let length = usize::try_from(u32::from_le_bytes(frame[..4].try_into().map_err(|_| ERROR)?))
        .map_err(|_| ERROR)?;
    if length == 0 || length > 98_304 || length > frame.len() - 4 {
        return Err(ERROR);
    }
    let header: Header = serde_json::from_slice(&frame[4..4 + length]).map_err(|_| ERROR)?;
    validate_header(&request, &header, &frame[4 + length..], &source_hash)?;
    Ok(Claims {
        compiler: authority.compiler.clone(),
        toolkit: authority.toolkit.clone(),
        policy: authority.policy.clone(),
        template: authority.binding_template.clone(),
        wit: header.identity.wit_sha256,
    })
}

fn validate_header(
    request: &Request,
    header: &Header,
    artifacts: &[u8],
    source_hash: &str,
) -> Result<(), &'static str> {
    let identity = &header.identity;
    if header.format != "zryna.playground-compilation.v1"
        || header.version != 1
        || header.revision != request.input.revision()
        || header.source_path != "src/main.zry"
        || header.source_sha256 != source_hash
        || header.source_bytes != request.input.source().len()
        || header.profile != "browser-component-v1"
        || header.status != "compiled"
        || !header.failure.is_null()
        || !header.report.is_object()
        || identity.revision != "zryna.browser-component-bindings.v1"
        || identity.world != "zryna:capability-profiles/browser@0.1.0"
        || identity.component_sha256 != request.selection.component_sha256
        || [
            &identity.component_sha256,
            &identity.core_sha256,
            &identity.interface_sha256,
            &identity.wit_sha256,
        ]
        .iter()
        .any(|value| !hash(value))
        || identity.core_bytes < 8
        || header.artifacts.len() != 3
        || header.exports.is_empty()
        || header.exports.len() > 512
    {
        return Err(ERROR);
    }
    let mut cursor = 0;
    for (index, expected) in ["component", "loader", "declarations"].into_iter().enumerate() {
        let entry = &header.artifacts[index];
        let cap = if index == 0 { 1_048_576 } else { 65_536 };
        if entry.role != expected
            || entry.bytes == 0
            || entry.bytes > cap
            || !hash(&entry.sha256)
            || entry.bytes > artifacts.len().saturating_sub(cursor)
        {
            return Err(ERROR);
        }
        let content = &artifacts[cursor..cursor + entry.bytes];
        if digest(content) != entry.sha256 {
            return Err(ERROR);
        }
        if index == 0 {
            if entry.sha256 != identity.component_sha256
                || identity.core_offset > content.len()
                || identity.core_bytes > content.len() - identity.core_offset
                || digest(
                    &content[identity.core_offset..identity.core_offset + identity.core_bytes],
                ) != identity.core_sha256
            {
                return Err(ERROR);
            }
        } else if std::str::from_utf8(content).is_err() {
            return Err(ERROR);
        }
        cursor += entry.bytes;
    }
    if cursor != artifacts.len() {
        return Err(ERROR);
    }
    validate_exports(request, header)
}

fn validate_exports(request: &Request, header: &Header) -> Result<(), &'static str> {
    let mut logical = std::collections::BTreeSet::new();
    let mut core = std::collections::BTreeSet::new();
    let mut selected = false;
    for entry in &header.exports {
        let valid_name = |name: &str| {
            !name.is_empty()
                && name.len() <= 1024
                && name.bytes().enumerate().all(|(index, byte)| {
                    byte.is_ascii_alphabetic() || byte == b'_' || index > 0 && byte.is_ascii_digit()
                })
        };
        let component = format!(
            "zryna-export-{}",
            entry
                .logical
                .bytes()
                .flat_map(|byte| {
                    const HEX: &[u8] = b"0123456789abcdef";
                    [
                        char::from(HEX[usize::from(byte >> 4)]),
                        char::from(HEX[usize::from(byte & 15)]),
                    ]
                })
                .collect::<String>()
        );
        if !valid_name(&entry.logical)
            || !valid_name(&entry.core)
            || entry.component != component
            || entry.arity > 256
            || !logical.insert(&entry.logical)
            || !core.insert(&entry.core)
        {
            return Err(ERROR);
        }
        if entry.logical == request.selection.logical {
            if entry.arity != request.selection.args.len() {
                return Err(ERROR);
            }
            selected = true;
        }
    }
    if !selected {
        return Err(ERROR);
    }
    Ok(())
}
