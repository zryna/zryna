//! Atomic one-entrypoint build and run orchestration.

use std::{
    fmt, fs,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use same_file::Handle;
use serde::Serialize;
use sha2::{Digest, Sha256};
use zryna_abi::{RawHostScalar, ScalarHostErrorCode, ScalarOutcome, ScalarTarget, ScalarValue};
use zryna_diagnostics::Diagnostic;
use zryna_frontend::VerifiedFrontendProvider;
use zryna_frontend::{
    FrontendCapabilities, ProviderExpectation, WorkerFrontend, WorkerLimits, WorkerSpec, syntax_v2,
};
use zryna_source::{MAX_SOURCE_FILE_BYTES, SourceFileInput, SourceMap};

use crate::{
    ArtifactOutputRoot, NativeProcessLimits, SourceToIrError, compile_to_verified_ir,
    discover_linux_native_toolchain,
    javascript::validate_artifact_stem,
    native::{prepare_native_invocation_from_verified, run_prepared_native_invocation},
    runtime::{NodeRuntimeCapability, node_compatible_path},
};

const MANIFEST_NAME: &str = "zryna-manifest-v1.json";
const MANIFEST_PROFILE: &str = "zryna-m1-cli-v1";
const NATIVE_TARGET: &str = "x86_64-unknown-linux-gnu";
const TRANSACTION_PREFIX: &str = ".zryna-transaction-";
static NEXT_TRANSACTION: AtomicU64 = AtomicU64::new(0);

/// Closed CLI target selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetSelection {
    /// Direct ECMAScript module.
    JavaScript,
    /// Import-free core WebAssembly module.
    WebAssembly,
    /// Linux x86-64 native object or invocation executable.
    Native,
    /// Every target in canonical order.
    All,
}

impl TargetSelection {
    fn javascript(self) -> bool {
        matches!(self, Self::JavaScript | Self::All)
    }

    fn webassembly(self) -> bool {
        matches!(self, Self::WebAssembly | Self::All)
    }

    fn native(self) -> bool {
        matches!(self, Self::Native | Self::All)
    }

    fn ordered(self) -> Vec<ManifestTarget> {
        let mut targets = Vec::with_capacity(if self == Self::All { 3 } else { 1 });
        if self.javascript() {
            targets.push(ManifestTarget::JavaScript);
        }
        if self.webassembly() {
            targets.push(ManifestTarget::WebAssembly);
        }
        if self.native() {
            targets.push(ManifestTarget::Native);
        }
        targets
    }
}

impl fmt::Display for TargetSelection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::JavaScript => "javascript",
            Self::WebAssembly => "webassembly",
            Self::Native => "native",
            Self::All => "all",
        })
    }
}

/// One build request after CLI parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildRequest {
    /// Absolute canonical workspace root.
    pub workspace_root: PathBuf,
    /// Portable workspace-relative `.zry` path.
    pub entrypoint: String,
    /// Portable output stem.
    pub artifact_stem: String,
    /// Explicit target selection.
    pub targets: TargetSelection,
    /// Absolute direct Node.js executable used by the frontend and target runtimes.
    pub node_runtime: PathBuf,
}

/// One run request after CLI parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunRequest {
    /// Shared build configuration.
    pub build: BuildRequest,
    /// Exact logical scalar export.
    pub logical_export: String,
    /// Ordered typed arguments.
    pub arguments: Vec<ScalarValue>,
}

/// Executed command kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandKind {
    /// Artifact-only build.
    Build,
    /// Artifact build plus typed execution.
    Run,
}

impl CommandKind {
    const fn suffix(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Run => "run",
        }
    }
}

/// Stable coarse command failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandFailureKind {
    /// Mandatory workspace architecture failed.
    Architecture,
    /// Request paths, names, or configuration failed validation.
    Request,
    /// Frontend, source, semantic, ABI, or IR verification rejected the input.
    Source,
    /// Backend, toolchain, audit, transaction, or commit failed.
    Preparation,
    /// Target execution or result framing failed.
    Execution,
    /// Process-tree or transaction cleanup could not be confirmed.
    Cleanup,
}

impl CommandFailureKind {
    /// Returns the stable process exit status.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Architecture => 1,
            Self::Request => 2,
            Self::Source => 3,
            Self::Preparation => 4,
            Self::Execution => 5,
            Self::Cleanup => 6,
        }
    }
}

/// Failed command with deterministic diagnostics.
#[derive(Clone, Debug)]
pub struct CommandFailure {
    kind: CommandFailureKind,
    diagnostics: Vec<Diagnostic>,
}

impl CommandFailure {
    /// Returns the stable coarse failure kind.
    #[must_use]
    pub const fn kind(&self) -> CommandFailureKind {
        self.kind
    }

    /// Returns deterministic diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for CommandFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Zryna command failed with {} diagnostic(s)", self.diagnostics.len())
    }
}

impl std::error::Error for CommandFailure {}

/// One artifact recorded in the committed manifest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PublishedTargetArtifact {
    target: ManifestTarget,
    kind: &'static str,
    path: String,
    bytes: u64,
    sha256: String,
}

impl PublishedTargetArtifact {
    /// Returns the target label.
    #[must_use]
    pub const fn target(&self) -> &'static str {
        self.target.as_str()
    }

    /// Returns the bundle-relative artifact path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// One ordered typed target observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetResult {
    target: ManifestTarget,
    outcome: ScalarOutcome,
}

impl TargetResult {
    /// Returns the target label.
    #[must_use]
    pub const fn target(&self) -> &'static str {
        self.target.as_str()
    }

    /// Returns the normalized typed outcome.
    #[must_use]
    pub const fn outcome(&self) -> ScalarOutcome {
        self.outcome
    }
}

/// Successful atomic build or run.
#[derive(Clone, Debug)]
pub struct CommandSuccess {
    command: CommandKind,
    manifest_path: PathBuf,
    manifest_portable_path: String,
    artifacts: Vec<PublishedTargetArtifact>,
    results: Vec<TargetResult>,
    diagnostics: Vec<Diagnostic>,
}

impl CommandSuccess {
    /// Returns the completed command kind.
    #[must_use]
    pub const fn command(&self) -> CommandKind {
        self.command
    }

    /// Returns the absolute committed manifest path.
    #[must_use]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// Returns the portable workspace-relative committed manifest path.
    #[must_use]
    pub fn manifest_portable_path(&self) -> &str {
        &self.manifest_portable_path
    }

    /// Returns canonical target artifacts.
    #[must_use]
    pub fn artifacts(&self) -> &[PublishedTargetArtifact] {
        &self.artifacts
    }

    /// Returns canonical typed observations; builds return an empty slice.
    #[must_use]
    pub fn results(&self) -> &[TargetResult] {
        &self.results
    }

    /// Returns non-fatal diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ManifestTarget {
    JavaScript,
    WebAssembly,
    Native,
}

impl ManifestTarget {
    const fn as_str(self) -> &'static str {
        match self {
            Self::JavaScript => "javascript",
            Self::WebAssembly => "webassembly",
            Self::Native => "native",
        }
    }
}

#[derive(Serialize)]
struct Manifest<'a> {
    version: u8,
    profile: &'static str,
    command: CommandKind,
    entrypoint: &'a str,
    source_sha256: String,
    stem: &'a str,
    targets: Vec<ManifestTarget>,
    artifacts: &'a [PublishedTargetArtifact],
    invocation: Option<ManifestInvocation<'a>>,
    results: Vec<ManifestResult>,
    diagnostics: &'a [Diagnostic],
}

