//! Audited `DataOwnershipV1` program, runtime, harness, and executable capabilities.

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
pub(crate) mod allocation_fixture_process;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod audit;
#[cfg(test)]
mod tests;

pub(crate) mod observation;

use std::{fmt::Write as _, sync::Arc};

use sha2::{Digest, Sha256};
use zryna_abi::{Invocation, ScalarAbiVersion, ScalarType, ScalarValue};
use zryna_backend_native::data_ownership_v1::ValidatedDataOwnershipObjectArtifact;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{FunctionIdentity, ProgramIdentity};
use zryna_source::SourceMapIdentity;

use super::{
    ArtifactOutputRoot, LinuxX8664LinkToolchain, MAX_NATIVE_HARNESS_BYTES,
    NATIVE_OBJECT_ARTIFACT_EXTENSION, NativeProcessLimits, PreparedNativeExecutable,
    PublishedNativeExecutableArtifact, PublishedNativeObjectArtifact, ensure_linux_x86_64_host,
    invocation_error, native_error, publish_complete_artifact, publish_prepared_native_invocation,
    select_object_target,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::{
    NativeStage, ProcessPhase, audit_staged_executable, revalidate_tool, run_bounded_process,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::ffi::OsString;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use audit::{
    audit_completed_executable, audit_runtime_object, ownership_link_error, read_stable_file,
};

const OWNERSHIP_OBJECT_LABEL: &str = "DataOwnershipV1 native object";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const RUNTIME_OBJECT_LIMIT: usize = 2 * 1_024 * 1_024;

/// Identity sealed to one prepared `DataOwnershipV1` executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataOwnershipExecutableIdentity {
    source_map: SourceMapIdentity,
    program: ProgramIdentity,
    entry: FunctionIdentity,
    target: &'static str,
    abi: ScalarAbiVersion,
    logical_export: Box<str>,
    arguments: Box<[ScalarValue]>,
    program_object_sha256: [u8; 32],
    runtime_object_sha256: [u8; 32],
    harness_sha256: [u8; 32],
    executable_sha256: [u8; 32],
}

impl DataOwnershipExecutableIdentity {
    /// Returns the authenticated source-map authority.
    #[must_use]
    pub const fn source_map(&self) -> SourceMapIdentity {
        self.source_map
    }
    /// Returns the sealed program authority.
    #[must_use]
    pub const fn program(&self) -> ProgramIdentity {
        self.program
    }
    /// Returns the exact exported function invoked by the harness.
    #[must_use]
    pub const fn entry(&self) -> FunctionIdentity {
        self.entry
    }
    /// Returns the exact native target spelling.
    #[must_use]
    pub const fn target(&self) -> &'static str {
        self.target
    }
    /// Returns the verified public ABI version.
    #[must_use]
    pub const fn abi(&self) -> ScalarAbiVersion {
        self.abi
    }
    /// Returns the exact logical export selected for this invocation.
    #[must_use]
    pub fn logical_export(&self) -> &str {
        &self.logical_export
    }
    /// Returns the authenticated typed arguments in declaration order.
    #[must_use]
    pub fn arguments(&self) -> &[ScalarValue] {
        &self.arguments
    }
    /// Returns the audited program-object digest.
    #[must_use]
    pub const fn program_object_sha256(&self) -> &[u8; 32] {
        &self.program_object_sha256
    }
    /// Returns the audited runtime-object digest.
    #[must_use]
    pub const fn runtime_object_sha256(&self) -> &[u8; 32] {
        &self.runtime_object_sha256
    }
    /// Returns the sealed invocation-harness digest.
    #[must_use]
    pub const fn harness_sha256(&self) -> &[u8; 32] {
        &self.harness_sha256
    }
    /// Returns the completed executable digest.
    #[must_use]
    pub const fn executable_sha256(&self) -> &[u8; 32] {
        &self.executable_sha256
    }
}

/// Fully linked but unpublished `DataOwnershipV1` executable and its audited inputs.
#[derive(Clone, Debug)]
pub struct PreparedDataOwnershipExecutable {
    object: ValidatedDataOwnershipObjectArtifact,
    executable: PreparedNativeExecutable,
    identity: DataOwnershipExecutableIdentity,
}

