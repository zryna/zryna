//! Private sealed scalar interface binding for future JavaScript adapter consumers.

mod consumer;

use sha2::{Digest, Sha256};
use zryna_abi::{ScalarType, VerifiedScalarAbiModule, raw as raw_abi, verify_v1};
use zryna_backend_javascript::JavaScriptArtifact;
use zryna_diagnostics::Diagnostic;

use crate::VerifiedModuleClosure;

/// Exact private interface revision implemented by this boundary.
pub(crate) const SCALAR_ADAPTER_INTERFACE_REVISION: &str = "zryna.scalar-adapter-interface.v1";
/// Exact verified language profile admitted by this boundary.
pub(crate) const SCALAR_ADAPTER_PROFILE: &str = "zryna-control-flow-v1";
/// The only backend artifact kind admitted by this boundary.
pub(crate) const SCALAR_ADAPTER_TARGET: &str = "javascript-esm";

const INTERFACE_IDENTITY_DOMAIN: &[u8] = b"ZRYNA-SCALAR-ADAPTER-INTERFACE\0";
const BINDING_IDENTITY_DOMAIN: &[u8] = b"ZRYNA-SCALAR-ADAPTER-BINDING\0";

/// Caller-controlled claims that cannot construct verified interface authority.
pub(crate) mod raw {
    use zryna_abi::raw::Module;

    /// Claimed browser or Node deployment policy.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct HostPolicy {
        /// Claimed deployment row.
        pub(crate) boundary: String,
        /// Claimed host interfaces; scalar v1 admits none.
        pub(crate) required_interfaces: Vec<String>,
    }

    /// Complete untrusted scalar-interface declaration.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct Interface {
        /// Claimed interface revision.
        pub(crate) revision: String,
        /// Claimed language profile.
        pub(crate) profile: String,
        /// Claimed backend artifact kind.
        pub(crate) target: String,
        /// Claimed deployment policy.
        pub(crate) host: HostPolicy,
        /// Claimed authenticated source-graph identity.
        pub(crate) source_graph_sha256: [u8; 32],
        /// Claimed exact backend artifact identity.
        pub(crate) artifact_sha256: [u8; 32],
        /// Claimed exact backend artifact byte length.
        pub(crate) artifact_bytes: u64,
        /// Claimed ordered exports and scalar signatures.
        pub(crate) exports: Module,
    }
}

/// Closed browser and Node ESM deployment rows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScalarAdapterHost {
    /// Browser ESM with no implicit DOM, fetch, storage, or host interface.
    Browser,
    /// Node ESM with no implicit process, filesystem, network, or host interface.
    Node,
}

impl ScalarAdapterHost {
    const fn identity(self) -> &'static str {
        match self {
            Self::Browser => "js-browser",
            Self::Node => "js-node",
        }
    }
}

/// Source-graph-bound `ControlFlowV1` authority prepared for interface verification.
///
/// Its fields are private; only [`lower_verified_scalar_source`] can bind a verified program to
/// the exact authenticated graph that produced it.
#[derive(Debug)]
pub(crate) struct VerifiedScalarSource {
    graph_sha256: [u8; 32],
    program: zryna_ir::control_flow_v1::VerifiedProgram,
}

impl VerifiedScalarSource {
    /// Returns the exact source-bound verified program for existing backend consumers.
    pub(crate) const fn program(&self) -> &zryna_ir::control_flow_v1::VerifiedProgram {
        &self.program
    }

    /// Emits and seals the exact pure Node ESM interface used by the current internal consumer.
    pub(crate) fn prepare_node_interface(&self) -> Result<VerifiedScalarEsm, Diagnostic> {
        let artifact = zryna_backend_javascript::emit_control_flow(&self.program)?;
        let claim = derive_scalar_adapter_claim(self, &artifact, ScalarAdapterHost::Node);
        verify_scalar_adapter_interface(self, artifact, claim)
            .map_err(|errors| errors.into_iter().next().expect("rejection contains a diagnostic"))
    }
}

/// Lowers one authenticated pure source closure exactly once for later artifact binding.
///
/// # Errors
///
/// Returns the existing semantic or mandatory IR-verifier diagnostics.
pub(crate) fn lower_verified_scalar_source(
    closure: &VerifiedModuleClosure,
) -> Result<VerifiedScalarSource, Vec<Diagnostic>> {
    let program = closure.lower_control_flow_v1()?;
    Ok(VerifiedScalarSource { graph_sha256: *closure.graph_sha256(), program })
}

/// Opaque source-, interface-, host-, profile-, and ESM-artifact-bound scalar authority.
///
/// Raw declarations and caller digests are not retained as authority. The exact verified program
/// and byte-compared backend artifact stay sealed inside this value for a later internal consumer.
#[derive(Debug)]
pub(crate) struct VerifiedScalarEsm {
    graph_sha256: [u8; 32],
    abi: VerifiedScalarAbiModule,
    artifact: std::sync::Arc<JavaScriptArtifact>,
    host: ScalarAdapterHost,
    artifact_sha256: [u8; 32],
    interface_sha256: [u8; 32],
    binding_sha256: [u8; 32],
}