#[derive(Serialize)]
struct ManifestInvocation<'a> {
    export: &'a str,
    arguments: &'a [ScalarValue],
}

#[derive(Serialize)]
struct ManifestResult {
    target: ManifestTarget,
    outcome: ManifestOutcome,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ManifestOutcome {
    Returned {
        #[serde(rename = "type")]
        ty: &'static str,
        value: ManifestScalar,
    },
    Trapped {
        code: &'static str,
    },
    HostError {
        code: &'static str,
    },
}

#[derive(Serialize)]
#[serde(untagged)]
enum ManifestScalar {
    Bool(bool),
    I32(i32),
}

struct PreparedArtifacts {
    javascript: Option<zryna_backend_javascript::JavaScriptArtifact>,
    webassembly: Option<zryna_backend_webassembly::ValidatedWebAssemblyArtifact>,
    native_object: Option<zryna_backend_native::ValidatedNativeObjectArtifact>,
    native_executable: Option<crate::native::PreparedNativeExecutable>,
}

fn compile_selected<Provider: VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
    targets: TargetSelection,
) -> Result<(crate::SourceToIrSuccess, PreparedArtifacts), CommandFailure> {
    let compiled =
        compile_to_verified_ir(frontend, sources).map_err(|error| source_failure(&error))?;
    let program = compiled.program();
    let javascript = if targets.javascript() {
        Some(zryna_backend_javascript::emit(program).map_err(preparation_failure)?)
    } else {
        None
    };
    let webassembly = if targets.webassembly() {
        Some(zryna_backend_webassembly::emit(program).map_err(preparation_failure)?)
    } else {
        None
    };
    let native_object = if targets.native() {
        let target =
            crate::select_native_object_target(NATIVE_TARGET).map_err(preparation_failure)?;
        let mir = zryna_native_mir::lower(program).map_err(|diagnostics| CommandFailure {
            kind: CommandFailureKind::Preparation,
            diagnostics,
        })?;
        Some(zryna_backend_native::emit_object(&mir, target).map_err(preparation_failure)?)
    } else {
        None
    };
    Ok((
        compiled,
        PreparedArtifacts { javascript, webassembly, native_object, native_executable: None },
    ))
}

/// Builds one entrypoint and atomically commits one complete target bundle.
///
/// # Errors
///
/// Returns stable categorized diagnostics without advertising a partial bundle.
pub fn build_workspace(request: &BuildRequest) -> Result<CommandSuccess, CommandFailure> {
    execute(request, None)
}

/// Builds, executes, and atomically commits one complete target bundle.
///
/// # Errors
///
/// Returns stable categorized diagnostics without advertising a partial bundle.
pub fn run_workspace(request: RunRequest) -> Result<CommandSuccess, CommandFailure> {
    let invocation =
        RunInvocation { logical_export: request.logical_export, arguments: request.arguments };
    execute(&request.build, Some(&invocation))
}

struct RunInvocation {
    logical_export: String,
    arguments: Vec<ScalarValue>,
}

fn execute(
    request: &BuildRequest,
    run: Option<&RunInvocation>,
) -> Result<CommandSuccess, CommandFailure> {
    let command = if run.is_some() { CommandKind::Run } else { CommandKind::Build };
    validate_architecture(&request.workspace_root)?;
    let validated = validate_request(request, command)?;
    let output_root = ArtifactOutputRoot::prepare_for_workspace(&request.workspace_root)
        .map_err(|diagnostic| failure(CommandFailureKind::Preparation, diagnostic))?;
    let final_bundle =
        output_root.path().join(format!("{}.{}", request.artifact_stem, command.suffix()));
    ensure_absent(&final_bundle)?;

    let source_text = read_entrypoint(&validated.source_path)?;
    let sources = SourceMap::build(vec![SourceFileInput {
        path: request.entrypoint.clone(),
        text: source_text.clone(),
    }])
    .map_err(|error| failure(CommandFailureKind::Source, Diagnostic::from_source_error(&error)))?;
    let node = NodeRuntimeCapability::discover(&request.node_runtime, &request.workspace_root)
        .map_err(|diagnostic| failure(CommandFailureKind::Preparation, diagnostic))?;
    let frontend = configured_frontend(request, &node)?;
    let (compiled, mut prepared) = compile_selected(&frontend, &sources, request.targets)?;
    node.revalidate().map_err(preparation_failure)?;
    let program = compiled.program();
    let verified_invocation = run
        .map(|invocation| {
            program.prepare_invocation(zryna_abi::Invocation::new(
                invocation.logical_export.clone(),
                invocation.arguments.clone(),
            ))
        })
        .transpose()
        .map_err(|error| {
            failure(
                CommandFailureKind::Source,
                Diagnostic::error(
                    error.code(),
                    None,
                    "run invocation does not match the verified scalar ABI export",
                    "use the exact export, arity, and scalar argument types",
                ),
            )
        })?;

    let native_executable = if run.is_some() && request.targets.native() {
        let invocation = verified_invocation.as_ref().ok_or_else(|| {
            request_error(
                "ZRYNA-C1010",
                "verified invocation preparation was not completed",
                "report this compiler invariant failure",
            )
        })?;
        let object = prepared.native_object.as_ref().ok_or_else(|| {
            request_error(
                "ZRYNA-C1010",
                "native object preparation was not completed",
                "report this compiler invariant failure",
            )
        })?;
        let toolchain = discover_linux_native_toolchain(NativeProcessLimits::default())
            .map_err(preparation_failure)?;
        Some(
            prepare_native_invocation_from_verified(
                program,
                object,
                invocation,
                &output_root,
                &toolchain,
                NativeProcessLimits::default(),
            )
            .map_err(native_preparation_failure)?,
        )
    } else {
        None
    };

    let mut diagnostics = compiled.diagnostics().to_vec();
    if let Some(executable) = &native_executable {
        diagnostics.extend_from_slice(executable.diagnostics());
    }
    prepared.native_executable = native_executable;
    commit_prepared(
        request,
        command,
        run,
        verified_invocation.as_ref(),
        &node,
        &output_root,
        &final_bundle,
        &source_text,
        &prepared,
        diagnostics,
    )
}

struct ValidatedRequest {
    source_path: PathBuf,
}

fn validate_architecture(root: &Path) -> Result<(), CommandFailure> {
    let report = crate::check_workspace(root);
    if report.is_valid() {
        Ok(())
    } else {
        Err(CommandFailure {
            kind: CommandFailureKind::Architecture,
            diagnostics: report.diagnostics,
        })
    }
}

