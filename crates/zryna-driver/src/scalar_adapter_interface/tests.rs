use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use zryna_abi::{RawHostScalar, ScalarBoundaryError, ScalarTarget, ScalarType, raw as raw_abi};
use zryna_frontend::{ProviderExpectationV3, WorkerFrontendV3, WorkerLimitsV3, WorkerSpecV3};
use zryna_source::NormalizedSourcePath;

use super::*;
use crate::{WorkspaceSourceRoot, discover_module_closure};

mod host_consumers;

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TemporaryWorkspace {
    path: PathBuf,
}

impl TemporaryWorkspace {
    fn with_source(label: &str, source: &str) -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir()
            .join(format!("zryna-scalar-interface-{}-{label}-{sequence}", std::process::id()));
        fs::create_dir(&path).expect("unique scalar-interface workspace must be created");
        fs::write(path.join("main.zry"), source).expect("fixture source must be written");
        Self { path }
    }
}

impl Drop for TemporaryWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct Fixture {
    workspace: TemporaryWorkspace,
    source: VerifiedScalarSource,
    artifact: JavaScriptArtifact,
}

impl Fixture {
    fn new(label: &str, text: &str) -> Self {
        let workspace = TemporaryWorkspace::with_source(label, text);
        let source_root =
            WorkspaceSourceRoot::capture(&workspace.path).expect("fixture source root must verify");
        let entrypoint =
            NormalizedSourcePath::new("main.zry").expect("fixture entry path must normalize");
        let closure = discover_module_closure(&source_root, entrypoint, &frontend())
            .expect("fixture module closure must verify");
        let source = lower_verified_scalar_source(&closure)
            .expect("fixture ControlFlowV1 source must verify");
        let artifact = zryna_backend_javascript::emit_control_flow(source.program())
            .expect("fixture ESM must emit");
        Self { workspace, source, artifact }
    }

    fn claim(&self, host: ScalarAdapterHost) -> raw::Interface {
        derive_scalar_adapter_claim(&self.source, &self.artifact, host)
    }

    fn verify(&self, claim: raw::Interface) -> Result<VerifiedScalarEsm, Vec<Diagnostic>> {
        verify_scalar_adapter_interface(&self.source, self.artifact.clone(), claim)
    }
}

fn frontend() -> WorkerFrontendV3 {
    let repository = repository_root();
    let adapter = crate::runtime::node_compatible_path(&repository.join("adapters/typescript-6"));
    let expected =
        ProviderExpectationV3::new("typescript-6", "6.0.3").expect("pinned provider identity");
    let spec = WorkerSpecV3::new(
        node_executable(),
        vec![adapter.join("src/worker-v3.mjs").into_os_string()],
        adapter,
        expected,
        WorkerLimitsV3::default(),
    )
    .expect("absolute pinned frontend command");
    WorkerFrontendV3::new(spec)
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repository root")
}

fn node_executable() -> PathBuf {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    for variable in ["ZRYNA_TEST_NODE", "NODE"] {
        if let Some(configured) = env::var_os(variable) {
            let path = PathBuf::from(configured);
            if path.is_file() {
                return path.canonicalize().expect("configured Node.js");
            }
        }
    }
    env::split_paths(&env::var_os("PATH").expect("PATH contains pinned Node.js"))
        .map(|directory| directory.join(executable))
        .find(|path| path.is_file())
        .expect("pinned Node.js")
        .canonicalize()
        .expect("canonical Node.js")
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(Diagnostic::code).collect()
}

fn assert_rejected(fixture: &Fixture, claim: raw::Interface, code: &str) {
    let diagnostics = fixture.verify(claim).expect_err("hostile claim must fail closed");
    assert_eq!(codes(&diagnostics), vec![code]);
}

fn export(name: &str, parameters: Vec<raw_abi::Type>, result: raw_abi::Type) -> raw_abi::Export {
    raw_abi::Export::new(name.to_owned(), raw_abi::Signature::new(parameters, result))
}

const SCALAR_SOURCE: &str = concat!(
    "export function add(left: i32, right: i32): i32 { return left + right; }\n",
    "export function identity(value: bool): bool { return value; }\n",
);

#[test]
fn seals_node_and_browser_scalar_views_with_strict_existing_carriers() {
    let fixture = Fixture::new("positive", SCALAR_SOURCE);
    let node = fixture.verify(fixture.claim(ScalarAdapterHost::Node)).expect("Node interface");
    let browser =
        fixture.verify(fixture.claim(ScalarAdapterHost::Browser)).expect("browser interface");

    assert_eq!(node.host, ScalarAdapterHost::Node);
    assert_eq!(browser.host, ScalarAdapterHost::Browser);
    assert_eq!(node.graph_sha256, fixture.source.graph_sha256);
    assert_eq!(node.artifact_sha256, browser.artifact_sha256);
    assert_ne!(node.interface_sha256, browser.interface_sha256);
    assert_ne!(node.binding_sha256, browser.binding_sha256);
    assert_eq!(node.scalar_abi().version(), zryna_abi::SCALAR_ABI_V1);
    let exports = node.scalar_abi().exports().collect::<Vec<_>>();
    assert_eq!(exports.len(), 2);
    assert_eq!(exports[0].logical_name().as_str(), "add");
    assert_eq!(exports[0].parameters(), &[ScalarType::I32, ScalarType::I32]);
    assert_eq!(exports[1].parameters(), &[ScalarType::Bool]);
    assert_eq!(exports[1].result(), ScalarType::Bool);
    let esm = node.javascript_source().expect("sealed ESM identity must revalidate");
    assert!(esm.contains("ZRYNA-B2001"));
    assert!(esm.contains("ZRYNA-B2002"));

    assert!(matches!(
        zryna_abi::decode_argument(
            ScalarTarget::JavaScript,
            ScalarType::I32,
            RawHostScalar::JavaScriptNumber(-0.0),
        ),
        Err(ScalarBoundaryError::InvalidJavaScriptI32(value)) if value.to_bits() == (-0.0_f64).to_bits()
    ));
    assert_eq!(
        zryna_abi::decode_argument(
            ScalarTarget::JavaScript,
            ScalarType::Bool,
            RawHostScalar::JavaScriptNumber(1.0),
        ),
        Err(ScalarBoundaryError::TargetCarrierMismatch)
    );
}