impl VerifiedScalarEsm {
    /// Returns the sealed scalar ABI used by the exact retained artifact.
    pub(crate) const fn scalar_abi(&self) -> &VerifiedScalarAbiModule {
        &self.abi
    }

    /// Returns the exact verified ESM source for a later internal consumer.
    pub(crate) fn javascript_source(&self) -> Result<&str, Diagnostic> {
        self.revalidate()?;
        Ok(&self.artifact.source)
    }

    /// Rechecks the retained interface, source, and artifact identities before consumption.
    fn revalidate(&self) -> Result<(), Diagnostic> {
        let artifact_sha256: [u8; 32] = Sha256::digest(self.artifact.source.as_bytes()).into();
        let artifact_bytes = u64::try_from(self.artifact.source.len())
            .expect("bounded JavaScript backend artifacts fit the interface length carrier");
        let interface_sha256 = interface_identity(self.host, self.scalar_abi());
        let binding_sha256 = binding_identity(
            &interface_sha256,
            &self.graph_sha256,
            &artifact_sha256,
            artifact_bytes,
        );
        if artifact_sha256 != self.artifact_sha256
            || interface_sha256 != self.interface_sha256
            || binding_sha256 != self.binding_sha256
        {
            return Err(Diagnostic::error(
                "ZRYNA-D3810",
                None,
                "sealed scalar adapter interface identity changed before consumption",
                "discard the interface and repeat verification from the authenticated source",
            ));
        }
        Ok(())
    }
}

/// Derives a provisional claim from sealed compiler authorities for internal composition.
///
/// The returned declaration is still reverified in full and is never itself authority.
pub(crate) fn derive_scalar_adapter_claim(
    source: &VerifiedScalarSource,
    artifact: &JavaScriptArtifact,
    host: ScalarAdapterHost,
) -> raw::Interface {
    let artifact_sha256 = Sha256::digest(artifact.source.as_bytes()).into();
    let artifact_bytes = u64::try_from(artifact.source.len())
        .expect("bounded JavaScript backend artifacts fit the interface length carrier");
    let exports = source
        .program
        .scalar_abi()
        .exports()
        .map(|export| {
            raw_abi::Export::new(
                export.logical_name().as_str().to_owned(),
                raw_abi::Signature::new(
                    export.parameters().iter().copied().map(raw_type).collect(),
                    raw_type(export.result()),
                ),
            )
        })
        .collect();
    raw::Interface {
        revision: SCALAR_ADAPTER_INTERFACE_REVISION.to_owned(),
        profile: SCALAR_ADAPTER_PROFILE.to_owned(),
        target: SCALAR_ADAPTER_TARGET.to_owned(),
        host: raw::HostPolicy {
            boundary: host.identity().to_owned(),
            required_interfaces: Vec::new(),
        },
        source_graph_sha256: source.graph_sha256,
        artifact_sha256,
        artifact_bytes,
        exports: raw_abi::Module::new(exports),
    }
}

/// Verifies hostile scalar-interface claims and binds an exact ESM artifact.
///
/// The artifact is independently regenerated from the already source-bound verified program and
/// compared byte-for-byte. A digest match alone never authorizes the candidate artifact.
///
/// # Errors
///
/// Returns stable diagnostics for malformed, unsupported, forged, or stale claims and for any
/// backend invariant failure. No verified interface is returned after partial validation.
pub(crate) fn verify_scalar_adapter_interface(
    source: &VerifiedScalarSource,
    artifact: JavaScriptArtifact,
    claim: raw::Interface,
) -> Result<VerifiedScalarEsm, Vec<Diagnostic>> {
    if claim.revision != SCALAR_ADAPTER_INTERFACE_REVISION {
        return Err(single_failure(
            "ZRYNA-D3811",
            "scalar adapter interface revision is unsupported or stale",
            "use the exact compiler-owned scalar interface revision",
        ));
    }
    if claim.profile != SCALAR_ADAPTER_PROFILE {
        return Err(single_failure(
            "ZRYNA-D3812",
            "scalar adapter language profile claim is unsupported",
            "use an interface derived from the verified ControlFlowV1 authority",
        ));
    }
    if claim.target != SCALAR_ADAPTER_TARGET {
        return Err(single_failure(
            "ZRYNA-D3813",
            "scalar adapter backend artifact claim is unsupported",
            "use the exact JavaScript ESM target; WebAssembly and native adapters are not admitted",
        ));
    }
    let host = verify_host_policy(&claim.host)?;
    if claim.source_graph_sha256 != source.graph_sha256 {
        return Err(single_failure(
            "ZRYNA-D3816",
            "scalar adapter source graph identity is stale or forged",
            "derive the interface from the exact authenticated source closure",
        ));
    }

    let claimed_abi = verify_v1(claim.exports).map_err(abi_failures)?;
    if claimed_abi != *source.program.scalar_abi() {
        return Err(single_failure(
            "ZRYNA-D3817",
            "scalar adapter exports or types differ from verified source authority",
            "preserve exact export order, names, arity, parameter types, and result types",
        ));
    }

    let expected = zryna_backend_javascript::emit_control_flow(&source.program)
        .map_err(|diagnostic| vec![diagnostic])?;
    if artifact != expected {
        return Err(single_failure(
            "ZRYNA-D3818",
            "scalar adapter ESM artifact is not the output of the bound verified program",
            "bind the exact backend artifact emitted from the authenticated verified source",
        ));
    }
    let artifact_sha256: [u8; 32] = Sha256::digest(artifact.source.as_bytes()).into();
    let artifact_bytes = u64::try_from(artifact.source.len())
        .expect("bounded JavaScript backend artifacts fit the interface length carrier");
    if claim.artifact_sha256 != artifact_sha256 || claim.artifact_bytes != artifact_bytes {
        return Err(single_failure(
            "ZRYNA-D3819",
            "scalar adapter artifact identity or byte length is stale or forged",
            "use the identity of the exact byte-compared JavaScript ESM artifact",
        ));
    }

    let interface_sha256 = interface_identity(host, source.program.scalar_abi());
    let binding_sha256 =
        binding_identity(&interface_sha256, &source.graph_sha256, &artifact_sha256, artifact_bytes);
    Ok(VerifiedScalarEsm {
        graph_sha256: source.graph_sha256,
        abi: claimed_abi,
        artifact: std::sync::Arc::new(artifact),
        host,
        artifact_sha256,
        interface_sha256,
        binding_sha256,
    })
}

