//! A constructor-owned actual hostile compile, never deserialized or pristine admission.
use super::super::material::RustTools;
use super::*;
use std::os::unix::process::ExitStatusExt as _;

pub(super) struct HostileObservation {
    kind: Kind,
    source: Vec<u8>,
    object: Vec<u8>,
    records: Vec<process::Record>,
    rust: RustTools,
    evidence: PathBuf,
    receipt: Vec<u8>,
    original_object_sha: [u8; 32],
    original_records: serde_json::Value,
}
impl HostileObservation {
    pub(super) const fn kind(&self) -> Kind {
        self.kind
    }
    pub(super) fn source(&self) -> &[u8] {
        &self.source
    }
    pub(super) fn object(&self) -> &[u8] {
        &self.object
    }
    pub(super) fn check(&self) -> Result<(), Diagnostic> {
        self.rust.check()?;
        if self.source != self.kind.source()?
            || sha(&self.object) != self.original_object_sha
            || records(&self.records) != self.original_records
            || material::read(&self.evidence.join("receipt.json"), 1024 * 1024)? != self.receipt
            || material::read(
                &self.evidence.join("foreign.rs"),
                zryna_backend_native::MAX_NATIVE_OBJECT_BYTES,
            )? != self.source
            || material::read(
                &self.evidence.join("foreign-original.o"),
                zryna_backend_native::MAX_NATIVE_OBJECT_BYTES,
            )? != self.object
        {
            return Err(rejected());
        }
        Ok(())
    }
    pub(super) fn capture(
        &self,
        requirements: &HandleLinkRequirements,
    ) -> Result<CapturedRustProfileLibrary, Diagnostic> {
        self.check()?;
        let object_sha = sha(&self.object);
        let source_sha = sha(&self.source);
        capture_rust_profile_library(
            requirements,
            &RustProfileLibraryInput {
                library: ForeignLibraryInput {
                    library_id: capture::LIBRARY_ID,
                    header_bytes: capture::HEADER,
                    policy_bytes: capture::POLICY,
                    object_bytes: &self.object,
                    object_size: self.object.len(),
                    object_sha256: &object_sha,
                    dependency_symbols: &[],
                },
                source_bytes: &self.source,
                source_size: self.source.len(),
                source_sha256: &source_sha,
            },
        )
    }
    pub(super) fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "scope":"private hostile Rust-source result control, not reviewed library behavior",
            "kind":self.kind.name(),"source_sha256":digest(&self.source),
            "object_sha256":digest(&self.object),"retained_evidence":self.evidence,
            "actual_records":self.original_records,"rust_tools":self.rust.report(),
            "original_receipt_sha256":digest(&self.receipt),
            "pristine_measurement":false,"production_admission":false,
            "private_recipe_qualified":false,"complete_input_use":false
        })
    }
}