impl PreparedDataOwnershipExecutable {
    /// Returns the sealed identity covering every executable input and output.
    #[must_use]
    pub fn identity(&self) -> &DataOwnershipExecutableIdentity {
        &self.identity
    }
    /// Returns the independently audited program object bytes.
    #[must_use]
    pub fn object_bytes(&self) -> &[u8] {
        self.object.bytes()
    }
    /// Returns the completed audited executable bytes.
    #[must_use]
    pub fn executable_bytes(&self) -> &[u8] {
        self.executable.bytes()
    }
    /// Returns the verified scalar result type.
    #[must_use]
    pub const fn result_type(&self) -> ScalarType {
        self.executable.result_type()
    }
    pub(crate) const fn prepared_executable(&self) -> &PreparedNativeExecutable {
        &self.executable
    }
}

/// Lowers, emits, links, and audits one authenticated `DataOwnershipV1` invocation.
///
/// This prepares bytes only. Public object and executable names remain absent until callers use
/// the create-only publication functions.
///
/// # Errors
/// Returns closed diagnostics for unsupported hosts or targets, invalid invocations, toolchain
/// drift, compiler/linker failures, failed audits, or unconfirmed staging cleanup.
#[allow(clippy::too_many_arguments)]
pub fn prepare_data_ownership_executable(
    program: &zryna_semantics::data_ownership_v1::VerifiedProgram,
    invocation: Invocation,
    output_root: &ArtifactOutputRoot,
    requested_target: &str,
    toolchain: &LinuxX8664LinkToolchain,
    limits: NativeProcessLimits,
) -> Result<PreparedDataOwnershipExecutable, Vec<Diagnostic>> {
    ensure_linux_x86_64_host().map_err(|error| vec![error])?;
    let target = select_object_target(requested_target).map_err(|error| vec![error])?;
    output_root.revalidate().map_err(|error| vec![error])?;
    let ir = program.verified_ir();
    let invocation = ir
        .scalar_abi()
        .prepare_invocation(invocation)
        .map_err(|error| vec![invocation_error(error)])?;
    let export = invocation.export();
    let entry = ir
        .modules()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
        .find(|function| {
            function.public_export().is_some_and(|item| item.index() == export.index())
        })
        .ok_or_else(|| vec![ownership_harness_error()])?;
    let mir = zryna_native_mir::data_ownership_v1::lower(ir, program.runtime_abi())?;
    let symbol = format!("zryna_m3_m{}_f{}", entry.id().module(), entry.id().declaration());
    let mir_entry = mir
        .functions()
        .find(|function| function.identity() == (entry.id().module(), entry.id().declaration()));
    if mir_entry.is_none_or(|function| function.symbol() != symbol) {
        return Err(vec![ownership_harness_error()]);
    }
    let object = zryna_backend_native::data_ownership_v1::emit_object(&mir, target)
        .map_err(|error| vec![error])?;
    let runtime = crate::ownership_runtime_v1::render_source(&mir);
    let harness = render_harness(&symbol, &invocation)?;
    let (runtime_object, executable_bytes, diagnostics) = link_and_audit(
        object.bytes(),
        &runtime,
        &harness,
        &symbol,
        &mir,
        output_root,
        toolchain,
        limits,
    )?;
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let identity = DataOwnershipExecutableIdentity {
        source_map: ir.source_map_identity(),
        program: ir.identity(),
        entry: entry.id(),
        target: zryna_backend_native::NATIVE_OBJECT_TARGET,
        abi: ir.scalar_abi().version(),
        logical_export: Box::from(export.logical_name().as_str()),
        arguments: invocation.arguments().into(),
        program_object_sha256: digest(object.bytes()),
        runtime_object_sha256: digest(&runtime_object),
        harness_sha256: digest(&harness),
        executable_sha256: digest(&executable_bytes),
    };
    Ok(PreparedDataOwnershipExecutable {
        object,
        executable: PreparedNativeExecutable {
            bytes: Arc::from(executable_bytes),
            result_type: export.result(),
            expected_symbol: symbol.into_boxed_str(),
            ownership_identity: Some(identity.clone()),
            diagnostics: Vec::new(),
        },
        identity,
    })
}