fn validate_request(
    request: &BuildRequest,
    _command: CommandKind,
) -> Result<ValidatedRequest, CommandFailure> {
    if !request.workspace_root.is_absolute() {
        return Err(request_error(
            "ZRYNA-C1001",
            "workspace root must be absolute",
            "resolve --root to an absolute real directory before dispatch",
        ));
    }
    validate_artifact_stem(&request.artifact_stem)
        .map_err(|diagnostic| failure(CommandFailureKind::Request, diagnostic))?;
    if request.entrypoint.contains('\\') {
        return Err(entrypoint_error("entrypoint must use portable forward slashes"));
    }
    let has_noncanonical_segment = request
        .entrypoint
        .split('/')
        .any(|segment| segment.is_empty() || matches!(segment, "." | ".."));
    let entry = Path::new(&request.entrypoint);
    if has_noncanonical_segment
        || entry.is_absolute()
        || entry.extension().and_then(|value| value.to_str()) != Some("zry")
        || entry.components().any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(entrypoint_error(
            "entrypoint must be one portable workspace-relative .zry path",
        ));
    }
    validate_real_directory(&request.workspace_root)
        .map_err(|diagnostic| failure(CommandFailureKind::Request, diagnostic))?;
    let source_path = request.workspace_root.join(entry);
    validate_source_chain(&request.workspace_root, &source_path)?;
    Ok(ValidatedRequest { source_path })
}

fn validate_source_chain(root: &Path, source: &Path) -> Result<(), CommandFailure> {
    let parent = source.parent().ok_or_else(|| entrypoint_error("entrypoint has no parent"))?;
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| entrypoint_error("entrypoint escapes the workspace root"))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(entrypoint_error("entrypoint contains an unsafe path component"));
        };
        current.push(component);
        validate_real_directory(&current)
            .map_err(|diagnostic| failure(CommandFailureKind::Request, diagnostic))?;
    }
    let metadata = fs::symlink_metadata(source)
        .map_err(|_| entrypoint_error("entrypoint could not be inspected"))?;
    if !metadata.is_file() || metadata_is_link_or_reparse(&metadata) {
        return Err(entrypoint_error("entrypoint is not a real regular file"));
    }
    Ok(())
}

fn validate_real_directory(path: &Path) -> Result<(), Diagnostic> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        Diagnostic::error(
            "ZRYNA-C1002",
            None,
            "workspace path could not be inspected",
            "use an existing real directory without links or reparse points",
        )
    })?;
    if metadata.is_dir() && !metadata_is_link_or_reparse(&metadata) {
        Ok(())
    } else {
        Err(Diagnostic::error(
            "ZRYNA-C1002",
            None,
            "workspace path is not a real directory",
            "use an existing real directory without links or reparse points",
        ))
    }
}

fn read_entrypoint(path: &Path) -> Result<String, CommandFailure> {
    read_entrypoint_impl(path, || {})
}

#[cfg(all(test, unix))]
fn read_entrypoint_with_after_read(
    path: &Path,
    after_read: impl FnOnce(),
) -> Result<String, CommandFailure> {
    read_entrypoint_impl(path, after_read)
}

fn read_entrypoint_impl(path: &Path, after_read: impl FnOnce()) -> Result<String, CommandFailure> {
    let file = open_regular_no_follow(path)
        .map_err(|_| entrypoint_error("entrypoint could not be opened safely"))?;
    let mut handle = Handle::from_file(file)
        .map_err(|_| entrypoint_error("entrypoint identity could not be established"))?;
    let opened = handle
        .as_file()
        .metadata()
        .map_err(|_| entrypoint_error("entrypoint metadata could not be read"))?;
    if !opened.is_file() || metadata_is_link_or_reparse(&opened) {
        return Err(entrypoint_error("entrypoint changed before it could be read"));
    }
    if opened.len() > u64::try_from(MAX_SOURCE_FILE_BYTES).unwrap_or(u64::MAX) {
        return Err(source_size_error());
    }
    let mut bytes = Vec::with_capacity(
        usize::try_from(opened.len()).unwrap_or(MAX_SOURCE_FILE_BYTES).min(MAX_SOURCE_FILE_BYTES),
    );
    handle
        .as_file_mut()
        .take(u64::try_from(MAX_SOURCE_FILE_BYTES).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| entrypoint_error("entrypoint could not be read completely"))?;
    if bytes.len() > MAX_SOURCE_FILE_BYTES {
        return Err(source_size_error());
    }
    after_read();
    let after = handle
        .as_file()
        .metadata()
        .map_err(|_| entrypoint_error("entrypoint changed while it was read"))?;
    let current_file = open_regular_no_follow(path)
        .map_err(|_| entrypoint_error("entrypoint changed while it was read"))?;
    let current = Handle::from_file(current_file)
        .map_err(|_| entrypoint_error("entrypoint identity could not be revalidated"))?;
    let current_metadata = current
        .as_file()
        .metadata()
        .map_err(|_| entrypoint_error("entrypoint metadata could not be revalidated"))?;
    if handle != current
        || !same_file_state(&opened, &after)
        || !same_file_state(&opened, &current_metadata)
    {
        return Err(entrypoint_error("entrypoint changed while it was read"));
    }
    String::from_utf8(bytes).map_err(|_| entrypoint_error("entrypoint is not valid UTF-8"))
}

#[cfg(unix)]
fn open_regular_no_follow(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = fs::OpenOptions::new();
    options.read(true).custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    options.open(path)
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    let mut options = fs::OpenOptions::new();
    options.read(true).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).share_mode(FILE_SHARE_READ);
    options.open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_regular_no_follow(_path: &Path) -> io::Result<fs::File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no fail-closed no-follow strategy exists for this platform",
    ))
}

#[cfg(unix)]
fn same_file_state(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.size() == right.size()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(windows)]
fn same_file_state(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    left.file_attributes() == right.file_attributes()
        && left.creation_time() == right.creation_time()
        && left.last_write_time() == right.last_write_time()
        && left.file_size() == right.file_size()
}

#[cfg(not(any(unix, windows)))]
fn same_file_state(_left: &fs::Metadata, _right: &fs::Metadata) -> bool {
    false
}

fn source_size_error() -> CommandFailure {
    failure(
        CommandFailureKind::Source,
        Diagnostic::error(
            "ZRYNA-S1003",
            None,
            "source file exceeds the fixed per-file byte budget",
            "reduce the source file before analysis",
        ),
    )
}

fn configured_frontend(
    request: &BuildRequest,
    node: &NodeRuntimeCapability,
) -> Result<WorkerFrontend, CommandFailure> {
    let adapter_root = request.workspace_root.join("adapters/typescript-6");
    validate_real_directory(&adapter_root)
        .map_err(|diagnostic| failure(CommandFailureKind::Preparation, diagnostic))?;
    let worker_entrypoint = adapter_root.join("src/worker.mjs");
    let worker_metadata = fs::symlink_metadata(&worker_entrypoint).map_err(|_| {
        entrypoint_error("TypeScript frontend worker entrypoint is unavailable")
            .with_kind(CommandFailureKind::Preparation)
    })?;
    if !worker_metadata.is_file() || metadata_is_link_or_reparse(&worker_metadata) {
        return Err(entrypoint_error(
            "TypeScript frontend worker entrypoint is not a real regular file",
        )
        .with_kind(CommandFailureKind::Preparation));
    }
    let node_adapter_root = node_compatible_path(&adapter_root);
    let node_worker_entrypoint = node_compatible_path(&worker_entrypoint);
    let expected = ProviderExpectation::new(
        "typescript-6",
        "6.0.3",
        syntax_v2::PROTOCOL_VERSION,
        FrontendCapabilities { module_resolution: false, semantic_diagnostics: false },
    )
    .map_err(|error| CommandFailure {
        kind: CommandFailureKind::Preparation,
        diagnostics: error.diagnostics().to_vec(),
    })?;
    let spec = WorkerSpec::new(
        node.executable().map_err(preparation_failure)?,
        vec![node_worker_entrypoint.into_os_string()],
        node_adapter_root,
        expected,
        WorkerLimits::default(),
    )
    .map_err(|error| CommandFailure {
        kind: CommandFailureKind::Preparation,
        diagnostics: error.diagnostics().to_vec(),
    })?;
    Ok(WorkerFrontend::new(spec))
}

