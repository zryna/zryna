//! Mandatory fresh ordinary Rust producer; no recipe/compiler/admission capability.
use super::material::{OrdinaryTools, RustTools, capture_data, check_source, read};
use super::*;

/// Fresh ordinary fixture source/object and actual Rust process records, never a public issuer.
#[derive(Clone)]
pub(super) struct OrdinaryRustMeasurement {
    pub(super) object: Vec<u8>,
    pub(super) source: Vec<u8>,
    pub(super) records: Vec<process::Record>,
    pub(super) evidence: PathBuf,
    pub(super) rust_observations: serde_json::Value,
    pub(super) receipt: Vec<u8>,
    original: OriginalObservation,
}
#[derive(Clone)]
struct OriginalObservation {
    object_sha: [u8; 32],
    records: serde_json::Value,
    rust: RustTools,
    evidence: PathBuf,
    receipt: Vec<u8>,
}
impl OrdinaryRustMeasurement {
    pub(super) fn capture(
        &self,
        requirements: &HandleLinkRequirements,
    ) -> Result<CapturedRustProfileLibrary, Diagnostic> {
        self.check_origin()?;
        capture_data(requirements, &self.object, &self.source)
    }
    fn check_origin(&self) -> Result<(), Diagnostic> {
        self.original.rust.check()?;
        if sha(&self.object) != self.original.object_sha
            || observed_records(&self.records) != self.original.records
            || self.rust_observations != self.original.rust.report()
            || self.evidence != self.original.evidence
            || self.receipt != self.original.receipt
            || read(&self.original.evidence.join("receipt.json"), 1024 * 1024)?
                != self.original.receipt
        {
            return Err(rejected());
        }
        check_source(&self.source)
    }
    pub(super) fn check_binding(&self, request: &ImportRequest<'_>) -> Result<(), Diagnostic> {
        request.library.check_binding(request.requirements)?;
        self.check_origin()?;
        if request.library.bytes() != self.object || request.library.source_bytes() != self.source {
            return Err(rejected());
        }
        targets::compare_functions(reference::ORIGINAL_RUST_OBJECT, &self.object)
    }
}
pub(super) fn fresh(
    tools: &OrdinaryTools,
    parent: &Path,
) -> Result<OrdinaryRustMeasurement, Diagnostic> {
    let source = capture::FOREIGN_RUST_SOURCE.as_bytes().to_vec();
    check_source(&source)?;
    let source_issuer = capture::capture();
    let issued = capture::issue(&source_issuer);
    for object in [&issued.scalar_import, &issued.byte_import] {
        let requirements = handle_link_requirements(object)?;
        capture_data(&requirements, reference::ORIGINAL_RUST_OBJECT, &source)?;
    }
    let rust = RustTools::acquire()?;
    let workspace = Workspace::new(parent)?;
    let stages = workspace.stages(1)?;
    let stage = &stages[0];
    let mut session = Session::new(tools);
    session.rust_tools = Some(&rust);
    session.artifacts.push(("foreign.rs".into(), source.clone()));
    let result = (|| {
        stage.write_input(&stage.directory.join("library.rs"), &source)?;
        verify_version(&mut session, stage, &rust)?;
        let args = compile_arguments(stage, &rust);
        if read(&stage.directory.join("library.rs"), source.len())? != source {
            return Err(rejected());
        }
        process::invoke(&mut session, stage, "rust-compile", &rust.rustc, args, false)?;
        let output = session.records.last().and_then(|r| r.output.as_ref()).ok_or_else(rejected)?;
        if !output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty() {
            return Err(rejected());
        }
        if read(&stage.directory.join("library.rs"), source.len())? != source {
            return Err(rejected());
        }
        Ok(())
    })();
    let object =
        read(&stage.directory.join("library.o"), zryna_backend_native::MAX_NATIVE_OBJECT_BYTES);
    if let Ok(bytes) = &object {
        session.artifacts.push(("foreign-original.o".into(), bytes.clone()));
    }
    let result = result.and_then(|()| {
        session.phase = "fresh-rust-profile-reference";
        targets::compare_functions(
            reference::ORIGINAL_RUST_OBJECT,
            object.as_ref().map_err(Clone::clone)?,
        )
    });
    cleanup(&mut session, &stages, &[(0, "library.rs"), (0, "library.o")]);
    let evidence = complete(&mut session, result, &workspace, "measurement");
    if !session.diagnostics.is_empty() {
        return Err(rejected());
    }
    let object = object?;
    session.rust_tools = None;
    let receipt = read(&evidence.join("receipt.json"), 1024 * 1024)?;
    let rust_observations = rust.report();
    let records = std::mem::take(&mut session.records);
    drop(session);
    let original = OriginalObservation {
        object_sha: sha(&object),
        records: observed_records(&records),
        rust,
        evidence: evidence.clone(),
        receipt: receipt.clone(),
    };
    Ok(OrdinaryRustMeasurement {
        object,
        source,
        records,
        evidence,
        rust_observations,
        receipt,
        original,
    })
}

fn verify_version(
    session: &mut Session<'_>,
    stage: &native::NativeStage,
    rust: &RustTools,
) -> Result<(), Diagnostic> {
    process::invoke(session, stage, "rust-version", &rust.rustc, vec!["-vV".into()], false)?;
    let version = session.records.last().and_then(|r| r.output.as_ref()).ok_or_else(rejected)?;
    let text = std::str::from_utf8(&version.stdout).map_err(|_| rejected())?;
    if !version.status.success()
        || !version.stderr.is_empty()
        || !text.lines().next().is_some_and(|v| v.starts_with("rustc 1.97.1 ("))
        || !text.lines().any(|v| v == "host: x86_64-unknown-linux-gnu")
    {
        return Err(rejected());
    }
    Ok(())
}

fn compile_arguments(stage: &native::NativeStage, rust: &RustTools) -> Vec<OsString> {
    let source_path = stage.capability_directory_path().join("library.rs");
    let product = stage.capability_directory_path().join("library.o");
    let mut args: Vec<OsString> = [
        "--edition=2024",
        "--crate-name=fixture_rust_c_v0",
        "--crate-type=lib",
        "--target=x86_64-unknown-linux-gnu",
        "--emit=obj",
        "--sysroot",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    args.push(rust.root.clone().into_os_string());
    args.extend(
        [
            "-Cpanic=abort",
            "-Copt-level=2",
            "-Ccodegen-units=1",
            "-Cdebug-assertions=no",
            "-Coverflow-checks=no",
            "-Cforce-unwind-tables=no",
            "-Cembed-bitcode=no",
        ]
        .map(Into::into),
    );
    args.extend([source_path.into_os_string(), "-o".into(), product.into_os_string()]);
    args
}

// Exact snapshots of genuine producer outputs, including raw status/streams, not imported claims.
fn observed_records(records: &[process::Record]) -> serde_json::Value {
    use std::os::unix::process::ExitStatusExt as _;
    serde_json::Value::Array(
        records
            .iter()
            .map(|r| {
                serde_json::json!({
                    "metadata":export::record(r),
                    "raw_status":r.output.as_ref().map(|o|o.status.into_raw()),
                    "stdout":r.output.as_ref().map(|o|o.stdout.as_slice()),
                    "stderr":r.output.as_ref().map(|o|o.stderr.as_slice())
                })
            })
            .collect(),
    )
}