/// Publishes the audited `DataOwnershipV1` program object at one absent destination.
///
/// # Errors
/// Returns a stable create-only publication diagnostic without replacing an existing artifact.
pub fn publish_data_ownership_object(
    prepared: &PreparedDataOwnershipExecutable,
    output_root: &ArtifactOutputRoot,
    artifact_stem: &str,
) -> Result<PublishedNativeObjectArtifact, Diagnostic> {
    let published = publish_complete_artifact(
        prepared.object.bytes(),
        output_root,
        artifact_stem,
        NATIVE_OBJECT_ARTIFACT_EXTENSION,
        OWNERSHIP_OBJECT_LABEL,
    )?;
    Ok(PublishedNativeObjectArtifact { path: published.path, diagnostics: published.diagnostics })
}

/// Publishes the completed audited `DataOwnershipV1` executable at one absent destination.
///
/// # Errors
/// Returns stable audit or create-only publication diagnostics without replacing a destination.
pub fn publish_data_ownership_executable(
    prepared: &PreparedDataOwnershipExecutable,
    output_root: &ArtifactOutputRoot,
    artifact_stem: &str,
) -> Result<PublishedNativeExecutableArtifact, Vec<Diagnostic>> {
    publish_prepared_native_invocation(&prepared.executable, output_root, artifact_stem)
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn render_harness(
    symbol: &str,
    invocation: &zryna_abi::VerifiedInvocation<'_>,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let export = invocation.export();
    let c_type = |ty| match ty {
        ScalarType::Bool => "uint8_t",
        ScalarType::I32 => "int32_t",
    };
    let mut source = String::from("#include <stdint.h>\n#include <stdio.h>\n");
    source.push_str(include_str!("ownership/observation.c"));
    write!(source, "extern {} {symbol}(", c_type(export.result()))
        .map_err(|_| vec![ownership_harness_error()])?;
    if export.parameters().is_empty() {
        source.push_str("void");
    } else {
        for (index, ty) in export.parameters().iter().copied().enumerate() {
            if index != 0 {
                source.push_str(", ");
            }
            source.push_str(c_type(ty));
        }
    }
    source.push_str(");\nint main(int argc, char **argv) {\n  if (!configure_fault(argc, argv)) return 72;\n  int32_t result = (int32_t)");
    source.push_str(symbol);
    source.push('(');
    for (index, argument) in invocation.arguments().iter().copied().enumerate() {
        if index != 0 {
            source.push_str(", ");
        }
        match argument {
            ScalarValue::Bool(value) => {
                source.push_str(if value { "UINT8_C(1)" } else { "UINT8_C(0)" });
            }
            ScalarValue::I32(i32::MIN) => source.push_str("INT32_MIN"),
            ScalarValue::I32(value) => {
                write!(source, "INT32_C({value})").map_err(|_| vec![ownership_harness_error()])?;
            }
        }
    }
    source.push_str(
        ");\n  if (zryna_m3_finish_invocation() != 0) return 72;\n  if (!emit_word(trap_status) || !emit_word((uint32_t)result) || !emit_word(trace_count)) return 70;\n  for (uint32_t i = 0; i < trace_count && i < 4096; ++i) if (!emit_word(trace_words[i])) return 70;\n  if (fflush(stdout) != 0) return 71;\n  return 0;\n}\n",
    );
    if source.len() > MAX_NATIVE_HARNESS_BYTES {
        return Err(vec![ownership_harness_error()]);
    }
    Ok(source.into_bytes())
}

fn ownership_harness_error() -> Diagnostic {
    native_error(
        "ZRYNA-N3401",
        "DataOwnershipV1 invocation could not be bound to its verified export",
        "use one exact verified public scalar export and typed invocation",
    )
}

type LinkedOwnershipArtifacts = (Box<[u8]>, Box<[u8]>, Vec<Diagnostic>);

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[allow(clippy::too_many_arguments)]
fn link_and_audit(
    object: &[u8],
    runtime: &[u8],
    harness: &[u8],
    symbol: &str,
    mir: &zryna_native_mir::data_ownership_v1::VerifiedMirModule,
    output_root: &ArtifactOutputRoot,
    toolchain: &LinuxX8664LinkToolchain,
    limits: NativeProcessLimits,
) -> Result<LinkedOwnershipArtifacts, Vec<Diagnostic>> {
    let stage = NativeStage::create(output_root, "ownership").map_err(|error| vec![error])?;
    let operation = (|| {
        stage.write_input(&stage.object, object)?;
        stage.write_input(&stage.runtime_source, runtime)?;
        stage.write_input(&stage.harness, harness)?;
        revalidate_tool(&toolchain.driver, &toolchain.driver_identity)?;
        revalidate_tool(&toolchain.linker, &toolchain.linker_identity)?;
        let directory = stage.capability_directory_path();
        let runtime_source = stage.capability_file_path("runtime.c")?;
        let runtime_object = stage.capability_file_path("runtime.o")?;
        let compile_arguments = vec![
            OsString::from("-std=c11"),
            OsString::from("-pedantic"),
            OsString::from("-Wall"),
            OsString::from("-Wextra"),
            OsString::from("-Werror"),
            OsString::from("-O2"),
            OsString::from("-fno-common"),
            OsString::from("-fno-stack-protector"),
            OsString::from("-fno-ident"),
            OsString::from("-fno-asynchronous-unwind-tables"),
            OsString::from("-fno-unwind-tables"),
            OsString::from("-fcf-protection=none"),
            OsString::from("-c"),
            runtime_source.as_os_str().to_owned(),
            OsString::from("-o"),
            runtime_object.as_os_str().to_owned(),
        ];
        let compiled = run_bounded_process(
            &toolchain.driver,
            &compile_arguments,
            &directory,
            limits.link_timeout(),
            limits.tool_output_bytes(),
            limits.tool_output_bytes(),
            ProcessPhase::Link,
            Some(&directory),
        )?;
        if !compiled.status.success() || !compiled.stdout.is_empty() || !compiled.stderr.is_empty()
        {
            return Err(ownership_link_error());
        }
        stage.revalidate()?;
        let runtime_bytes = read_stable_file(&runtime_object, RUNTIME_OBJECT_LIMIT)?;
        audit_runtime_object(&runtime_bytes, mir)?;

        revalidate_tool(&toolchain.driver, &toolchain.driver_identity)?;
        revalidate_tool(&toolchain.linker, &toolchain.linker_identity)?;
        let executable = stage.capability_file_path("invocation.elf")?;
        let harness = stage.capability_file_path("invocation.c")?;
        let program_object = stage.capability_file_path("program.o")?;
        let link_arguments = vec![
            OsString::from("-std=c11"),
            OsString::from("-O0"),
            OsString::from("-g0"),
            OsString::from("-fno-ident"),
            OsString::from("-fno-pie"),
            OsString::from("-no-pie"),
            OsString::from("-fno-stack-protector"),
            OsString::from("-fcf-protection=none"),
            OsString::from("-Wl,--build-id=none"),
            OsString::from("-Wl,--fatal-warnings"),
            OsString::from("-Wl,--no-undefined"),
            OsString::from("-Wl,-z,noexecstack,-z,relro,-z,now"),
            OsString::from("-o"),
            executable.as_os_str().to_owned(),
            harness.as_os_str().to_owned(),
            program_object.as_os_str().to_owned(),
            runtime_object.as_os_str().to_owned(),
        ];
        let linked = run_bounded_process(
            &toolchain.driver,
            &link_arguments,
            &directory,
            limits.link_timeout(),
            limits.tool_output_bytes(),
            limits.tool_output_bytes(),
            ProcessPhase::Link,
            Some(&directory),
        )?;
        if !linked.status.success() || !linked.stdout.is_empty() || !linked.stderr.is_empty() {
            return Err(ownership_link_error());
        }
        stage.revalidate()?;
        let (_, executable_bytes) = audit_staged_executable(&executable, symbol)?;
        audit_completed_executable(&executable_bytes, symbol, mir)?;
        Ok((runtime_bytes, executable_bytes))
    })();
    let cleanup = stage.cleanup();
    match operation {
        Ok((runtime, executable)) => Ok((runtime, executable, cleanup)),
        Err(error) => {
            let mut diagnostics = vec![error];
            diagnostics.extend(cleanup);
            Err(diagnostics)
        }
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
#[allow(clippy::too_many_arguments)]
fn link_and_audit(
    _object: &[u8],
    _runtime: &[u8],
    _harness: &[u8],
    _symbol: &str,
    _mir: &zryna_native_mir::data_ownership_v1::VerifiedMirModule,
    _output_root: &ArtifactOutputRoot,
    _toolchain: &LinuxX8664LinkToolchain,
    _limits: NativeProcessLimits,
) -> Result<LinkedOwnershipArtifacts, Vec<Diagnostic>> {
    Err(vec![native_error(
        "ZRYNA-N4002",
        "native linking and invocation require a Linux x86-64 host",
        "run this operation on Linux x86-64; other native hosts are not implemented",
    )])
}