#[allow(clippy::too_many_arguments)]
fn commit_prepared(
    request: &BuildRequest,
    command: CommandKind,
    run: Option<&RunInvocation>,
    invocation: Option<&zryna_abi::VerifiedInvocation<'_>>,
    node: &NodeRuntimeCapability,
    output_root: &ArtifactOutputRoot,
    final_bundle: &Path,
    source_text: &str,
    prepared: &PreparedArtifacts,
    diagnostics: Vec<Diagnostic>,
) -> Result<CommandSuccess, CommandFailure> {
    let mut transaction = Transaction::create(output_root)?;
    let operation: Result<CommandSuccess, CommandFailure> = (|| {
        let artifacts = write_prepared_artifacts(&transaction, request, command, prepared)?;

        let results = if let Some(invocation) = invocation {
            execute_targets(request, node, output_root, &transaction, prepared, invocation)?
        } else {
            Vec::new()
        };
        let manifest_results = results.iter().map(manifest_result).collect::<Vec<_>>();
        let source_sha256 = sha256(source_text.as_bytes());
        let manifest = Manifest {
            version: 1,
            profile: MANIFEST_PROFILE,
            command,
            entrypoint: &request.entrypoint,
            source_sha256,
            stem: &request.artifact_stem,
            targets: request.targets.ordered(),
            artifacts: &artifacts,
            invocation: run.map(|value| ManifestInvocation {
                export: &value.logical_export,
                arguments: &value.arguments,
            }),
            results: manifest_results,
            diagnostics: &diagnostics,
        };
        let mut manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|_| {
            request_error(
                "ZRYNA-C1011",
                "manifest serialization failed",
                "report this compiler invariant failure",
            )
        })?;
        manifest_bytes.push(b'\n');
        transaction.write_manifest(&manifest_bytes)?;
        transaction.commit(output_root, final_bundle)?;
        Ok(CommandSuccess {
            command,
            manifest_path: final_bundle.join(MANIFEST_NAME),
            manifest_portable_path: format!(
                ".zryna/out/{}.{}/{MANIFEST_NAME}",
                request.artifact_stem,
                command.suffix()
            ),
            artifacts,
            results,
            diagnostics,
        })
    })();
    match operation {
        Ok(success) => Ok(success),
        Err(mut failure) => {
            if let Err(cleanup) = transaction.cleanup(output_root) {
                failure.kind = CommandFailureKind::Cleanup;
                failure.diagnostics.extend(cleanup.diagnostics);
            }
            Err(failure)
        }
    }
}

fn write_prepared_artifacts(
    transaction: &Transaction,
    request: &BuildRequest,
    command: CommandKind,
    prepared: &PreparedArtifacts,
) -> Result<Vec<PublishedTargetArtifact>, CommandFailure> {
    let mut artifacts = Vec::with_capacity(request.targets.ordered().len());
    if let Some(artifact) = &prepared.javascript {
        artifacts.push(transaction.write_artifact(
            ManifestTarget::JavaScript,
            "ecmascript-module",
            &request.artifact_stem,
            "mjs",
            artifact.source.as_bytes(),
        )?);
    }
    if let Some(artifact) = &prepared.webassembly {
        artifacts.push(transaction.write_artifact(
            ManifestTarget::WebAssembly,
            "core-webassembly-module",
            &request.artifact_stem,
            "wasm",
            artifact.bytes(),
        )?);
    }
    if request.targets.native() {
        let (kind, extension, bytes): (&str, &str, &[u8]) = if command == CommandKind::Run {
            let executable = prepared.native_executable.as_ref().ok_or_else(|| {
                request_error(
                    "ZRYNA-C1010",
                    "native executable preparation was not completed",
                    "report this compiler invariant failure",
                )
            })?;
            ("linux-x86-64-invocation-executable", "elf", executable.bytes())
        } else {
            let object = prepared.native_object.as_ref().ok_or_else(|| {
                request_error(
                    "ZRYNA-C1010",
                    "native object preparation was not completed",
                    "report this compiler invariant failure",
                )
            })?;
            ("linux-x86-64-relocatable-object", "o", object.bytes())
        };
        artifacts.push(transaction.write_artifact(
            ManifestTarget::Native,
            kind,
            &request.artifact_stem,
            extension,
            bytes,
        )?);
    }
    Ok(artifacts)
}

fn execute_targets(
    request: &BuildRequest,
    node: &NodeRuntimeCapability,
    output_root: &ArtifactOutputRoot,
    transaction: &Transaction,
    prepared: &PreparedArtifacts,
    invocation: &zryna_abi::VerifiedInvocation<'_>,
) -> Result<Vec<TargetResult>, CommandFailure> {
    let mut results = Vec::with_capacity(request.targets.ordered().len());
    if request.targets.javascript() {
        let harness = render_javascript_harness(&request.artifact_stem, invocation)?;
        let harness_path = transaction.write_runtime_harness("javascript", &harness)?;
        let frame = node.run_module(&harness_path, transaction.path()).map_err(runtime_failure)?;
        results.push(TargetResult {
            target: ManifestTarget::JavaScript,
            outcome: normalize_frame(ScalarTarget::JavaScript, invocation.export().result(), frame),
        });
    }
    if request.targets.webassembly() {
        let harness = render_webassembly_harness(&request.artifact_stem, invocation)?;
        let harness_path = transaction.write_runtime_harness("webassembly", &harness)?;
        let frame = node.run_module(&harness_path, transaction.path()).map_err(runtime_failure)?;
        results.push(TargetResult {
            target: ManifestTarget::WebAssembly,
            outcome: normalize_frame(
                ScalarTarget::CoreWebAssembly,
                invocation.export().result(),
                frame,
            ),
        });
    }
    if request.targets.native() {
        let executable = prepared.native_executable.as_ref().ok_or_else(|| {
            request_error(
                "ZRYNA-C1010",
                "native executable preparation was not completed",
                "report this compiler invariant failure",
            )
        })?;
        let outcome =
            run_prepared_native_invocation(executable, output_root, NativeProcessLimits::default())
                .map_err(|error| native_runtime_failure(error.diagnostic().clone()))?;
        results.push(TargetResult { target: ManifestTarget::Native, outcome });
    }
    Ok(results)
}

fn render_javascript_harness(
    stem: &str,
    invocation: &zryna_abi::VerifiedInvocation<'_>,
) -> Result<Vec<u8>, CommandFailure> {
    let arguments = render_i32_arguments(invocation)?;
    let module_path =
        serde_json::to_string(&format!("./javascript/{stem}.mjs")).map_err(invariant_failure)?;
    let export = invocation.export().javascript_name().as_str();
    Ok(format!(
        "import {{ {export} as invoke }} from {module_path};\nconst value = invoke({arguments});\nif (!Number.isInteger(value) || value < -2147483648 || value > 2147483647 || Object.is(value, -0)) process.exit(70);\nconst frame = Buffer.allocUnsafe(4);\nframe.writeInt32LE(value, 0);\nprocess.stdout.write(frame);\n"
    )
    .into_bytes())
}