#[test]
fn independently_rejects_forged_revision_profile_target_host_source_and_policy() {
    let fixture = Fixture::new("closed-claims", SCALAR_SOURCE);

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.revision.push_str("-stale");
    assert_rejected(&fixture, claim, "ZRYNA-D3811");

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.profile = "zryna-i32-v1".to_owned();
    assert_rejected(&fixture, claim, "ZRYNA-D3812");

    for target in ["core-webassembly", "native-linux-x86-64", "component"] {
        let mut claim = fixture.claim(ScalarAdapterHost::Node);
        claim.target = target.to_owned();
        assert_rejected(&fixture, claim, "ZRYNA-D3813");
    }

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.host.boundary = "wasi-command".to_owned();
    assert_rejected(&fixture, claim, "ZRYNA-D3814");

    let mut claim = fixture.claim(ScalarAdapterHost::Browser);
    claim.host.required_interfaces.push("network".to_owned());
    assert_rejected(&fixture, claim, "ZRYNA-D3815");

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.source_graph_sha256[0] ^= 1;
    assert_rejected(&fixture, claim, "ZRYNA-D3816");
}

#[test]
fn rejects_hostile_exports_types_and_same_signature_artifact_substitution_then_recovers() {
    let fixture = Fixture::new("hostile-abi", SCALAR_SOURCE);

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.exports = raw_abi::Module::new(vec![
        export("add", vec![raw_abi::Type::I32, raw_abi::Type::I32], raw_abi::Type::I32),
        export("add", vec![raw_abi::Type::Bool], raw_abi::Type::Bool),
    ]);
    assert_rejected(&fixture, claim, "ZRYNA-B1002");

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.exports =
        raw_abi::Module::new(vec![export("add", vec![raw_abi::Type::Unit], raw_abi::Type::I32)]);
    assert_rejected(&fixture, claim, "ZRYNA-B1004");

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.exports = raw_abi::Module::new(vec![
        export("add", vec![raw_abi::Type::Bool, raw_abi::Type::I32], raw_abi::Type::I32),
        export("identity", vec![raw_abi::Type::Bool], raw_abi::Type::Bool),
    ]);
    assert_rejected(&fixture, claim, "ZRYNA-D3817");

    let substituted = Fixture::new(
        "artifact-substitution",
        concat!(
            "export function add(left: i32, right: i32): i32 { return left - right; }\n",
            "export function identity(value: bool): bool { return value; }\n",
        ),
    );
    let claim = derive_scalar_adapter_claim(
        &fixture.source,
        &substituted.artifact,
        ScalarAdapterHost::Node,
    );
    let diagnostics =
        verify_scalar_adapter_interface(&fixture.source, substituted.artifact.clone(), claim)
            .expect_err("same-signature different-byte artifact must fail");
    assert_eq!(codes(&diagnostics), vec!["ZRYNA-D3818"]);

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.artifact_sha256[0] ^= 1;
    assert_rejected(&fixture, claim, "ZRYNA-D3819");
    fixture.verify(fixture.claim(ScalarAdapterHost::Node)).expect("recovery must be deterministic");
}

#[test]
fn accepts_256_parameters_and_rejects_the_first_extra_declaration() {
    let parameters = (0..zryna_abi::MAX_ABI_PARAMETERS_PER_EXPORT)
        .map(|index| format!("p{index}: i32"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!("export function exact({parameters}): i32 {{ return p0; }}\n");
    let fixture = Fixture::new("parameter-limit", &source);
    let interface = fixture
        .verify(fixture.claim(ScalarAdapterHost::Node))
        .expect("exact parameter limit must verify");
    assert_eq!(
        interface.scalar_abi().exports().next().expect("exact export").parameters().len(),
        zryna_abi::MAX_ABI_PARAMETERS_PER_EXPORT
    );

    let mut claim = fixture.claim(ScalarAdapterHost::Node);
    claim.exports = raw_abi::Module::new(vec![export(
        "exact",
        vec![raw_abi::Type::I32; zryna_abi::MAX_ABI_PARAMETERS_PER_EXPORT + 1],
        raw_abi::Type::I32,
    )]);
    assert_rejected(&fixture, claim, "ZRYNA-B1201");
}
