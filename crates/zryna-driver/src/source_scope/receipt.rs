//! Owned source/provenance metadata, issued only after exact NEW execution verification.
use super::records::{GenericSourceEdge, GenericSourceModule};
use crate::{CommandFailure, generic_closure::VerifiedGenericClosure};
use sha2::{Digest, Sha256};
use zryna_abi::ScalarType;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;
use zryna_source::NormalizedSourcePath;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenericScalarExport {
    name: String,
    parameters: Vec<ScalarType>,
    result: ScalarType,
}
impl GenericScalarExport {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn parameters(&self) -> &[ScalarType] {
        &self.parameters
    }
    pub(crate) const fn result(&self) -> ScalarType {
        self.result
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenericSourceReceipt {
    profile: &'static str,
    syntax_protocol: u32,
    source_map: zryna_source::SourceMapIdentity,
    runtime_identity: zryna_ownership_runtime_abi::generic_v1::Identity,
    entrypoint: NormalizedSourcePath,
    entry_module: u32,
    modules: Vec<GenericSourceModule>,
    edges: Vec<GenericSourceEdge>,
    graph_sha256: [u8; 32],
    execution_sha256: [u8; 32],
    type_universe_sha256: [u8; 32],
    linear32_sha256: [u8; 32],
    linux_x86_64_sha256: [u8; 32],
    runtime_abi_sha256: [u8; 32],
    exports: Vec<GenericScalarExport>,
}
impl GenericSourceReceipt {
    pub(super) fn capture(
        closure: &VerifiedGenericClosure,
        program: &VerifiedExecution<'_>,
        wire: &[u8],
    ) -> Result<Self, CommandFailure> {
        let (modules, edges) = super::records::capture(closure)?;
        let mut exports = super::failure::reserve(program.scalar_abi().exports().len())?;
        for export in program.scalar_abi().exports() {
            let mut parameters = super::failure::reserve(export.parameters().len())?;
            parameters.extend_from_slice(export.parameters());
            exports.push(GenericScalarExport {
                name: export.logical_name().as_str().into(),
                parameters,
                result: export.result(),
            });
        }
        let linear = program.layouts(StorageTarget::Linear32V1);
        let linux = program.layouts(StorageTarget::LinuxX8664V1);
        Ok(Self {
            profile: "bounded-generics-v1",
            syntax_protocol: 5,
            source_map: closure.sources().identity(),
            runtime_identity: program.runtime().identity(),
            entrypoint: closure.entrypoint().clone(),
            entry_module: program.entry_file().index(),
            modules,
            edges,
            graph_sha256: *closure.graph_sha256(),
            execution_sha256: Sha256::digest(wire).into(),
            type_universe_sha256: *linear.universe_identity(),
            linear32_sha256: *linear.fingerprint(),
            linux_x86_64_sha256: *linux.fingerprint(),
            runtime_abi_sha256: program.runtime().identity().fingerprint(),
            exports,
        })
    }
    pub(crate) fn is_bound_to(&self, program: &VerifiedExecution<'_>) -> bool {
        let linear = program.layouts(StorageTarget::Linear32V1);
        let linux = program.layouts(StorageTarget::LinuxX8664V1);
        self.entry_module == program.entry_file().index()
            && self.source_map == linear.source_map_identity()
            && self.source_map == linux.source_map_identity()
            && self.runtime_identity == program.runtime().identity()
            && self.type_universe_sha256 == *linear.universe_identity()
            && self.type_universe_sha256 == *linux.universe_identity()
            && self.linear32_sha256 == *linear.fingerprint()
            && self.linux_x86_64_sha256 == *linux.fingerprint()
            && self.runtime_abi_sha256 == program.runtime().identity().fingerprint()
    }
    pub(crate) const fn profile(&self) -> &'static str {
        self.profile
    }
    pub(crate) const fn syntax_protocol(&self) -> u32 {
        self.syntax_protocol
    }
    pub(crate) fn entrypoint(&self) -> &NormalizedSourcePath {
        &self.entrypoint
    }
    pub(crate) const fn entry_module(&self) -> u32 {
        self.entry_module
    }
    pub(crate) fn modules(&self) -> &[GenericSourceModule] {
        &self.modules
    }
    pub(crate) fn edges(&self) -> &[GenericSourceEdge] {
        &self.edges
    }
    pub(crate) const fn graph_sha256(&self) -> &[u8; 32] {
        &self.graph_sha256
    }
    pub(crate) const fn execution_sha256(&self) -> &[u8; 32] {
        &self.execution_sha256
    }
    pub(crate) const fn type_universe_sha256(&self) -> &[u8; 32] {
        &self.type_universe_sha256
    }
    pub(crate) const fn linear32_sha256(&self) -> &[u8; 32] {
        &self.linear32_sha256
    }
    pub(crate) const fn linux_x86_64_sha256(&self) -> &[u8; 32] {
        &self.linux_x86_64_sha256
    }
    pub(crate) const fn runtime_abi_sha256(&self) -> &[u8; 32] {
        &self.runtime_abi_sha256
    }
    pub(crate) fn exports(&self) -> &[GenericScalarExport] {
        &self.exports
    }
}