fn render_webassembly_harness(
    stem: &str,
    invocation: &zryna_abi::VerifiedInvocation<'_>,
) -> Result<Vec<u8>, CommandFailure> {
    let arguments = render_i32_arguments(invocation)?;
    let module_path =
        serde_json::to_string(&format!("./webassembly/{stem}.wasm")).map_err(invariant_failure)?;
    let export = serde_json::to_string(invocation.export().webassembly_name().as_str())
        .map_err(invariant_failure)?;
    Ok(format!(
        "import {{ readFileSync }} from 'node:fs';\nconst bytes = readFileSync({module_path});\nconst {{ instance }} = await WebAssembly.instantiate(bytes, {{}});\nconst invoke = instance.exports[{export}];\nif (typeof invoke !== 'function') process.exit(70);\nconst value = invoke({arguments});\nconst frame = Buffer.allocUnsafe(4);\nframe.writeInt32LE(value, 0);\nprocess.stdout.write(frame);\n"
    )
    .into_bytes())
}

fn render_i32_arguments(
    invocation: &zryna_abi::VerifiedInvocation<'_>,
) -> Result<String, CommandFailure> {
    let mut rendered = Vec::with_capacity(invocation.arguments().len());
    for argument in invocation.arguments() {
        match argument {
            ScalarValue::I32(value) => rendered.push(value.to_string()),
            ScalarValue::Bool(_) => {
                return Err(request_error(
                    "ZRYNA-C1008",
                    "Boolean execution is outside the current I32V1 profile",
                    "use signed 32-bit arguments until the Boolean executable profile lands",
                ));
            }
        }
    }
    Ok(rendered.join(", "))
}

fn normalize_frame(
    target: ScalarTarget,
    result_type: zryna_abi::ScalarType,
    frame: [u8; 4],
) -> ScalarOutcome {
    let raw = i32::from_le_bytes(frame);
    let carrier = match target {
        ScalarTarget::JavaScript => RawHostScalar::JavaScriptNumber(f64::from(raw)),
        ScalarTarget::CoreWebAssembly | ScalarTarget::NativeLinuxX8664 => RawHostScalar::I32(raw),
    };
    match zryna_abi::normalize_result(target, result_type, carrier) {
        Ok(value) => ScalarOutcome::Returned { value },
        Err(_) => ScalarOutcome::HostError { code: ScalarHostErrorCode::InvalidTargetResult },
    }
}

struct Transaction {
    path: PathBuf,
    identity: Handle,
    committed: bool,
}

impl Transaction {
    fn create(output_root: &ArtifactOutputRoot) -> Result<Self, CommandFailure> {
        output_root.revalidate().map_err(preparation_failure)?;
        for _ in 0..64 {
            let sequence = NEXT_TRANSACTION.fetch_add(1, Ordering::Relaxed);
            let path = output_root
                .path()
                .join(format!("{TRANSACTION_PREFIX}{}-{sequence}", std::process::id()));
            match create_private_directory(&path) {
                Ok(()) => {
                    let Ok(identity) = Handle::from_path(&path) else {
                        let _ = fs::remove_dir(&path);
                        return Err(transaction_error(
                            "could not establish private stage identity",
                        ));
                    };
                    let transaction = Self { path, identity, committed: false };
                    transaction.revalidate_stage()?;
                    return Ok(transaction);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(transaction_error("could not create private stage")),
            }
        }
        Err(transaction_error("could not allocate a unique private stage"))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write_artifact(
        &self,
        target: ManifestTarget,
        kind: &'static str,
        stem: &str,
        extension: &str,
        bytes: &[u8],
    ) -> Result<PublishedTargetArtifact, CommandFailure> {
        self.revalidate_stage()?;
        let directory = self.path.join(target.as_str());
        create_owned_directory(&directory)?;
        let filename = format!("{stem}.{extension}");
        let path = directory.join(&filename);
        write_complete_file(&path, bytes)?;
        if target == ManifestTarget::Native && extension == "elf" {
            prepare_executable_file(&path)?;
        }
        self.revalidate_stage()?;
        Ok(PublishedTargetArtifact {
            target,
            kind,
            path: format!("{}/{filename}", target.as_str()),
            bytes: u64::try_from(bytes.len()).map_err(|_| {
                transaction_error("artifact length could not be represented in the manifest")
            })?,
            sha256: sha256(bytes),
        })
    }

    fn write_runtime_harness(&self, target: &str, bytes: &[u8]) -> Result<PathBuf, CommandFailure> {
        self.revalidate_stage()?;
        let path = self.path.join(format!(".{target}-runtime.mjs"));
        write_complete_file(&path, bytes)?;
        self.revalidate_stage()?;
        Ok(path)
    }

    fn write_manifest(&self, bytes: &[u8]) -> Result<(), CommandFailure> {
        self.revalidate_stage()?;
        for target in [".javascript-runtime.mjs", ".webassembly-runtime.mjs"] {
            let path = self.path.join(target);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => {
                    return Err(transaction_error("could not remove private runtime harness"));
                }
            }
        }
        write_complete_file(&self.path.join(MANIFEST_NAME), bytes)?;
        sync_directory(&self.path)?;
        self.revalidate_stage()
    }

    fn commit(
        &mut self,
        output_root: &ArtifactOutputRoot,
        final_bundle: &Path,
    ) -> Result<(), CommandFailure> {
        output_root.revalidate().map_err(preparation_failure)?;
        self.revalidate_stage()?;
        ensure_absent(final_bundle)?;
        rename_create_only(&self.path, final_bundle)?;
        self.committed = true;
        Ok(())
    }

    fn cleanup(&mut self, output_root: &ArtifactOutputRoot) -> Result<(), CommandFailure> {
        if self.committed {
            return Ok(());
        }
        output_root
            .revalidate()
            .map_err(|diagnostic| failure(CommandFailureKind::Cleanup, diagnostic))?;
        self.revalidate_stage().map_err(|failure| CommandFailure {
            kind: CommandFailureKind::Cleanup,
            diagnostics: failure.diagnostics,
        })?;
        let Some(name) = self.path.file_name().and_then(|value| value.to_str()) else {
            return Err(cleanup_failure("private stage name could not be validated"));
        };
        if self.path.parent() != Some(output_root.path()) || !name.starts_with(TRANSACTION_PREFIX) {
            return Err(cleanup_failure("private stage escaped the validated output root"));
        }
        match fs::remove_dir_all(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(cleanup_failure("private stage could not be fully removed")),
        }
    }

    fn revalidate_stage(&self) -> Result<(), CommandFailure> {
        let metadata = fs::symlink_metadata(&self.path)
            .map_err(|_| transaction_error("private stage identity could not be inspected"))?;
        if !metadata.is_dir() || metadata_is_link_or_reparse(&metadata) {
            return Err(transaction_error("private stage is not a real directory"));
        }
        let current = Handle::from_path(&self.path)
            .map_err(|_| transaction_error("private stage identity could not be opened"))?;
        if current != self.identity {
            return Err(transaction_error("private stage identity changed during the operation"));
        }
        let after = fs::symlink_metadata(&self.path)
            .map_err(|_| transaction_error("private stage identity could not be revalidated"))?;
        if !after.is_dir() || metadata_is_link_or_reparse(&after) {
            return Err(transaction_error("private stage changed during revalidation"));
        }
        Ok(())
    }
}

fn create_owned_directory(path: &Path) -> Result<(), CommandFailure> {
    create_private_directory(path)
        .map_err(|_| transaction_error("could not create secure target stage directory"))
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

fn write_complete_file(path: &Path, bytes: &[u8]) -> Result<(), CommandFailure> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| transaction_error("could not create staged file"))?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all())
        .map_err(|_| transaction_error("could not write and synchronize staged file"))
}

