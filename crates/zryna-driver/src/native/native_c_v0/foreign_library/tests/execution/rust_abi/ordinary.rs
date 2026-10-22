//! Ordinary trusted Rust-backed fixture observations, never production or recipe admission.
use super::capture;
use crate::native::{
    self,
    native_c_v0::{
        foreign_library::{
            CapturedRustProfileLibrary, ForeignLibraryInput, RustProfileLibraryInput,
            capture_rust_profile_library,
        },
        resource_identity::{HandleLinkRequirements, handle_link_requirements},
    },
};
use sha2::{Digest as _, Sha256};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use zryna_diagnostics::Diagnostic;
mod clients;
mod controls;
mod export;
mod material;
mod process;
mod result_faults;
use super::object_fixture as reference;
mod rust_measurement;
mod sanitizers;
mod stage;
mod targets;
mod tests;
use stage::{Workspace, cleanup};
fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rejected() -> Diagnostic {
    native::native_error(
        "ZRYNA-C4102",
        "ordinary Rust fixture input or comparison failed",
        "retain original issuers, actual separately measured materials and bounded process evidence",
    )
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Scalar,
    Bytes { missing_increment: bool },
    Reverse,
}
struct ImportRequest<'a> {
    requirements: &'a HandleLinkRequirements,
    library: &'a CapturedRustProfileLibrary,
    measurement: &'a material::OrdinaryRustMeasurement,
}
impl ImportRequest<'_> {
    fn check(&self) -> Result<(), Diagnostic> {
        self.measurement.check_binding(self)
    }
}
struct Session<'a> {
    tools: &'a material::OrdinaryTools,
    rust_tools: Option<&'a material::RustTools>,
    measurement: Option<&'a material::OrdinaryRustMeasurement>,
    records: Vec<process::Record>,
    artifacts: Vec<(String, Vec<u8>)>,
    diagnostics: Vec<Diagnostic>,
    phase: &'static str,
    targets_checked: bool,
}
impl<'a> Session<'a> {
    fn new(tools: &'a material::OrdinaryTools) -> Self {
        Self {
            tools,
            rust_tools: None,
            measurement: None,
            records: Vec::new(),
            artifacts: Vec::new(),
            diagnostics: Vec::new(),
            phase: "input-binding",
            targets_checked: false,
        }
    }
}
fn complete(
    session: &mut Session<'_>,
    result: Result<(), Diagnostic>,
    workspace: &Workspace,
    name: &str,
) -> PathBuf {
    if let Err(error) = result {
        session.diagnostics.push(error);
    }
    let path = export::finish(session, &workspace.path, name)
        .expect("create-only retained ordinary observation including failures");
    eprintln!("m5-ordinary-rust-evidence {}", path.display());
    path
}
fn observe_import<'a>(
    request: &ImportRequest<'a>,
    tools: &'a material::OrdinaryTools,
    parent: &Path,
    mode: Mode,
) -> Result<(PathBuf, Session<'a>), Diagnostic> {
    request.check()?;
    tools.check()?;
    if mode == Mode::Reverse {
        return Err(rejected());
    }
    let client = clients::import(request, mode)?;
    let workspace = Workspace::new(parent)?;
    let stages = workspace.stages(3)?;
    let mut session = Session::new(tools);
    session.measurement = Some(request.measurement);
    capture_import_artifacts(&mut session, request, &client);
    let result = (|| {
        stages[0].write_input(&stages[0].object, request.requirements.object().bytes())?;
        stages[0]
            .write_input(&stages[0].directory.join("runtime.o"), &request.measurement.object)?;
        process::compile(&mut session, &stages[1], "compile-client", client.as_bytes())?;
        if matches!(mode, Mode::Bytes { .. }) {
            let source = request.requirements.private_runtime_source().ok_or_else(rejected)?;
            session.artifacts.push(("private-runtime.c".into(), source.to_vec()));
            session.artifacts.push((
                "private-runtime-header.h".into(),
                request
                    .requirements
                    .object()
                    .program()
                    .source()
                    .runtime_abi()
                    .native_linux_x86_64_header()
                    .to_vec(),
            ));
            process::compile(&mut session, &stages[2], "compile-runtime", source)?;
        } else if request.requirements.private_runtime_source().is_some() {
            return Err(rejected());
        }
        process::link(&mut session, &stages, mode)?;
        targets::verify_rust_targets(
            &mut session,
            &stages[0],
            if mode == Mode::Scalar { "add" } else { "sum_bytes" },
        )?;
        request.check()?;
        process::run(&mut session, &stages[0], mode)
    })();
    for (index, name) in [(1, "client-partial.o"), (2, "runtime-partial.o")] {
        if !session
            .artifacts
            .iter()
            .any(|(n, _)| n == if index == 1 { "compile-client.o" } else { "compile-runtime.o" })
            && let Ok(bytes) = material::read(
                &stages[index].directory.join("runtime.o"),
                zryna_backend_native::MAX_NATIVE_OBJECT_BYTES,
            )
        {
            session.artifacts.push((name.into(), bytes));
        }
    }
    if !session.artifacts.iter().any(|(n, _)| n == "final.elf")
        && let Ok(bytes) = material::read(
            &stages[0].directory.join("invocation.elf"),
            native::MAX_NATIVE_EXECUTABLE_BYTES,
        )
    {
        session.artifacts.push(("final.elf".into(), bytes));
    }
    cleanup(&mut session, &stages, &[]);
    let path = complete(
        &mut session,
        result,
        &workspace,
        if mode == Mode::Scalar { "scalar" } else { "bytes" },
    );
    Ok((path, session))
}
fn observe_reverse<'a>(
    object: &zryna_backend_native::native_c_v0::ValidatedScalarExports,
    tools: &'a material::OrdinaryTools,
    parent: &Path,
) -> Result<(PathBuf, Session<'a>), Diagnostic> {
    tools.check()?;
    let workspace = Workspace::new(parent)?;
    let stages = workspace.stages(2)?;
    let mut session = Session::new(tools);
    session.artifacts.extend([
        ("program.o".into(), object.bytes().to_vec()),
        ("reverse-exports.h".into(), object.header().as_bytes().to_vec()),
        ("reverse-client.c".into(), clients::reverse().as_bytes().to_vec()),
    ]);
    let result = (|| {
        stages[0].write_input(&stages[0].object, object.bytes())?;
        stages[1].write_input(
            &stages[1].directory.join("reverse-exports.h"),
            object.header().as_bytes(),
        )?;
        process::compile(
            &mut session,
            &stages[1],
            "compile-client",
            clients::reverse().as_bytes(),
        )?;
        process::link(&mut session, &stages, Mode::Reverse)?;
        process::run(&mut session, &stages[0], Mode::Reverse)
    })();
    cleanup(&mut session, &stages, &[(1, "reverse-exports.h")]);
    let path = complete(&mut session, result, &workspace, "reverse");
    Ok((path, session))
}

fn capture_import_artifacts(session: &mut Session<'_>, request: &ImportRequest<'_>, client: &str) {
    session.artifacts.extend([
        ("program.o".into(), request.requirements.object().bytes().to_vec()),
        ("program-header.h".into(), request.requirements.object().header().as_bytes().to_vec()),
        ("foreign.o".into(), request.measurement.object.clone()),
        ("foreign.rs".into(), request.measurement.source.clone()),
        ("client.c".into(), client.as_bytes().to_vec()),
        ("library-header.h".into(), capture::HEADER.to_vec()),
        ("library-policy.json".into(), capture::POLICY.to_vec()),
    ]);
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
pub(super) fn m5_owned_sanitizer_runtime(parent: &Path) -> Result<serde_json::Value, Diagnostic> {
    sanitizers::m5_owned_sanitizer_runtime(parent)
}
