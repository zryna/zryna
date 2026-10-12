//! Independently frozen private fixture data. Matching bytes never approve a recipe or host.
use super::{CapturedForeignLibrary, HandleLinkRequirements, closure, rejected};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::{path::PathBuf, sync::Arc};
use zryna_diagnostics::Diagnostic;

mod files;
mod inventory;
mod observed;
#[cfg(test)]
mod tests;

pub(super) const MAX_MATERIALS: usize = 64;
pub(super) const MAX_TOOL_MATERIALS: usize = 8;
pub(super) const MAX_TOOL_BYTES: usize = 64 * 1024 * 1024;
pub(super) const MAX_TOTAL_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Role {
    Tool,
    Startup,
    Static,
    Script,
    Shared,
    Loader,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MaterialExpectation {
    pub(crate) id: String,
    pub(crate) role: Role,
    pub(crate) path: String,
    pub(crate) bytes: usize,
    pub(crate) sha256: [u8; 32],
    pub(crate) trace_path: Option<String>,
    pub(crate) original_path: String,
}

/// Caller-owned equality prerequisites, not approval or a #361 sysroot representation.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Expectation {
    pub(crate) format: String,
    pub(crate) target: String,
    pub(crate) root: PathBuf,
    pub(crate) materials: Vec<MaterialExpectation>,
    pub(crate) linker_order: Vec<String>,
    pub(crate) providers: closure::ProviderExpectation,
}

#[derive(Clone, Debug)]
pub(crate) struct RetainedMaterial {
    expected: MaterialExpectation,
    bytes: Arc<[u8]>,
}

/// Keeps original issuer identity and immutable acquired bytes; contains no execution seal.
#[derive(Clone, Debug)]
pub(crate) struct RetainedLinkInputs {
    requirements: HandleLinkRequirements,
    library: CapturedForeignLibrary,
    expectation: Expectation,
    materials: Vec<RetainedMaterial>,
}

pub(crate) fn capture(
    requirements: &HandleLinkRequirements,
    library: &CapturedForeignLibrary,
    expected: &Expectation,
) -> Result<RetainedLinkInputs, Diagnostic> {
    library.check_binding(requirements)?;
    inventory::validate(expected)?;
    let root = files::Root::open(&expected.root)?;
    let mut materials = Vec::new();
    for item in &expected.materials {
        let bytes = root.read(&item.path, item.bytes)?;
        if bytes.len() != item.bytes || <[u8; 32]>::from(Sha256::digest(&bytes)) != item.sha256 {
            return Err(rejected());
        }
        materials.push(RetainedMaterial { expected: item.clone(), bytes: bytes.into() });
    }
    root.revalidate()?;
    let retained = RetainedLinkInputs {
        requirements: requirements.clone(),
        library: library.clone(),
        expectation: expected.clone(),
        materials,
    };
    closure::validate_providers(&retained.providers(), retained.provider_expectation())?;
    retained.check_originals()?;
    Ok(retained)
}

/// Bounded exact raw-byte expectation pin supplied independently of the observer.
pub(crate) fn read_expectation(
    path: &std::path::Path,
    expected_sha256: &[u8; 32],
) -> Result<Expectation, Diagnostic> {
    let parent = path.parent().ok_or_else(rejected)?;
    let name = path.file_name().and_then(|value| value.to_str()).ok_or_else(rejected)?;
    let root = files::Root::open(parent)?;
    let bytes = root.read(name, crate::native::MAX_NATIVE_TOOL_OUTPUT_BYTES)?;
    if <[u8; 32]>::from(Sha256::digest(&bytes)) != *expected_sha256 {
        return Err(rejected());
    }
    let expected: Expectation = serde_json::from_slice(&bytes).map_err(|_| rejected())?;
    inventory::validate(&expected)?;
    Ok(expected)
}

impl RetainedLinkInputs {
    pub(crate) fn check_tools(
        &self,
        tools: &super::LinuxX8664LinkToolchain,
    ) -> Result<(), Diagnostic> {
        let selected = self
            .materials
            .iter()
            .filter(|item| item.expected.role == Role::Tool)
            .collect::<Vec<_>>();
        for path in [&tools.driver, &tools.linker] {
            if !selected.iter().any(|item| item.expected.trace_path.as_deref() == path.to_str()) {
                return Err(rejected());
            }
        }
        self.check_originals()
    }