#[cfg(unix)]
fn prepare_executable_file(path: &Path) -> Result<(), CommandFailure> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .and_then(|()| fs::File::open(path)?.sync_all())
        .map_err(|_| transaction_error("could not apply and synchronize executable mode"))
}

#[cfg(not(unix))]
fn prepare_executable_file(path: &Path) -> Result<(), CommandFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| transaction_error("could not revalidate executable artifact"))?;
    if metadata.is_file() && !metadata_is_link_or_reparse(&metadata) {
        Ok(())
    } else {
        Err(transaction_error("executable artifact is not a real regular file"))
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), CommandFailure> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| transaction_error("could not synchronize transaction directory"))
}

#[cfg(not(unix))]
fn sync_directory(path: &Path) -> Result<(), CommandFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| transaction_error("could not revalidate transaction directory"))?;
    if metadata.is_dir() && !metadata_is_link_or_reparse(&metadata) {
        Ok(())
    } else {
        Err(transaction_error("transaction path is not a real directory"))
    }
}

#[cfg(target_os = "linux")]
fn rename_create_only(source: &Path, destination: &Path) -> Result<(), CommandFailure> {
    use nix::fcntl::{AT_FDCWD, RenameFlags, renameat2};

    renameat2(AT_FDCWD, source, AT_FDCWD, destination, RenameFlags::RENAME_NOREPLACE)
        .map_err(|_| transaction_error("create-only bundle commit failed"))
}

#[cfg(windows)]
fn rename_create_only(source: &Path, destination: &Path) -> Result<(), CommandFailure> {
    fs::rename(source, destination)
        .map_err(|_| transaction_error("create-only bundle commit failed"))
}

#[cfg(not(any(target_os = "linux", windows)))]
fn rename_create_only(_source: &Path, _destination: &Path) -> Result<(), CommandFailure> {
    Err(transaction_error("create-only bundle commit is unsupported on this platform"))
}

fn ensure_absent(path: &Path) -> Result<(), CommandFailure> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(transaction_error("create-only output bundle already exists")),
        Err(_) => Err(transaction_error("output bundle destination could not be inspected")),
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn manifest_result(result: &TargetResult) -> ManifestResult {
    let outcome = match result.outcome {
        ScalarOutcome::Returned { value: ScalarValue::Bool(value) } => {
            ManifestOutcome::Returned { ty: "bool", value: ManifestScalar::Bool(value) }
        }
        ScalarOutcome::Returned { value: ScalarValue::I32(value) } => {
            ManifestOutcome::Returned { ty: "i32", value: ManifestScalar::I32(value) }
        }
        ScalarOutcome::Trapped { code } => ManifestOutcome::Trapped {
            code: match code {
                zryna_abi::ScalarTrapCode::Unreachable => "unreachable",
                zryna_abi::ScalarTrapCode::TargetTrap => "target-trap",
            },
        },
        ScalarOutcome::HostError { code } => ManifestOutcome::HostError {
            code: match code {
                ScalarHostErrorCode::UnknownExport => "unknown-export",
                ScalarHostErrorCode::InvalidInvocation => "invalid-invocation",
                ScalarHostErrorCode::InvalidTargetResult => "invalid-target-result",
                ScalarHostErrorCode::TargetUnavailable => "target-unavailable",
            },
        },
    };
    ManifestResult { target: result.target, outcome }
}

fn source_failure(error: &SourceToIrError) -> CommandFailure {
    let diagnostics = match error {
        SourceToIrError::Frontend(frontend) if frontend.diagnostics().is_empty() => {
            vec![Diagnostic::error(
                frontend.code(),
                None,
                frontend.to_string(),
                "verify the pinned frontend runtime and adapter, then retry",
            )]
        }
        SourceToIrError::Frontend(frontend) => frontend.diagnostics().to_vec(),
        SourceToIrError::Rejected(diagnostics) => diagnostics.clone(),
    };
    CommandFailure { kind: CommandFailureKind::Source, diagnostics }
}

fn preparation_failure(diagnostic: Diagnostic) -> CommandFailure {
    failure(CommandFailureKind::Preparation, diagnostic)
}

fn execution_failure(diagnostic: Diagnostic) -> CommandFailure {
    failure(CommandFailureKind::Execution, diagnostic)
}

fn runtime_failure(diagnostic: Diagnostic) -> CommandFailure {
    if diagnostic.code() == "ZRYNA-R3007" {
        failure(CommandFailureKind::Cleanup, diagnostic)
    } else {
        execution_failure(diagnostic)
    }
}

fn native_runtime_failure(diagnostic: Diagnostic) -> CommandFailure {
    if matches!(diagnostic.code(), "ZRYNA-N4009" | "ZRYNA-N4016") {
        failure(CommandFailureKind::Cleanup, diagnostic)
    } else {
        execution_failure(diagnostic)
    }
}

fn native_preparation_failure(diagnostics: Vec<Diagnostic>) -> CommandFailure {
    let kind = if diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-N4016") {
        CommandFailureKind::Cleanup
    } else {
        CommandFailureKind::Preparation
    };
    CommandFailure { kind, diagnostics }
}

fn invariant_failure(_error: serde_json::Error) -> CommandFailure {
    request_error(
        "ZRYNA-C1011",
        "runtime harness serialization failed",
        "report this compiler invariant failure",
    )
}

fn transaction_error(message: &'static str) -> CommandFailure {
    request_error(
        "ZRYNA-C1009",
        message,
        "use a writable real output filesystem and a fresh artifact stem",
    )
    .with_kind(CommandFailureKind::Preparation)
}

fn cleanup_failure(message: &'static str) -> CommandFailure {
    request_error(
        "ZRYNA-C1012",
        message,
        "inspect and remove only the exact reported transaction directory before retrying",
    )
    .with_kind(CommandFailureKind::Cleanup)
}

fn entrypoint_error(message: &'static str) -> CommandFailure {
    request_error(
        "ZRYNA-C1003",
        message,
        "use one existing portable workspace-relative .zry file without links",
    )
}

fn request_error(
    code: &'static str,
    message: &'static str,
    guidance: &'static str,
) -> CommandFailure {
    failure(CommandFailureKind::Request, Diagnostic::error(code, None, message, guidance))
}

fn failure(kind: CommandFailureKind, diagnostic: Diagnostic) -> CommandFailure {
    CommandFailure { kind, diagnostics: vec![diagnostic] }
}

trait FailureKindExt {
    fn with_kind(self, kind: CommandFailureKind) -> Self;
}

impl FailureKindExt for CommandFailure {
    fn with_kind(mut self, kind: CommandFailureKind) -> Self {
        self.kind = kind;
        self
    }
}