fn verify_host_policy(policy: &raw::HostPolicy) -> Result<ScalarAdapterHost, Vec<Diagnostic>> {
    let host = match policy.boundary.as_str() {
        "js-browser" => ScalarAdapterHost::Browser,
        "js-node" => ScalarAdapterHost::Node,
        _ => {
            return Err(single_failure(
                "ZRYNA-D3814",
                "scalar adapter host policy claim is unsupported",
                "use the exact js-browser or js-node pure ESM policy",
            ));
        }
    };
    if !policy.required_interfaces.is_empty() {
        return Err(single_failure(
            "ZRYNA-D3815",
            "scalar adapter host policy requests unsupported interfaces",
            "the private scalar interface is pure and requires no host interfaces",
        ));
    }
    Ok(host)
}

const fn raw_type(value: ScalarType) -> raw_abi::Type {
    match value {
        ScalarType::Bool => raw_abi::Type::Bool,
        ScalarType::I32 => raw_abi::Type::I32,
    }
}

fn abi_failures(violations: Vec<zryna_abi::AbiViolation>) -> Vec<Diagnostic> {
    violations
        .into_iter()
        .map(|violation| {
            Diagnostic::error(
                violation.code(),
                None,
                "scalar adapter declaration violates scalar ABI v1",
                "use only the exact bounded compiler-verified scalar export inventory",
            )
        })
        .collect()
}

fn interface_identity(host: ScalarAdapterHost, abi: &VerifiedScalarAbiModule) -> [u8; 32] {
    let mut identity = Sha256::new();
    identity.update(INTERFACE_IDENTITY_DOMAIN);
    hash_field(&mut identity, SCALAR_ADAPTER_INTERFACE_REVISION.as_bytes());
    hash_field(&mut identity, SCALAR_ADAPTER_PROFILE.as_bytes());
    hash_field(&mut identity, SCALAR_ADAPTER_TARGET.as_bytes());
    hash_field(&mut identity, host.identity().as_bytes());
    identity.update((abi.version() as u16).to_le_bytes());
    hash_count(&mut identity, abi.exports().len());
    for export in abi.exports() {
        hash_field(&mut identity, export.logical_name().as_str().as_bytes());
        hash_count(&mut identity, export.parameters().len());
        for parameter in export.parameters() {
            identity.update([scalar_code(*parameter)]);
        }
        identity.update([scalar_code(export.result())]);
    }
    identity.finalize().into()
}

fn binding_identity(
    interface: &[u8; 32],
    source: &[u8; 32],
    artifact: &[u8; 32],
    artifact_bytes: u64,
) -> [u8; 32] {
    let mut identity = Sha256::new();
    identity.update(BINDING_IDENTITY_DOMAIN);
    identity.update(interface);
    identity.update(source);
    identity.update(artifact);
    identity.update(artifact_bytes.to_le_bytes());
    identity.finalize().into()
}

fn hash_field(identity: &mut Sha256, bytes: &[u8]) {
    hash_count(identity, bytes.len());
    identity.update(bytes);
}

fn hash_count(identity: &mut Sha256, value: usize) {
    let value = u64::try_from(value).expect("verified interface counts fit canonical u64 fields");
    identity.update(value.to_le_bytes());
}

const fn scalar_code(value: ScalarType) -> u8 {
    match value {
        ScalarType::Bool => 1,
        ScalarType::I32 => 2,
    }
}

fn single_failure(
    code: &'static str,
    message: &'static str,
    guidance: &'static str,
) -> Vec<Diagnostic> {
    vec![Diagnostic::error(code, None, message, guidance)]
}

#[cfg(test)]
mod tests;