pub(super) fn fresh(
    tools: &material::OrdinaryTools,
    parent: &Path,
    kind: Kind,
) -> Result<HostileObservation, Diagnostic> {
    let source = kind.source()?;
    let rust = RustTools::acquire()?;
    let workspace = Workspace::new(parent)?;
    let stages = workspace.stages(1)?;
    let stage = &stages[0];
    let mut session = Session::new(tools);
    session.rust_tools = Some(&rust);
    session.artifacts.push(("foreign.rs".into(), source.clone()));
    session.artifacts.push((
        "hostile-source-kind.json".into(),
        serde_json::to_vec(&serde_json::json!({"kind":kind.name(),
            "source_sha256":digest(&source),"pristine_admission":false}))
        .map_err(|_| rejected())?,
    ));
    let result = compile_source(&mut session, stage, &rust, &source);
    let object = material::read(
        &stage.directory.join("library.o"),
        zryna_backend_native::MAX_NATIVE_OBJECT_BYTES,
    );
    if let Ok(bytes) = &object {
        session.artifacts.push(("foreign-original.o".into(), bytes.clone()));
    }
    let result = result.and_then(|()| {
        session.phase = "hostile-rust-profile-audit";
        let bytes = object.as_ref().map_err(Clone::clone)?;
        if bytes.is_empty() {
            return Err(rejected());
        }
        let captured = capture::capture();
        let issued = capture::issue(&captured);
        let requirements = handle_link_requirements(&issued.byte_import)?;
        let object_sha = sha(bytes);
        let source_sha = sha(&source);
        capture_rust_profile_library(
            &requirements,
            &RustProfileLibraryInput {
                library: ForeignLibraryInput {
                    library_id: capture::LIBRARY_ID,
                    header_bytes: capture::HEADER,
                    policy_bytes: capture::POLICY,
                    object_bytes: bytes,
                    object_size: bytes.len(),
                    object_sha256: &object_sha,
                    dependency_symbols: &[],
                },
                source_bytes: &source,
                source_size: source.len(),
                source_sha256: &source_sha,
            },
        )?;
        Ok(())
    });
    cleanup(&mut session, &stages, &[(0, "library.rs"), (0, "library.o")]);
    let evidence = complete(&mut session, result, &workspace, kind.name());
    if !session.diagnostics.is_empty() {
        return Err(rejected());
    }
    let object = object?;
    let receipt = material::read(&evidence.join("receipt.json"), 1024 * 1024)?;
    let actual_records = std::mem::take(&mut session.records);
    session.rust_tools = None;
    drop(session);
    let original_object_sha = sha(&object);
    let original_records = records(&actual_records);
    let result = HostileObservation {
        kind,
        source,
        object,
        records: actual_records,
        rust,
        evidence,
        receipt,
        original_object_sha,
        original_records,
    };
    result.check()?;
    Ok(result)
}
fn compile_source(
    session: &mut Session<'_>,
    stage: &native::NativeStage,
    rust: &RustTools,
    source: &[u8],
) -> Result<(), Diagnostic> {
    stage.write_input(&stage.directory.join("library.rs"), source)?;
    process::invoke(session, stage, "rust-version", &rust.rustc, vec!["-vV".into()], false)?;
    process::successful(session)?;
    let output =
        &session.records.last().and_then(|r| r.output.as_ref()).ok_or_else(rejected)?.stdout;
    let version = std::str::from_utf8(output).map_err(|_| rejected())?;
    if !version.lines().next().is_some_and(|v| v.starts_with("rustc 1.97.1 ("))
        || !version.lines().any(|v| v == "host: x86_64-unknown-linux-gnu")
    {
        return Err(rejected());
    }
    let args = arguments(stage, rust);
    if material::read(&stage.directory.join("library.rs"), source.len())? != source {
        return Err(rejected());
    }
    process::invoke(session, stage, "rust-compile", &rust.rustc, args, false)?;
    process::successful(session)?;
    if !session
        .records
        .last()
        .and_then(|r| r.output.as_ref())
        .ok_or_else(rejected)?
        .stdout
        .is_empty()
        || material::read(&stage.directory.join("library.rs"), source.len())? != source
    {
        return Err(rejected());
    }
    Ok(())
}

fn records(values: &[process::Record]) -> serde_json::Value {
    serde_json::Value::Array(values.iter().map(|r| serde_json::json!({
        "metadata":export::record(r),"raw_status":r.output.as_ref().map(|o|o.status.into_raw()),
        "stdout":r.output.as_ref().map(|o|o.stdout.as_slice()),
        "stderr":r.output.as_ref().map(|o|o.stderr.as_slice())
    })).collect())
}
fn arguments(stage: &native::NativeStage, rust: &RustTools) -> Vec<OsString> {
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
    args.extend([
        stage.capability_directory_path().join("library.rs").into_os_string(),
        "-o".into(),
        stage.capability_directory_path().join("library.o").into_os_string(),
    ]);
    args
}