#[cfg(windows)]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use zryna_diagnostics::Diagnostic;
    use zryna_frontend::{VerifiedFrontendProvider, WorkerError, syntax_v2};
    use zryna_source::{SourceFileInput, SourceMap};

    #[cfg(unix)]
    use super::read_entrypoint_with_after_read;
    use super::{
        BuildRequest, CommandFailure, CommandFailureKind, CommandKind, ManifestTarget,
        TargetSelection, Transaction, compile_selected, native_preparation_failure,
        native_runtime_failure, runtime_failure, validate_request,
    };
    use crate::ArtifactOutputRoot;

    static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

    struct TemporaryRoot {
        path: PathBuf,
    }

    impl TemporaryRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir()
                .join(format!("zryna-pipeline-{}-{label}-{sequence}", std::process::id()));
            fs::create_dir(&path).expect("unique test root must be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn output(&self) -> ArtifactOutputRoot {
            ArtifactOutputRoot::prepare_for_workspace(&self.path)
                .expect("test output capability must be prepared")
        }

        fn request(&self, entrypoint: impl Into<String>) -> BuildRequest {
            BuildRequest {
                workspace_root: self.path.clone(),
                entrypoint: entrypoint.into(),
                artifact_stem: "artifact".to_owned(),
                targets: TargetSelection::JavaScript,
                node_runtime: self.path.join("node"),
            }
        }
    }

    impl Drop for TemporaryRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn rejected_request(result: Result<super::ValidatedRequest, CommandFailure>) -> CommandFailure {
        match result {
            Ok(_) => panic!("request must be rejected"),
            Err(failure) => failure,
        }
    }

    struct CountingFrontend {
        calls: AtomicUsize,
    }

    impl VerifiedFrontendProvider for CountingFrontend {
        fn analyze_verified(
            &self,
            sources: &SourceMap,
        ) -> Result<syntax_v2::ProjectSyntaxSnapshot, WorkerError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let raw = syntax_v2::decode_snapshot(include_bytes!(
                "../../../tests/fixtures/typescript-adapter-v2-result.json"
            ))
            .expect("checked protocol fixture must decode");
            Ok(syntax_v2::verify_snapshot(raw, sources)
                .expect("checked protocol fixture must bind to the authoritative source map"))
        }
    }

    #[test]
    fn all_analyzes_once_and_emits_every_target_from_one_program() {
        let source = include_str!("../../../examples/universal/add.zry");
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "examples/universal/add.zry".to_owned(),
            text: source.to_owned(),
        }])
        .expect("fixture source map");
        let frontend = CountingFrontend { calls: AtomicUsize::new(0) };

        let (compiled, artifacts) = compile_selected(&frontend, &sources, TargetSelection::All)
            .expect("all targets must prepare from one supported source");

        assert_eq!(frontend.calls.load(Ordering::SeqCst), 1);
        assert_eq!(compiled.program().functions().count(), 1);
        assert!(artifacts.javascript.is_some());
        assert!(artifacts.webassembly.is_some());
        assert!(artifacts.native_object.is_some());
        assert_eq!(
            TargetSelection::All.ordered(),
            vec![ManifestTarget::JavaScript, ManifestTarget::WebAssembly, ManifestTarget::Native]
        );
    }

    #[test]
    fn request_rejects_traversal_and_absolute_entrypoints() {
        let root = TemporaryRoot::new("request-paths");
        let source_directory = root.path().join("source");
        fs::create_dir(&source_directory).expect("source directory must be created");
        let source = source_directory.join("main.zry");
        fs::write(&source, "export function value(): i32 { return 1; }\n")
            .expect("source must be written");

        for entrypoint in [
            "../main.zry".to_owned(),
            "source//main.zry".to_owned(),
            "source/./main.zry".to_owned(),
            "source/main.zry/".to_owned(),
            source.to_string_lossy().into_owned(),
        ] {
            let failure =
                rejected_request(validate_request(&root.request(entrypoint), CommandKind::Build));
            assert_eq!(failure.kind(), CommandFailureKind::Request);
            assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-C1003");
        }
    }

    #[cfg(unix)]
    #[test]
    fn request_rejects_linked_source_components() {
        use std::os::unix::fs::symlink;

        let root = TemporaryRoot::new("request-links");
        let source_directory = root.path().join("source");
        fs::create_dir(&source_directory).expect("source directory must be created");
        fs::write(
            source_directory.join("main.zry"),
            "export function value(): i32 { return 1; }\n",
        )
        .expect("source must be written");
        symlink("main.zry", source_directory.join("linked.zry"))
            .expect("source link must be created");
        symlink(&source_directory, root.path().join("linked-source"))
            .expect("directory link must be created");

        for entrypoint in ["source/linked.zry", "linked-source/main.zry"] {
            let failure =
                rejected_request(validate_request(&root.request(entrypoint), CommandKind::Build));
            assert_eq!(failure.kind(), CommandFailureKind::Request);
            assert!(matches!(failure.diagnostics()[0].code(), "ZRYNA-C1002" | "ZRYNA-C1003"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn entrypoint_read_rejects_same_length_final_replacement() {
        let root = TemporaryRoot::new("source-replacement");
        let source = root.path().join("main.zry");
        let replacement = root.path().join("replacement.zry");
        fs::write(&source, b"original-source\n").expect("source must be written");
        fs::write(&replacement, b"replaced-source\n").expect("replacement must be written");
        assert_eq!(
            fs::metadata(&source).expect("source metadata").len(),
            fs::metadata(&replacement).expect("replacement metadata").len()
        );

        let failure = read_entrypoint_with_after_read(&source, || {
            fs::rename(&replacement, &source).expect("replacement must be installed");
        })
        .expect_err("replaced source identity must be rejected");

        assert_eq!(failure.kind(), CommandFailureKind::Request);
        assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-C1003");
    }

    #[cfg(unix)]
    #[test]
    fn output_root_replacement_fails_closed_and_stage_remains_cleanable() {
        use std::os::unix::fs::symlink;

        let root = TemporaryRoot::new("output-replacement");
        let output = root.output();
        let mut transaction = Transaction::create(&output).expect("transaction must be created");
        let stage = transaction.path().to_path_buf();
        transaction
            .write_artifact(
                ManifestTarget::JavaScript,
                "ecmascript-module",
                "artifact",
                "mjs",
                b"export {};\n",
            )
            .expect("staged artifact must be written");
        let displaced = root.path().join("displaced-output");
        let replacement = root.path().join("replacement-output");
        fs::create_dir(&replacement).expect("replacement directory must be created");
        fs::rename(output.path(), &displaced).expect("output root must be displaced");
        symlink(&replacement, output.path()).expect("linked output replacement must be installed");
        let final_bundle = output.path().join("artifact.build");

        let failure = transaction
            .commit(&output, &final_bundle)
            .expect_err("linked output replacement must fail closed");
        assert_eq!(failure.kind(), CommandFailureKind::Preparation);
        assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-D2002");
        assert!(!replacement.join("artifact.build").exists());

        fs::remove_file(output.path()).expect("replacement link must be removed");
        fs::rename(&displaced, output.path()).expect("validated output root must be restored");
        transaction.cleanup(&output).expect("restored stage must be cleanable");
        assert!(!stage.exists());
        assert!(!final_bundle.exists());
    }

    #[cfg(unix)]
    #[test]
    fn transaction_stage_replacement_cannot_redirect_writes() {
        use std::os::unix::fs::symlink;

        let root = TemporaryRoot::new("stage-replacement");
        let output = root.output();
        let mut transaction = Transaction::create(&output).expect("transaction must be created");
        let stage = transaction.path().to_path_buf();
        let displaced = output.path().join("displaced-stage");
        let attacker = root.path().join("attacker-output");
        fs::create_dir(&attacker).expect("attacker directory must be created");
        fs::rename(&stage, &displaced).expect("private stage must be displaced");
        symlink(&attacker, &stage).expect("replacement stage link must be installed");

        let failure = transaction
            .write_artifact(
                ManifestTarget::JavaScript,
                "ecmascript-module",
                "artifact",
                "mjs",
                b"export {};\n",
            )
            .expect_err("replaced private stage must fail closed");
        assert_eq!(failure.kind(), CommandFailureKind::Preparation);
        assert!(fs::read_dir(&attacker).expect("attacker directory listing").next().is_none());

        fs::remove_file(&stage).expect("replacement link must be removed");
        fs::rename(&displaced, &stage).expect("private stage must be restored");
        transaction.cleanup(&output).expect("restored transaction must be removed");
        assert!(!stage.exists());
    }

    #[test]
    fn real_output_root_replacement_is_detected_or_prevented() {
        let root = TemporaryRoot::new("real-output-replacement");
        let output = root.output();
        let mut transaction = Transaction::create(&output).expect("transaction must be created");
        let stage = transaction.path().to_path_buf();
        transaction
            .write_artifact(
                ManifestTarget::JavaScript,
                "ecmascript-module",
                "artifact",
                "mjs",
                b"export {};\n",
            )
            .expect("staged artifact must be written");
        let displaced = root.path().join("displaced-real-output");
        if let Err(error) = fs::rename(output.path(), &displaced) {
            assert!(
                cfg!(windows)
                    && (error.kind() == std::io::ErrorKind::PermissionDenied
                        || matches!(error.raw_os_error(), Some(5 | 32))),
                "unexpected output replacement failure: {error}"
            );
            let final_bundle = output.path().join("artifact.build");
            assert!(!final_bundle.exists());
            transaction.cleanup(&output).expect("protected stage must be cleanable");
            assert!(!stage.exists());
            return;
        }
        fs::create_dir(output.path()).expect("real replacement output must be installed");
        let final_bundle = output.path().join("artifact.build");

        let failure = transaction
            .commit(&output, &final_bundle)
            .expect_err("displaced transaction must not commit through a replacement root");
        assert_eq!(failure.kind(), CommandFailureKind::Preparation);
        assert!(!final_bundle.exists());

        fs::remove_dir(output.path()).expect("empty replacement output must be removed");
        fs::rename(&displaced, output.path()).expect("original output root must be restored");
        transaction.cleanup(&output).expect("restored stage must be cleanable");
        assert!(!stage.exists());
        assert!(!final_bundle.exists());
    }

    #[test]
    fn post_transaction_failure_cleans_stage_without_advertising_bundle() {
        let root = TemporaryRoot::new("transaction-failure");
        let output = root.output();
        let mut transaction = Transaction::create(&output).expect("transaction must be created");
        let stage = transaction.path().to_path_buf();
        let final_bundle = output.path().join("artifact.build");
        transaction
            .write_artifact(
                ManifestTarget::JavaScript,
                "ecmascript-module",
                "artifact",
                "mjs",
                b"export {};\n",
            )
            .expect("staged artifact must be written");
        fs::write(stage.join(super::MANIFEST_NAME), b"occupied")
            .expect("manifest collision must be installed");

        let failure = transaction
            .write_manifest(b"{}\n")
            .expect_err("post-stage manifest failure must be reported");
        assert_eq!(failure.kind(), CommandFailureKind::Preparation);
        transaction.cleanup(&output).expect("failed transaction must be removed");
        assert!(!stage.exists());
        assert!(!final_bundle.exists());
        assert!(
            fs::read_dir(output.path()).expect("output root must remain readable").next().is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn destination_link_collision_preserves_prior_bytes() {
        use std::os::unix::fs::symlink;

        let root = TemporaryRoot::new("destination-link");
        let output = root.output();
        let sentinel = root.path().join("sentinel");
        fs::write(&sentinel, b"prior-bytes").expect("sentinel must be written");
        let final_bundle = output.path().join("artifact.build");
        symlink(&sentinel, &final_bundle).expect("destination link must be created");
        let mut transaction = Transaction::create(&output).expect("transaction must be created");
        let stage = transaction.path().to_path_buf();

        let failure = transaction
            .commit(&output, &final_bundle)
            .expect_err("destination link must be treated as a collision");
        assert_eq!(failure.kind(), CommandFailureKind::Preparation);
        assert_eq!(fs::read(&sentinel).expect("sentinel must remain"), b"prior-bytes");
        assert!(
            fs::symlink_metadata(&final_bundle)
                .expect("destination link must remain")
                .file_type()
                .is_symlink()
        );
        transaction.cleanup(&output).expect("failed transaction must be removed");
        assert!(!stage.exists());
    }

    #[test]
    fn exit_categories_and_cleanup_diagnostics_are_stable() {
        for (kind, expected) in [
            (CommandFailureKind::Architecture, 1),
            (CommandFailureKind::Request, 2),
            (CommandFailureKind::Source, 3),
            (CommandFailureKind::Preparation, 4),
            (CommandFailureKind::Execution, 5),
            (CommandFailureKind::Cleanup, 6),
        ] {
            assert_eq!(kind.exit_code(), expected);
        }

        let runtime_cleanup = runtime_failure(Diagnostic::error(
            "ZRYNA-R3007",
            None,
            "runtime cleanup failed",
            "stop the process tree",
        ));
        assert_eq!(runtime_cleanup.kind(), CommandFailureKind::Cleanup);
        assert_eq!(runtime_cleanup.kind().exit_code(), 6);
        let native_cleanup = native_runtime_failure(Diagnostic::error(
            "ZRYNA-N4009",
            None,
            "native cleanup failed",
            "stop the process group",
        ));
        assert_eq!(native_cleanup.kind(), CommandFailureKind::Cleanup);
        assert_eq!(native_cleanup.kind().exit_code(), 6);
        let native_stage_cleanup = native_runtime_failure(Diagnostic::error(
            "ZRYNA-N4016",
            None,
            "native stage cleanup failed",
            "restore the private stage permissions",
        ));
        assert_eq!(native_stage_cleanup.kind(), CommandFailureKind::Cleanup);
        assert_eq!(native_stage_cleanup.kind().exit_code(), 6);
        let native_preparation_cleanup = native_preparation_failure(vec![Diagnostic::warning(
            "ZRYNA-N4016",
            None,
            "native preparation stage cleanup failed",
            "remove the private stage",
        )]);
        assert_eq!(native_preparation_cleanup.kind(), CommandFailureKind::Cleanup);
        assert_eq!(native_preparation_cleanup.kind().exit_code(), 6);
        let execution = runtime_failure(Diagnostic::error(
            "ZRYNA-R3005",
            None,
            "runtime execution failed",
            "retry",
        ));
        assert_eq!(execution.kind(), CommandFailureKind::Execution);
        assert_eq!(execution.kind().exit_code(), 5);
    }
}
