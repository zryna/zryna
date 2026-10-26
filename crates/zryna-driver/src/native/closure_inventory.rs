//! Closed retained-provider comparison. This is data, never loader or execution permission.
use crate::native;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zryna_diagnostics::Diagnostic;

#[path = "native_c_v0/foreign_library/linked_output/closure/dynamic.rs"]
mod dynamic;
#[path = "native_c_v0/foreign_library/linked_output/closure/image.rs"]
mod image;
#[path = "native_c_v0/foreign_library/linked_output/closure/providers.rs"]
mod providers;
#[path = "native_c_v0/foreign_library/linked_output/closure/resolution.rs"]
mod resolution;
#[path = "native_c_v0/foreign_library/linked_output/closure/symbols.rs"]
mod symbols;
#[cfg(test)]
#[path = "native_c_v0/foreign_library/linked_output/closure/tests.rs"]
mod tests;
#[path = "native_c_v0/foreign_library/linked_output/closure/versions.rs"]
mod versions;

#[derive(Clone, Debug)]
pub(crate) struct RetainedProvider {
    pub(crate) id: String,
    pub(crate) bytes: std::sync::Arc<[u8]>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProviderRule {
    pub(crate) id: String,
    pub(crate) sha256: [u8; 32],
    pub(crate) soname: Option<String>,
    pub(crate) needed: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProviderExpectation {
    pub(crate) interpreter: String,
    pub(crate) needed: Vec<String>,
    pub(crate) providers: Vec<ProviderRule>,
    pub(crate) allow_weak: bool,
    pub(crate) allow_ifunc: bool,
    pub(crate) allow_tls: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClosureInventory {
    pub(crate) executable: ObjectInventory,
    pub(crate) providers: Vec<(String, ObjectInventory)>,
    execution_authorized: bool,
    complete_input_use_attestation: bool,
    relocation_or_resolver_execution_attested: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ObjectInventory {
    pub(crate) sha256: [u8; 32],
    pub(crate) interpreter: Option<String>,
    pub(crate) soname: Option<String>,
    pub(crate) needed: Vec<String>,
    pub(crate) dynamic_entries: Vec<(u64, u64)>,
    pub(crate) symbols: Vec<symbols::Symbol>,
    pub(crate) versions: versions::Versions,
}
impl ObjectInventory {
    fn name_bytes(&self) -> usize {
        self.versions.name_bytes() + self.symbols.iter().map(|s| s.name.len()).sum::<usize>()
    }
}

fn inspect(
    bytes: &[u8],
    kind: u16,
    policy: &ProviderExpectation,
) -> Result<ObjectInventory, Diagnostic> {
    let elf = image::Image::read(bytes, kind)?;
    let dynamic = dynamic::read(&elf)?;
    let versions = versions::read(&elf, &dynamic)?;
    if (!policy.allow_tls && dynamic.tags.get(&30).is_some_and(|v| v & 16 != 0))
        || (!policy.allow_weak
            && versions
                .definitions
                .values()
                .chain(versions.needs.values())
                .any(|v| v.flags & 2 != 0))
    {
        return Err(rejected());
    }
    let symbols = symbols::read(&elf, &dynamic, &versions, policy)?;
    if versions.name_bytes() + symbols.iter().map(|s| s.name.len()).sum::<usize>()
        > image::MAX_STRINGS
    {
        return Err(rejected());
    }
    Ok(ObjectInventory {
        sha256: Sha256::digest(bytes).into(),
        interpreter: elf.interpreter,
        soname: dynamic.soname,
        needed: dynamic.needed,
        dynamic_entries: dynamic.entries,
        symbols,
        versions,
    })
}

pub(crate) fn audit(
    final_elf: &[u8],
    providers: &[RetainedProvider],
    expected: &ProviderExpectation,
) -> Result<ClosureInventory, Diagnostic> {
    providers::audit(final_elf, providers, expected)
}

/// Validate retained target-provider prerequisites before any newly affected fixture process.
pub(crate) fn validate_providers(
    providers: &[RetainedProvider],
    expected: &ProviderExpectation,
) -> Result<(), Diagnostic> {
    providers::retain(providers, expected, 0, 0, 0).map(|_| ())
}

fn rejected() -> Diagnostic {
    native::native_error(
        "ZRYNA-C4102",
        "private linked-output observations lack exact retained fixture input binding",
        "retain the original issuers and actual bytes; observations never approve dependencies",
    )
}