    fn check_originals(&self) -> Result<(), Diagnostic> {
        let host = files::Root::open(std::path::Path::new("/"))?;
        for item in &self.materials {
            let relative = item.expected.original_path.strip_prefix('/').ok_or_else(rejected)?;
            host.compare(relative, &item.bytes)?;
        }
        Ok(())
    }

    pub(crate) fn check_binding(
        &self,
        requirements: &HandleLinkRequirements,
        library: &CapturedForeignLibrary,
    ) -> Result<(), Diagnostic> {
        self.library.check_binding(requirements)?;
        library.check_binding(&self.requirements)?;
        if self.library.library_id() != library.library_id()
            || self.library.bytes() != library.bytes()
            || !self.library.definitions().eq(library.definitions())
            || !self.library.dependencies().eq(library.dependencies())
        {
            return Err(rejected());
        }
        Ok(())
    }

    pub(crate) fn providers(&self) -> Vec<closure::RetainedProvider> {
        self.expectation
            .providers
            .providers
            .iter()
            .map(|rule| {
                let material = self
                    .materials
                    .iter()
                    .find(|item| item.expected.id == rule.id)
                    .expect("validated closed provider material reference");
                closure::RetainedProvider { id: rule.id.clone(), bytes: material.bytes.clone() }
            })
            .collect()
    }

    pub(crate) fn provider_expectation(&self) -> &closure::ProviderExpectation {
        &self.expectation.providers
    }

    /// Stage aliases are original producer-bound object paths, never caller process arguments.
    pub(crate) fn check_order(
        &self,
        observed: &[String],
        objects: &[(&str, String)],
    ) -> Result<(), Diagnostic> {
        inventory::check_order(&self.expectation, observed, objects)
    }

    pub(crate) fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "scope":"independently-pinned-selected-private-fixture-materials",
            "target":self.expectation.target,
            "linker_order":self.expectation.linker_order,
            "materials":self.materials.iter().map(|item| serde_json::json!({
                "id":item.expected.id,"role":format!("{:?}",item.expected.role),
                "bytes":item.bytes.len(),"sha256":format!("{:x}",Sha256::digest(&item.bytes))
            })).collect::<Vec<_>>(),
            "production_approval":false,"complete_tool_dependency_closure":false,
            "selected_original_paths":self.materials.iter().map(|item|&item.expected.original_path).collect::<Vec<_>>(),
            "transient_input_replacement_excluded":false,
            "sysroot_digest_interpretation":null,"complete_input_use_attestation":false,
            "execution_authorized":false,"cache_authorized":false,"publication_authorized":false
        })
    }

    #[cfg(test)]
    pub(crate) fn capture_disaffiliated_control(
        &self,
        requirements: &HandleLinkRequirements,
        library: &CapturedForeignLibrary,
        tools: &super::LinuxX8664LinkToolchain,
    ) -> Result<Self, Diagnostic> {
        let mut expected = self.expectation.clone();
        for item in &mut expected.materials {
            if item.role == Role::Tool
                && [&tools.driver, &tools.linker]
                    .iter()
                    .any(|path| path.to_str() == Some(item.original_path.as_str()))
            {
                item.role = Role::Script;
            }
        }
        capture(requirements, library, &expected)
    }

    pub(crate) fn exported_materials(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.materials.iter().map(|item| (item.expected.id.as_str(), item.bytes.as_ref()))
    }
}

/// Absence leaves legacy observations unqualified; partial configuration always rejects.
pub(crate) fn capture_from_environment(
    requirements: &HandleLinkRequirements,
    library: &CapturedForeignLibrary,
) -> Result<Option<RetainedLinkInputs>, Diagnostic> {
    let path = std::env::var_os("ZRYNA_LINK_INVENTORY_EXPECTATION_FILE");
    let pin = std::env::var_os("ZRYNA_LINK_INVENTORY_EXPECTATION_SHA256");
    match (path, pin) {
        (None, None) => Ok(None),
        (Some(path), Some(pin)) => {
            let text = pin.to_str().ok_or_else(rejected)?;
            if text.len() != 64
                || !text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(rejected());
            }
            let mut digest = [0; 32];
            for (index, value) in digest.iter_mut().enumerate() {
                *value = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
                    .map_err(|_| rejected())?;
            }
            let expected = read_expectation(std::path::Path::new(&path), &digest)?;
            capture(requirements, library, &expected).map(Some)
        }
        _ => Err(rejected()),
    }
}
