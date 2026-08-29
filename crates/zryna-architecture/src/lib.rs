//! Canonical fail-closed Zryna repository architecture engine.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use same_file::Handle;
use serde::{Deserialize, Serialize};
use zryna_diagnostics::Diagnostic;

const CONTRACT_FILE: &str = "zryna.workspace.json";
const CONTRACT_VERSION: u32 = 1;
const CONTRACT_PROFILE: &str = "zryna-compiler-workspace-v1";
const MAX_CONTRACT_BYTES: u64 = 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_SCANNED_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SCAN_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SCAN_ENTRIES: usize = 50_000;
const MAX_SCAN_DEPTH: usize = 32;
const MAX_SCAN_DIAGNOSTICS: usize = 256;
const MAX_REGISTERED_COMPONENTS: usize = 256;

const ALLOWED_ROOT_ENTRIES: &[&str] = &[
    ".cargo",
    ".git",
    ".github",
    ".gitattributes",
    ".gitignore",
    ".zryna",
    "CODE_OF_CONDUCT.md",
    "CONTRIBUTING.md",
    "Cargo.lock",
    "Cargo.toml",
    "LICENSE",
    "NOTICE",
    "README.md",
    "SECURITY.md",
    "adapters",
    "apps",
    "crates",
    "docs",
    "editors",
    "examples",
    "node_modules",
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "runtime",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "schemas",
    "scripts",
    "spec",
    "target",
    "tests",
    "toolchains",
    "zryna.workspace.json",
];

/// Authoritative workspace contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceContract {
    /// Editor schema reference.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Contract version.
    pub version: u32,
    /// Exact architecture profile.
    pub profile: String,
    /// Registered Rust components.
    pub members: Vec<MemberContract>,
    /// Registered replaceable frontend adapters.
    pub adapters: Vec<AdapterContract>,
    /// Exclusive generated output roots.
    pub outputs: Vec<String>,
}

/// One registered Rust component.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MemberContract {
    /// Cargo package and architecture identifier.
    pub id: String,
    /// Workspace-relative root.
    pub root: String,
    /// Architecture layer.
    pub kind: MemberKind,
    /// Exact allowed internal direct dependencies.
    pub dependencies: Vec<String>,
    /// Allowed immediate files and directories, excluding generated vendor/output directories.
    pub allowed_entries: Vec<String>,
}

/// Architecture layer classification.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemberKind {
    /// Base contracts and validation.
    Foundation,
    /// Replaceable source acquisition.
    Frontend,
    /// Target-neutral compiler logic.
    Compiler,
    /// Target-specific output generation.
    Backend,
    /// Pipeline orchestration.
    Orchestrator,
    /// User-facing entrypoint.
    Application,
}

/// External frontend adapter declaration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AdapterContract {
    /// Adapter identity.
    pub id: String,
    /// Workspace-relative root.
    pub root: String,
    /// ZRYNA-owned protocol version.
    pub protocol_version: u32,
    /// Exact external toolchain package.
    pub toolchain: String,
    /// Allowed immediate files and directories, excluding generated vendor/output directories.
    pub allowed_entries: Vec<String>,
}

/// Complete result of one architecture check.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    /// Stable diagnostics in deterministic order.
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    /// Whether the architecture is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Default)]
struct ValidationDiagnostics {
    values: Vec<Diagnostic>,
    halted: bool,
}

impl ValidationDiagnostics {
    fn push(&mut self, diagnostic: Diagnostic) {
        if self.halted {
            return;
        }
        if self.values.len().saturating_add(1) >= MAX_SCAN_DIAGNOSTICS {
            self.halt(architecture_error(
                "ZRYNA-A1204",
                None,
                "architecture validation exceeded its deterministic diagnostic budget",
                "reduce invalid controlled input; incomplete validation never passes",
            ));
            return;
        }
        self.values.push(diagnostic);
    }

    fn halt(&mut self, diagnostic: Diagnostic) {
        if self.halted {
            return;
        }
        if self.values.len() >= MAX_SCAN_DIAGNOSTICS {
            self.values.truncate(MAX_SCAN_DIAGNOSTICS.saturating_sub(1));
        }
        self.values.push(diagnostic);
        self.halted = true;
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    const fn is_halted(&self) -> bool {
        self.halted
    }

    fn into_vec(self) -> Vec<Diagnostic> {
        self.values
    }
}

/// Validates a complete Zryna workspace and fails closed on incomplete inspection.
#[must_use]
pub fn validate_workspace(root: &Path) -> ValidationReport {
    let mut diagnostics = ValidationDiagnostics::default();
    let canonical_root = match canonical_workspace_root(root) {
        Ok(value) => value,
        Err(diagnostic) => return ValidationReport { diagnostics: vec![diagnostic] },
    };
    let (contract, contract_source) = match load_contract(&canonical_root) {
        Ok(value) => value,
        Err(diagnostic) => return ValidationReport { diagnostics: vec![diagnostic] },
    };

    validate_contract_identity(&contract, &mut diagnostics);
    if !diagnostics.is_empty() {
        return validation_report(diagnostics);
    }
    validate_contract_paths(&contract, &mut diagnostics);
    if !diagnostics.is_empty() {
        return validation_report(diagnostics);
    }

    let scan_started_with = diagnostics.len();
    let scan_completed =
        validate_bounded_filesystem(&canonical_root, &contract, &contract_source, &mut diagnostics);
    if !scan_completed || diagnostics.len() != scan_started_with {
        return validation_report(diagnostics);
    }

    validate_root_entries(&canonical_root, &mut diagnostics);
    if diagnostics.is_halted() {
        return validation_report(diagnostics);
    }
    validate_paths(&canonical_root, &contract, &mut diagnostics);
    if diagnostics.is_halted() {
        return validation_report(diagnostics);
    }
    validate_component_entries(&canonical_root, &contract, &mut diagnostics);
    if diagnostics.is_halted() {
        return validation_report(diagnostics);
    }
    validate_members(&canonical_root, &contract, &mut diagnostics);
    if diagnostics.is_halted() {
        return validation_report(diagnostics);
    }
    validate_adapters(&canonical_root, &contract, &mut diagnostics);
    if diagnostics.is_halted() {
        return validation_report(diagnostics);
    }
    validate_dependency_graph(&contract, &mut diagnostics);
    if diagnostics.is_halted() {
        return validation_report(diagnostics);
    }
    validate_contract_unchanged(&canonical_root, &contract_source, &mut diagnostics);

    validation_report(diagnostics)
}

fn validation_report(diagnostics: ValidationDiagnostics) -> ValidationReport {
    let mut values = diagnostics.into_vec();
    values.sort_by(|left, right| {
        (&left.code, &left.path, &left.message).cmp(&(&right.code, &right.path, &right.message))
    });
    ValidationReport { diagnostics: values }
}

fn canonical_workspace_root(root: &Path) -> Result<PathBuf, Diagnostic> {
    let metadata = fs::symlink_metadata(root).map_err(|error| {
        architecture_error(
            "ZRYNA-A1001",
            Some(root),
            format!("workspace root is unavailable: {error}"),
            "select an existing regular directory containing zryna.workspace.json",
        )
    })?;
    if metadata_is_link_or_reparse(&metadata) || !metadata.is_dir() {
        return Err(architecture_error(
            "ZRYNA-A1201",
            Some(root),
            "workspace root must be a real, non-symlink directory",
            "open the canonical project directory directly",
        ));
    }
    fs::canonicalize(root).map_err(|error| {
        architecture_error(
            "ZRYNA-A1001",
            Some(root),
            format!("workspace root cannot be canonicalized: {error}"),
            "fix workspace permissions and path components",
        )
    })
}

fn read_bounded_utf8(
    path: &Path,
    diagnostic_path: Option<&Path>,
    max_bytes: u64,
    unavailable_code: &str,
    unavailable_guidance: &str,
) -> Result<(String, u64), Diagnostic> {
    let policy = ControlledReadPolicy {
        diagnostic_path,
        max_bytes,
        unavailable_code,
        unavailable_guidance,
        expected_size: None,
    };
    read_bounded_utf8_with_hooks(path, policy, || {}, || {})
}

fn read_bounded_utf8_with_expected_size(
    path: &Path,
    diagnostic_path: Option<&Path>,
    max_bytes: u64,
    unavailable_code: &str,
    unavailable_guidance: &str,
    expected_size: u64,
) -> Result<(String, u64), Diagnostic> {
    let policy = ControlledReadPolicy {
        diagnostic_path,
        max_bytes,
        unavailable_code,
        unavailable_guidance,
        expected_size: Some(expected_size),
    };
    read_bounded_utf8_with_hooks(path, policy, || {}, || {})
}

#[derive(Clone, Copy)]
struct ControlledReadPolicy<'a> {
    diagnostic_path: Option<&'a Path>,
    max_bytes: u64,
    unavailable_code: &'a str,
    unavailable_guidance: &'a str,
    expected_size: Option<u64>,
}

fn read_bounded_utf8_with_hooks<BeforeOpen, AfterRead>(
    path: &Path,
    policy: ControlledReadPolicy<'_>,
    before_open: BeforeOpen,
    after_read: AfterRead,
) -> Result<(String, u64), Diagnostic>
where
    BeforeOpen: FnOnce(),
    AfterRead: FnOnce(),
{
    let (mut handle, opened) = open_controlled_file(
        path,
        policy.diagnostic_path,
        policy.max_bytes,
        policy.unavailable_code,
        policy.unavailable_guidance,
    )?;
    if policy.expected_size.is_some_and(|expected| expected != opened.len()) {
        return Err(architecture_error(
            "ZRYNA-A1203",
            policy.diagnostic_path,
            "controlled file size changed before its bounded read",
            "stop concurrent replacement and retry architecture validation",
        ));
    }
    before_open();
    validate_current_controlled_path(path, policy.diagnostic_path, &handle, &opened)?;

    let mut bytes = Vec::new();
    handle.as_file_mut().take(policy.max_bytes.saturating_add(1)).read_to_end(&mut bytes).map_err(
        |error| {
            architecture_error(
                "ZRYNA-A1203",
                policy.diagnostic_path,
                format!("controlled file could not be read completely: {error}"),
                "restore stable read access and retry",
            )
        },
    )?;
    if u64::try_from(bytes.len()).map_or(true, |length| length > policy.max_bytes) {
        return Err(architecture_error(
            "ZRYNA-A1204",
            policy.diagnostic_path,
            format!("controlled file exceeds its {}-byte safety limit", policy.max_bytes),
            "reduce the file before architecture validation",
        ));
    }

    after_read();
    revalidate_controlled_file(path, policy.diagnostic_path, &handle, &opened)?;
    let length = u64::try_from(bytes.len()).map_err(|_| {
        architecture_error(
            "ZRYNA-A1204",
            policy.diagnostic_path,
            "controlled file length cannot be represented safely",
            "reduce the file before architecture validation",
        )
    })?;
    String::from_utf8(bytes).map(|source| (source, length)).map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            policy.diagnostic_path,
            format!("controlled file is not valid UTF-8: {error}"),
            "save controlled source and manifest files as UTF-8",
        )
    })
}

fn open_controlled_file(
    path: &Path,
    diagnostic_path: Option<&Path>,
    max_bytes: u64,
    unavailable_code: &str,
    unavailable_guidance: &str,
) -> Result<(Handle, fs::Metadata), Diagnostic> {
    let link_metadata = fs::symlink_metadata(path).map_err(|error| {
        architecture_error(
            unavailable_code,
            diagnostic_path,
            format!("controlled file is unavailable: {error}"),
            unavailable_guidance,
        )
    })?;
    if metadata_is_link_or_reparse(&link_metadata) || !link_metadata.is_file() {
        return Err(architecture_error(
            "ZRYNA-A1201",
            diagnostic_path,
            "controlled input must be a regular, non-symlink file",
            "replace it with a regular in-workspace UTF-8 file",
        ));
    }
    let file = open_regular_no_follow(path).map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            format!("controlled file could not be opened safely: {error}"),
            "restore a stable readable regular file and retry",
        )
    })?;
    let handle = Handle::from_file(file).map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            format!("controlled file identity is unavailable: {error}"),
            "restore a stable readable regular file and retry",
        )
    })?;
    let opened = handle.as_file().metadata().map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            format!("opened file metadata is unavailable: {error}"),
            "restore a stable readable regular file and retry",
        )
    })?;
    if metadata_is_link_or_reparse(&opened) || !opened.is_file() {
        return Err(architecture_error(
            "ZRYNA-A1201",
            diagnostic_path,
            "opened controlled input is not a regular file",
            "replace it with a regular in-workspace UTF-8 file",
        ));
    }
    if opened.len() > max_bytes {
        return Err(architecture_error(
            "ZRYNA-A1204",
            diagnostic_path,
            format!("controlled file exceeds its {max_bytes}-byte safety limit"),
            "reduce the file before architecture validation",
        ));
    }
    validate_current_controlled_path(path, diagnostic_path, &handle, &opened)?;
    Ok((handle, opened))
}

fn validate_current_controlled_path(
    path: &Path,
    diagnostic_path: Option<&Path>,
    reference: &Handle,
    opened: &fs::Metadata,
) -> Result<(), Diagnostic> {
    let path_metadata = fs::symlink_metadata(path).map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            format!("controlled file path changed during inspection: {error}"),
            "stop concurrent replacement and retry architecture validation",
        )
    })?;
    if metadata_is_link_or_reparse(&path_metadata) || !path_metadata.is_file() {
        return Err(architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            "controlled file path became a link or non-file during inspection",
            "stop concurrent replacement and retry architecture validation",
        ));
    }
    let file = open_regular_no_follow(path).map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            format!("controlled file could not be opened safely: {error}"),
            "restore a stable readable regular file and retry",
        )
    })?;
    let current = Handle::from_file(file).map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            format!("controlled file identity is unavailable: {error}"),
            "restore a stable readable regular file and retry",
        )
    })?;
    let current_metadata = current.as_file().metadata().map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            format!("current file metadata is unavailable: {error}"),
            "restore a stable readable regular file and retry",
        )
    })?;
    if reference != &current || !same_file_state(opened, &current_metadata) {
        return Err(architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            "controlled file identity or state changed during inspection",
            "stop concurrent replacement and retry architecture validation",
        ));
    }
    Ok(())
}

fn revalidate_controlled_file(
    path: &Path,
    diagnostic_path: Option<&Path>,
    handle: &Handle,
    opened: &fs::Metadata,
) -> Result<(), Diagnostic> {
    let after = handle.as_file().metadata().map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            format!("controlled file could not be revalidated: {error}"),
            "restore a stable readable regular file and retry",
        )
    })?;
    if !same_file_state(opened, &after) {
        return Err(architecture_error(
            "ZRYNA-A1203",
            diagnostic_path,
            "controlled file changed during inspection",
            "stop concurrent modification and retry architecture validation",
        ));
    }
    validate_current_controlled_path(path, diagnostic_path, handle, opened)
}

#[cfg(unix)]
fn open_regular_no_follow(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    options.open(path)
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path) -> std::io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    let mut options = fs::OpenOptions::new();
    options.read(true).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).share_mode(FILE_SHARE_READ);
    options.open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_regular_no_follow(_path: &Path) -> std::io::Result<fs::File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "no fail-closed no-follow file strategy exists for this platform",
    ))
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

fn load_contract(root: &Path) -> Result<(WorkspaceContract, String), Diagnostic> {
    let path = root.join(CONTRACT_FILE);
    let (source, _) = read_bounded_utf8(
        &path,
        Some(Path::new(CONTRACT_FILE)),
        MAX_CONTRACT_BYTES,
        "ZRYNA-A1001",
        "restore the canonical zryna.workspace.json file",
    )?;
    let contract = serde_json::from_str(&source).map_err(|error| {
        architecture_error(
            "ZRYNA-A1001",
            Some(&path),
            format!("workspace contract is invalid: {error}"),
            "match schemas/zryna-workspace-v1.schema.json exactly; unknown fields are forbidden",
        )
    })?;
    Ok((contract, source))
}

fn validate_contract_unchanged(
    root: &Path,
    expected_source: &str,
    diagnostics: &mut ValidationDiagnostics,
) {
    let path = root.join(CONTRACT_FILE);
    match read_bounded_utf8(
        &path,
        Some(Path::new(CONTRACT_FILE)),
        MAX_CONTRACT_BYTES,
        "ZRYNA-A1001",
        "restore the canonical zryna.workspace.json file",
    ) {
        Ok((source, _)) if source == expected_source => {}
        Ok(_) => diagnostics.push(architecture_error(
            "ZRYNA-A1203",
            Some(Path::new(CONTRACT_FILE)),
            "workspace contract changed during architecture validation",
            "stop concurrent mutation and retry architecture validation",
        )),
        Err(diagnostic) => diagnostics.push(diagnostic),
    }
}

fn validate_contract_identity(
    contract: &WorkspaceContract,
    diagnostics: &mut ValidationDiagnostics,
) {
    if contract.version != CONTRACT_VERSION || contract.profile != CONTRACT_PROFILE {
        diagnostics.push(architecture_error(
            "ZRYNA-A1001",
            Some(Path::new(CONTRACT_FILE)),
            "workspace contract version or profile is unsupported",
            format!("use version {CONTRACT_VERSION} and profile {CONTRACT_PROFILE}"),
        ));
    }
    if contract.schema != "./schemas/zryna-workspace-v1.schema.json" {
        diagnostics.push(architecture_error(
            "ZRYNA-A1001",
            Some(Path::new(CONTRACT_FILE)),
            "workspace schema reference is not canonical",
            "set $schema to ./schemas/zryna-workspace-v1.schema.json",
        ));
    }
    if contract.members.is_empty() {
        diagnostics.push(architecture_error(
            "ZRYNA-A1001",
            Some(Path::new(CONTRACT_FILE)),
            "workspace contract must register at least one Rust member",
            "register the compiler components explicitly",
        ));
    }
    let component_count = contract.members.len().saturating_add(contract.adapters.len());
    if component_count > MAX_REGISTERED_COMPONENTS {
        diagnostics.push(architecture_error(
            "ZRYNA-A1001",
            Some(Path::new(CONTRACT_FILE)),
            format!(
                "workspace contract registers {component_count} components; the limit is {MAX_REGISTERED_COMPONENTS}"
            ),
            "split unrelated components into another workspace or reduce the registry",
        ));
    }
    let expected_outputs = BTreeSet::from([".zryna/cache", ".zryna/out", "target"]);
    let actual_outputs: BTreeSet<&str> = contract.outputs.iter().map(String::as_str).collect();
    if actual_outputs != expected_outputs || contract.outputs.len() != expected_outputs.len() {
        diagnostics.push(architecture_error(
            "ZRYNA-A1001",
            Some(Path::new(CONTRACT_FILE)),
            "generated output roots differ from the strict architecture profile",
            "declare target, .zryna/cache, and .zryna/out exactly once",
        ));
    }
}

fn validate_contract_paths(contract: &WorkspaceContract, diagnostics: &mut ValidationDiagnostics) {
    let mut identities = BTreeSet::new();
    let mut roots = BTreeSet::new();
    for (id, component_root) in contract
        .members
        .iter()
        .map(|member| (&member.id, &member.root))
        .chain(contract.adapters.iter().map(|adapter| (&adapter.id, &adapter.root)))
    {
        if diagnostics.is_halted() {
            return;
        }
        if !valid_id(id) || !identities.insert(id.to_ascii_lowercase()) {
            diagnostics.push(architecture_error(
                "ZRYNA-A1003",
                Some(Path::new(CONTRACT_FILE)),
                format!("component id '{id}' is invalid or collides case-insensitively"),
                "use one unique lowercase kebab-case identifier",
            ));
        }
        if !safe_relative_path(component_root) || !roots.insert(component_root.to_ascii_lowercase())
        {
            diagnostics.push(architecture_error(
                "ZRYNA-A1003",
                Some(Path::new(component_root)),
                format!("component root '{component_root}' is unsafe or duplicated"),
                "use a unique normalized workspace-relative path without traversal or backslashes",
            ));
        }
    }
}

fn validate_root_entries(root: &Path, diagnostics: &mut ValidationDiagnostics) {
    let allowed: BTreeSet<&str> = ALLOWED_ROOT_ENTRIES.iter().copied().collect();
    let Some(entries) = sorted_directory_entries(root, root, diagnostics) else {
        return;
    };
    for entry in entries {
        if diagnostics.is_halted() {
            return;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            diagnostics.push(architecture_error(
                "ZRYNA-A1203",
                Some(&entry.path()),
                "workspace entry name is not valid UTF-8",
                "rename the entry with a portable UTF-8 name",
            ));
            continue;
        };
        if !allowed.contains(name) {
            diagnostics.push(architecture_error(
                "ZRYNA-A1004",
                Some(Path::new(name)),
                format!("root entry '{name}' is not part of the Zryna architecture"),
                "move the content into a registered component or redefine the contract deliberately",
            ));
        }
    }
}

fn sorted_directory_entries(
    path: &Path,
    diagnostic_path: &Path,
    diagnostics: &mut ValidationDiagnostics,
) -> Option<Vec<fs::DirEntry>> {
    let entries = match fs::read_dir(path) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(architecture_error(
                "ZRYNA-A1205",
                Some(diagnostic_path),
                format!("directory scan failed: {error}"),
                "restore read access; incomplete scans never pass",
            ));
            return None;
        }
    };
    let mut sorted = Vec::new();
    for entry in entries {
        if diagnostics.is_halted() {
            return None;
        }
        if sorted.len() >= MAX_SCAN_ENTRIES {
            diagnostics.halt(architecture_error(
                "ZRYNA-A1204",
                Some(diagnostic_path),
                "directory validation exceeded its deterministic entry budget",
                "reduce controlled entries; incomplete validation never passes",
            ));
            return None;
        }
        match entry {
            Ok(value) => sorted.push(value),
            Err(error) => diagnostics.push(architecture_error(
                "ZRYNA-A1205",
                Some(diagnostic_path),
                format!("directory entry could not be inspected: {error}"),
                "restore directory consistency and retry",
            )),
        }
    }
    sorted.sort_by_key(fs::DirEntry::file_name);
    Some(sorted)
}

fn validate_paths(
    root: &Path,
    contract: &WorkspaceContract,
    diagnostics: &mut ValidationDiagnostics,
) {
    for member_root in contract
        .members
        .iter()
        .map(|member| &member.root)
        .chain(contract.adapters.iter().map(|adapter| &adapter.root))
    {
        if diagnostics.is_halted() {
            return;
        }
        let joined = root.join(member_root);
        match fs::canonicalize(&joined) {
            Ok(canonical) if canonical.starts_with(root) => {}
            Ok(_) => diagnostics.push(architecture_error(
                "ZRYNA-A1202",
                Some(Path::new(member_root)),
                "component root escapes the canonical workspace",
                "place the component inside the workspace without symlink indirection",
            )),
            Err(error) => diagnostics.push(architecture_error(
                "ZRYNA-A1005",
                Some(Path::new(member_root)),
                format!("registered component root is unavailable: {error}"),
                "create it with the canonical project planner",
            )),
        }
    }
}

fn validate_component_entries(
    root: &Path,
    contract: &WorkspaceContract,
    diagnostics: &mut ValidationDiagnostics,
) {
    for (component_root, allowed_entries, allows_node_modules) in
        contract.members.iter().map(|member| (&member.root, &member.allowed_entries, false)).chain(
            contract.adapters.iter().map(|adapter| (&adapter.root, &adapter.allowed_entries, true)),
        )
    {
        if diagnostics.is_halted() {
            return;
        }
        let mut allowed = BTreeSet::new();
        for entry in allowed_entries {
            if diagnostics.is_halted() {
                return;
            }
            if !valid_component_entry(entry) || !allowed.insert(entry.as_str()) {
                diagnostics.push(architecture_error(
                    "ZRYNA-A1003",
                    Some(Path::new(component_root)),
                    format!("allowed component entry '{entry}' is invalid or duplicated"),
                    "use each portable immediate file or directory name exactly once",
                ));
            }
        }
        let path = root.join(component_root);
        let Some(entries) = sorted_directory_entries(&path, Path::new(component_root), diagnostics)
        else {
            continue;
        };
        for entry in entries {
            if diagnostics.is_halted() {
                return;
            }
            let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                diagnostics.push(architecture_error(
                    "ZRYNA-A1203",
                    entry.path().strip_prefix(root).ok(),
                    "component entry name is not valid UTF-8",
                    "rename the entry with a portable UTF-8 name",
                ));
                continue;
            };
            if allows_node_modules && name == "node_modules" {
                match entry.file_type() {
                    Ok(file_type) if file_type.is_dir() => continue,
                    Ok(_) => {}
                    Err(error) => {
                        diagnostics.push(architecture_error(
                            "ZRYNA-A1205",
                            entry.path().strip_prefix(root).ok(),
                            format!("component entry type could not be inspected: {error}"),
                            "restore directory consistency and retry",
                        ));
                        continue;
                    }
                }
            }
            if !allowed.contains(name.as_str()) {
                diagnostics.push(architecture_error(
                    "ZRYNA-A1007",
                    entry.path().strip_prefix(root).ok(),
                    format!("'{name}' is outside the registered component layout"),
                    "move it into an allowed entry or update zryna.workspace.json deliberately",
                ));
            }
        }
    }
}

fn validate_members(
    root: &Path,
    contract: &WorkspaceContract,
    diagnostics: &mut ValidationDiagnostics,
) {
    let declared: BTreeSet<&str> =
        contract.members.iter().map(|member| member.id.as_str()).collect();
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = read_toml(&root_manifest_path, diagnostics);
    if let Some(manifest) = root_manifest {
        let cargo_members: BTreeSet<String> = manifest
            .get("workspace")
            .and_then(|workspace| workspace.get("members"))
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(toml::Value::as_str)
            .map(ToOwned::to_owned)
            .collect();
        let contract_members: BTreeSet<String> =
            contract.members.iter().map(|member| member.root.clone()).collect();
        if cargo_members != contract_members {
            diagnostics.push(architecture_error(
                "ZRYNA-A1005",
                Some(Path::new("Cargo.toml")),
                "Cargo workspace members differ from zryna.workspace.json",
                "register every Rust member exactly once in both authoritative manifests",
            ));
        }
    }
    if diagnostics.is_halted() {
        return;
    }

    for member in &contract.members {
        if diagnostics.is_halted() {
            return;
        }
        let member_root = root.join(&member.root);
        let manifest_path = member_root.join("Cargo.toml");
        let readme_path = member_root.join("README.md");
        if !readme_path.is_file() {
            diagnostics.push(architecture_error(
                "ZRYNA-A1006",
                Some(Path::new(&member.root)),
                "registered member is missing README.md",
                "document the component authority and dependency boundary",
            ));
        }
        let expected_entry = if member.kind == MemberKind::Application {
            member_root.join("src/main.rs")
        } else {
            member_root.join("src/lib.rs")
        };
        if !expected_entry.is_file() {
            diagnostics.push(architecture_error(
                "ZRYNA-A1006",
                Some(Path::new(&member.root)),
                "registered member has the wrong Rust entrypoint",
                "applications require src/main.rs; all other members require src/lib.rs",
            ));
        }
        if diagnostics.is_halted() {
            return;
        }
        let Some(manifest) = read_toml(&manifest_path, diagnostics) else {
            continue;
        };
        let package_name = manifest
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(toml::Value::as_str);
        if package_name != Some(member.id.as_str()) {
            diagnostics.push(architecture_error(
                "ZRYNA-A1006",
                Some(&manifest_path),
                format!("Cargo package name does not match registered id '{}'", member.id),
                "make the directory, member id, and Cargo package name identical",
            ));
        }
        let actual_dependencies = internal_dependencies(&manifest, &declared);
        let expected_dependencies: BTreeSet<&str> =
            member.dependencies.iter().map(String::as_str).collect();
        if actual_dependencies != expected_dependencies {
            diagnostics.push(architecture_error(
                "ZRYNA-A1101",
                Some(&manifest_path),
                format!("internal dependencies for '{}' differ from the architecture contract", member.id),
                "declare the exact same direct internal dependencies in Cargo.toml and zryna.workspace.json",
            ));
        }
    }
}

fn validate_adapters(
    root: &Path,
    contract: &WorkspaceContract,
    diagnostics: &mut ValidationDiagnostics,
) {
    for adapter in &contract.adapters {
        if diagnostics.is_halted() {
            return;
        }
        let adapter_root = root.join(&adapter.root);
        let readme_path = adapter_root.join("README.md");
        let worker_path = adapter_root.join("src/worker.mjs");
        let package_path = adapter_root.join("package.json");
        if !readme_path.is_file() || !worker_path.is_file() {
            diagnostics.push(architecture_error(
                "ZRYNA-A1010",
                Some(Path::new(&adapter.root)),
                "registered frontend adapter is missing README.md or src/worker.mjs",
                "restore the documented newline-JSON worker boundary",
            ));
        }
        if diagnostics.is_halted() {
            return;
        }
        let Some(package) = read_json(&package_path, diagnostics) else {
            continue;
        };
        let expected_name = format!("@zryna/adapter-{}", adapter.id);
        if package.get("name").and_then(serde_json::Value::as_str) != Some(expected_name.as_str()) {
            diagnostics.push(architecture_error(
                "ZRYNA-A1010",
                Some(&package_path),
                format!("adapter package name must be '{expected_name}'"),
                "make the package identity derive from the registered adapter id",
            ));
        }
        let metadata = package.get("zryna");
        let metadata_id =
            metadata.and_then(|value| value.get("adapterId")).and_then(serde_json::Value::as_str);
        let metadata_protocol = metadata
            .and_then(|value| value.get("protocolVersion"))
            .and_then(serde_json::Value::as_u64);
        let metadata_worker =
            metadata.and_then(|value| value.get("worker")).and_then(serde_json::Value::as_str);
        if metadata_id != Some(adapter.id.as_str())
            || metadata_protocol != Some(u64::from(adapter.protocol_version))
            || metadata_worker != Some("src/worker.mjs")
        {
            diagnostics.push(architecture_error(
                "ZRYNA-A1010",
                Some(&package_path),
                "adapter metadata differs from zryna.workspace.json",
                "set zryna.adapterId, zryna.protocolVersion, and zryna.worker to the registered values",
            ));
        }
        let Some((toolchain_name, toolchain_version)) = adapter.toolchain.rsplit_once('@') else {
            diagnostics.push(architecture_error(
                "ZRYNA-A1011",
                Some(Path::new(CONTRACT_FILE)),
                format!(
                    "adapter toolchain '{}' is not an exact package@version",
                    adapter.toolchain
                ),
                "pin one exact frontend package version in the architecture contract",
            ));
            continue;
        };
        let dependency_version = package
            .get("dependencies")
            .and_then(|value| value.get(toolchain_name))
            .and_then(serde_json::Value::as_str);
        if toolchain_name.is_empty()
            || toolchain_version.is_empty()
            || dependency_version != Some(toolchain_version)
        {
            diagnostics.push(architecture_error(
                "ZRYNA-A1011",
                Some(&package_path),
                format!("adapter must pin toolchain '{}' exactly", adapter.toolchain),
                "make package.json dependencies match the registered package and version",
            ));
        }
    }
}

fn validate_dependency_graph(
    contract: &WorkspaceContract,
    diagnostics: &mut ValidationDiagnostics,
) {
    let members: BTreeMap<&str, &MemberContract> =
        contract.members.iter().map(|member| (member.id.as_str(), member)).collect();
    for member in &contract.members {
        if diagnostics.is_halted() {
            return;
        }
        for dependency in &member.dependencies {
            if diagnostics.is_halted() {
                return;
            }
            let Some(target) = members.get(dependency.as_str()) else {
                diagnostics.push(architecture_error(
                    "ZRYNA-A1101",
                    Some(Path::new(&member.root)),
                    format!("'{}' depends on unknown member '{dependency}'", member.id),
                    "register the dependency or remove the edge",
                ));
                continue;
            };
            if !allowed_layer_edge(member.kind, target.kind) {
                diagnostics.push(architecture_error(
                    "ZRYNA-A1102",
                    Some(Path::new(&member.root)),
                    format!("forbidden dependency direction: '{}' -> '{dependency}'", member.id),
                    "depend only toward lower-level contracts or route orchestration through zryna-driver",
                ));
            }
        }
    }
    for member in &contract.members {
        if diagnostics.is_halted() {
            return;
        }
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        if has_cycle(member.id.as_str(), &members, &mut visiting, &mut visited) {
            diagnostics.push(architecture_error(
                "ZRYNA-A1103",
                Some(Path::new(&member.root)),
                format!("dependency cycle reaches '{}'", member.id),
                "break the cycle by moving shared contracts into a lower foundation member",
            ));
        }
    }
}

#[derive(Clone, Copy)]
struct ScanLimits {
    entries: usize,
    depth: usize,
    file_bytes: u64,
    total_bytes: u64,
    diagnostics: usize,
}

const PRODUCTION_SCAN_LIMITS: ScanLimits = ScanLimits {
    entries: MAX_SCAN_ENTRIES,
    depth: MAX_SCAN_DEPTH,
    file_bytes: MAX_SCANNED_FILE_BYTES,
    total_bytes: MAX_SCAN_TOTAL_BYTES,
    diagnostics: MAX_SCAN_DIAGNOSTICS,
};

struct ScanState {
    limits: ScanLimits,
    entries_seen: usize,
    bytes_seen: u64,
    diagnostics_seen: usize,
    halted: bool,
}

struct ScanPolicy<'a> {
    exclusions: BTreeMap<PathBuf, bool>,
    contract_source: &'a str,
}

impl<'a> ScanPolicy<'a> {
    fn new(contract: &WorkspaceContract, contract_source: &'a str) -> Self {
        let mut exclusions =
            BTreeMap::from([(PathBuf::from(".git"), false), (PathBuf::from("node_modules"), true)]);
        for output in &contract.outputs {
            exclusions.insert(PathBuf::from(output), true);
        }
        for adapter in &contract.adapters {
            exclusions.insert(Path::new(&adapter.root).join("node_modules"), true);
        }
        Self { exclusions, contract_source }
    }

    fn excluded_shape(&self, relative: &Path) -> Option<bool> {
        self.exclusions.get(relative).copied()
    }
}

impl ScanState {
    const fn new(limits: ScanLimits) -> Self {
        Self { limits, entries_seen: 0, bytes_seen: 0, diagnostics_seen: 0, halted: false }
    }
}

fn validate_bounded_filesystem(
    root: &Path,
    contract: &WorkspaceContract,
    contract_source: &str,
    diagnostics: &mut ValidationDiagnostics,
) -> bool {
    let policy = ScanPolicy::new(contract, contract_source);
    let mut state = ScanState::new(PRODUCTION_SCAN_LIMITS);
    scan_path(root, root, &policy, 0, &mut state, diagnostics);
    !state.halted
}

fn scan_path(
    root: &Path,
    path: &Path,
    policy: &ScanPolicy<'_>,
    depth: usize,
    state: &mut ScanState,
    diagnostics: &mut ValidationDiagnostics,
) {
    if state.halted {
        return;
    }
    if depth > state.limits.depth {
        halt_scan(
            state,
            diagnostics,
            path.strip_prefix(root).ok(),
            "architecture scan exceeded its deterministic depth budget",
        );
        return;
    }
    if state.entries_seen >= state.limits.entries {
        halt_scan(
            state,
            diagnostics,
            path.strip_prefix(root).ok(),
            "architecture scan exceeded its deterministic entry budget",
        );
        return;
    }
    state.entries_seen += 1;

    let relative = path.strip_prefix(root).unwrap_or(path);
    let metadata = match fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) => {
            push_scan_diagnostic(
                state,
                diagnostics,
                architecture_error(
                    "ZRYNA-A1205",
                    Some(relative),
                    format!("filesystem entry could not be inspected: {error}"),
                    "restore a stable readable entry and retry",
                ),
            );
            return;
        }
    };
    if metadata_is_link_or_reparse(&metadata) {
        push_scan_diagnostic(
            state,
            diagnostics,
            architecture_error(
                "ZRYNA-A1201",
                Some(relative),
                "symlinks are forbidden inside the controlled workspace",
                "replace the link with a real in-workspace file or directory",
            ),
        );
        return;
    }

    if let Some(expected_directory) = policy.excluded_shape(relative) {
        validate_excluded_entry(relative, &metadata, expected_directory, state, diagnostics);
        return;
    }

    if metadata.is_file() {
        scan_regular_file(path, relative, &metadata, policy, state, diagnostics);
        return;
    }
    if !metadata.is_dir() {
        push_scan_diagnostic(
            state,
            diagnostics,
            architecture_error(
                "ZRYNA-A1201",
                Some(relative),
                "non-regular filesystem entries are forbidden",
                "remove sockets, devices, and FIFOs from controlled source roots",
            ),
        );
        return;
    }

    scan_directory(root, path, relative, policy, depth, state, diagnostics);
}

fn validate_excluded_entry(
    relative: &Path,
    metadata: &fs::Metadata,
    expected_directory: bool,
    state: &mut ScanState,
    diagnostics: &mut ValidationDiagnostics,
) {
    if expected_directory && !metadata.is_dir() {
        push_scan_diagnostic(
            state,
            diagnostics,
            architecture_error(
                "ZRYNA-A1201",
                Some(relative),
                "declared generated output must be a real directory",
                "replace it with the declared non-symlink output directory",
            ),
        );
    } else if !expected_directory && !metadata.is_dir() && !metadata.is_file() {
        push_scan_diagnostic(
            state,
            diagnostics,
            architecture_error(
                "ZRYNA-A1201",
                Some(relative),
                "Git metadata must be a real file or directory",
                "replace it with regular Git metadata",
            ),
        );
    }
}

fn scan_regular_file(
    path: &Path,
    relative: &Path,
    metadata: &fs::Metadata,
    policy: &ScanPolicy<'_>,
    state: &mut ScanState,
    diagnostics: &mut ValidationDiagnostics,
) {
    let Some(advertised_total) = state.bytes_seen.checked_add(metadata.len()) else {
        halt_scan(
            state,
            diagnostics,
            Some(relative),
            "architecture scan byte accounting overflowed",
        );
        return;
    };
    if advertised_total > state.limits.total_bytes {
        halt_scan(
            state,
            diagnostics,
            Some(relative),
            "architecture scan exceeded its deterministic aggregate byte budget",
        );
        return;
    }
    state.bytes_seen = advertised_total;
    match read_bounded_utf8_with_expected_size(
        path,
        Some(relative),
        state.limits.file_bytes,
        "ZRYNA-A1203",
        "restore the controlled UTF-8 source file",
        metadata.len(),
    ) {
        Ok((source, _)) => {
            if relative == Path::new(CONTRACT_FILE) && source != policy.contract_source {
                halt_scan_with_diagnostic(
                    state,
                    diagnostics,
                    architecture_error(
                        "ZRYNA-A1203",
                        Some(relative),
                        "workspace contract changed after it was parsed",
                        "stop concurrent mutation and retry architecture validation",
                    ),
                );
            }
        }
        Err(diagnostic) if diagnostic.code == "ZRYNA-A1204" => {
            halt_scan_with_diagnostic(state, diagnostics, diagnostic);
        }
        Err(diagnostic) => push_scan_diagnostic(state, diagnostics, diagnostic),
    }
}

fn scan_directory(
    root: &Path,
    path: &Path,
    relative: &Path,
    policy: &ScanPolicy<'_>,
    depth: usize,
    state: &mut ScanState,
    diagnostics: &mut ValidationDiagnostics,
) {
    let children = match fs::read_dir(path) {
        Ok(value) => value,
        Err(error) => {
            push_scan_diagnostic(
                state,
                diagnostics,
                architecture_error(
                    "ZRYNA-A1205",
                    Some(relative),
                    format!("directory scan failed: {error}"),
                    "restore read access; incomplete scans never pass",
                ),
            );
            return;
        }
    };
    let remaining_entries = state.limits.entries.saturating_sub(state.entries_seen);
    let mut child_paths = Vec::new();
    for child in children {
        if state.halted {
            break;
        }
        match child {
            Ok(entry) => {
                if child_paths.len() >= remaining_entries {
                    halt_scan(
                        state,
                        diagnostics,
                        Some(relative),
                        "architecture scan exceeded its deterministic entry budget",
                    );
                    break;
                }
                child_paths.push(entry.path());
            }
            Err(error) => push_scan_diagnostic(
                state,
                diagnostics,
                architecture_error(
                    "ZRYNA-A1205",
                    Some(relative),
                    format!("directory entry could not be read: {error}"),
                    "restore directory consistency and retry",
                ),
            ),
        }
    }
    child_paths.sort();
    for child_path in child_paths {
        if state.halted {
            break;
        }
        scan_path(root, &child_path, policy, depth + 1, state, diagnostics);
    }
}

fn push_scan_diagnostic(
    state: &mut ScanState,
    diagnostics: &mut ValidationDiagnostics,
    diagnostic: Diagnostic,
) {
    if state.halted {
        return;
    }
    if state.diagnostics_seen.saturating_add(1) >= state.limits.diagnostics {
        halt_scan(
            state,
            diagnostics,
            None,
            "architecture scan exceeded its deterministic diagnostic budget",
        );
        return;
    }
    diagnostics.push(diagnostic);
    state.diagnostics_seen += 1;
    if diagnostics.is_halted() {
        state.halted = true;
    }
}

fn halt_scan(
    state: &mut ScanState,
    diagnostics: &mut ValidationDiagnostics,
    path: Option<&Path>,
    message: &str,
) {
    if state.halted {
        return;
    }
    halt_scan_with_diagnostic(
        state,
        diagnostics,
        architecture_error(
            "ZRYNA-A1204",
            path,
            message,
            "reduce controlled input size or depth; incomplete scans never pass",
        ),
    );
}

fn halt_scan_with_diagnostic(
    state: &mut ScanState,
    diagnostics: &mut ValidationDiagnostics,
    diagnostic: Diagnostic,
) {
    if state.halted {
        return;
    }
    if state.diagnostics_seen < state.limits.diagnostics {
        diagnostics.push(diagnostic);
        state.diagnostics_seen += 1;
    }
    state.halted = true;
}

fn read_toml(path: &Path, diagnostics: &mut ValidationDiagnostics) -> Option<toml::Value> {
    let (source, _) = match read_bounded_utf8(
        path,
        Some(path),
        MAX_MANIFEST_BYTES,
        "ZRYNA-A1005",
        "restore the canonical component Cargo.toml",
    ) {
        Ok(value) => value,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            return None;
        }
    };
    match toml::from_str(&source) {
        Ok(value) => Some(value),
        Err(error) => {
            diagnostics.push(architecture_error(
                "ZRYNA-A1006",
                Some(path),
                format!("Cargo manifest is invalid: {error}"),
                "repair the manifest before architecture validation",
            ));
            None
        }
    }
}

fn read_json(path: &Path, diagnostics: &mut ValidationDiagnostics) -> Option<serde_json::Value> {
    let (source, _) = match read_bounded_utf8(
        path,
        Some(path),
        MAX_MANIFEST_BYTES,
        "ZRYNA-A1010",
        "restore the registered adapter package.json",
    ) {
        Ok(value) => value,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            return None;
        }
    };
    match serde_json::from_str(&source) {
        Ok(value) => Some(value),
        Err(error) => {
            diagnostics.push(architecture_error(
                "ZRYNA-A1010",
                Some(path),
                format!("adapter package manifest is invalid JSON: {error}"),
                "repair package.json before architecture validation",
            ));
            None
        }
    }
}

fn internal_dependencies<'a>(
    manifest: &'a toml::Value,
    declared: &BTreeSet<&str>,
) -> BTreeSet<&'a str> {
    manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|table| table.keys())
        .map(String::as_str)
        .filter(|name| declared.contains(name))
        .collect()
}

const fn allowed_layer_edge(source: MemberKind, target: MemberKind) -> bool {
    use MemberKind::{Application, Backend, Compiler, Foundation, Frontend, Orchestrator};
    match source {
        Foundation | Frontend => matches!(target, Foundation),
        Compiler => matches!(target, Foundation | Compiler),
        Backend => matches!(target, Foundation | Compiler),
        Orchestrator => !matches!(target, Application | Orchestrator),
        Application => matches!(target, Foundation | Orchestrator),
    }
}

fn has_cycle<'a>(
    id: &'a str,
    members: &BTreeMap<&'a str, &'a MemberContract>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> bool {
    if visiting.contains(id) {
        return true;
    }
    if visited.contains(id) {
        return false;
    }
    visiting.insert(id);
    if let Some(member) = members.get(id) {
        for dependency in &member.dependencies {
            if has_cycle(dependency, members, visiting, visited) {
                return true;
            }
        }
    }
    visiting.remove(id);
    visited.insert(id);
    false
}

fn safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\\') {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute()
        && path.components().all(|component| matches!(component, Component::Normal(_)))
}

fn valid_component_entry(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains(['/', '\\'])
        && Path::new(value).components().count() == 1
}

fn valid_id(value: &str) -> bool {
    value.bytes().next().is_some_and(|byte| byte.is_ascii_lowercase())
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || (byte == b'-' && index > 0)
        })
        && !value.ends_with('-')
        && !value.contains("--")
}

fn architecture_error(
    code: &str,
    path: Option<&Path>,
    message: impl Into<String>,
    guidance: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error(
        code,
        path.map(|value| value.to_string_lossy().replace('\\', "/")),
        message,
        guidance,
    )
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use super::read_bounded_utf8;
    use super::{
        AdapterContract, CONTRACT_PROFILE, CONTRACT_VERSION, ControlledReadPolicy,
        MAX_CONTRACT_BYTES, MAX_MANIFEST_BYTES, MemberContract, MemberKind, ScanLimits, ScanPolicy,
        ScanState, ValidationDiagnostics, WorkspaceContract, load_contract,
        read_bounded_utf8_with_expected_size, read_bounded_utf8_with_hooks, read_toml,
        safe_relative_path, scan_path, valid_id, validate_contract_identity,
        validate_contract_unchanged, validate_workspace,
    };

    const FIXTURE_LIMITS: ScanLimits =
        ScanLimits { entries: 128, depth: 16, file_bytes: 128, total_bytes: 1024, diagnostics: 8 };

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct TempFixture {
        root: PathBuf,
    }

    impl TempFixture {
        fn new() -> std::io::Result<Self> {
            let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("zryna-architecture-{}-{sequence}", std::process::id()));
            fs::create_dir_all(&root)?;
            Ok(Self { root })
        }

        fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
            self.root.join(relative)
        }

        fn directory(&self, relative: impl AsRef<Path>) -> std::io::Result<PathBuf> {
            let path = self.path(relative);
            fs::create_dir_all(&path)?;
            Ok(path)
        }

        fn write(&self, relative: impl AsRef<Path>, bytes: &[u8]) -> std::io::Result<PathBuf> {
            let path = self.path(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, bytes)?;
            Ok(path)
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn fixture_contract() -> WorkspaceContract {
        WorkspaceContract {
            schema: "./schemas/zryna-workspace-v1.schema.json".to_owned(),
            version: CONTRACT_VERSION,
            profile: CONTRACT_PROFILE.to_owned(),
            members: Vec::new(),
            adapters: vec![AdapterContract {
                id: "typescript-6".to_owned(),
                root: "adapters/typescript-6".to_owned(),
                protocol_version: 1,
                toolchain: "@typescript/typescript6@6.0.2".to_owned(),
                allowed_entries: Vec::new(),
            }],
            outputs: vec!["target".to_owned(), ".zryna/cache".to_owned(), ".zryna/out".to_owned()],
        }
    }

    fn write_minimal_workspace(fixture: &TempFixture) -> Result<(), Box<dyn Error>> {
        let contract = WorkspaceContract {
            schema: "./schemas/zryna-workspace-v1.schema.json".to_owned(),
            version: CONTRACT_VERSION,
            profile: CONTRACT_PROFILE.to_owned(),
            members: vec![MemberContract {
                id: "sample".to_owned(),
                root: "crates/sample".to_owned(),
                kind: MemberKind::Foundation,
                dependencies: Vec::new(),
                allowed_entries: vec![
                    "Cargo.toml".to_owned(),
                    "README.md".to_owned(),
                    "src".to_owned(),
                ],
            }],
            adapters: Vec::new(),
            outputs: vec!["target".to_owned(), ".zryna/cache".to_owned(), ".zryna/out".to_owned()],
        };
        fixture.write("zryna.workspace.json", &serde_json::to_vec_pretty(&contract)?)?;
        fixture.write(
            "Cargo.toml",
            b"[workspace]\nresolver = \"2\"\nmembers = [\"crates/sample\"]\n",
        )?;
        fixture.write(
            "crates/sample/Cargo.toml",
            b"[package]\nname = \"sample\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )?;
        fixture.write("crates/sample/README.md", b"# sample\n")?;
        fixture.write("crates/sample/src/lib.rs", b"//! Sample fixture.\n")?;
        Ok(())
    }

    fn scan_fixture(root: &Path, limits: ScanLimits) -> (bool, Vec<zryna_diagnostics::Diagnostic>) {
        let contract = fixture_contract();
        let policy = ScanPolicy::new(&contract, "");
        let mut diagnostics = ValidationDiagnostics::default();
        let mut state = ScanState::new(limits);
        scan_path(root, root, &policy, 0, &mut state, &mut diagnostics);
        (!state.halted, diagnostics.into_vec())
    }

    fn has_code(diagnostics: &[zryna_diagnostics::Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn rejects_unsafe_paths() {
        assert!(safe_relative_path("crates/zryna-ir"));
        assert!(!safe_relative_path("../outside"));
        assert!(!safe_relative_path("C:\\outside"));
        assert!(!safe_relative_path("crates\\zryna-ir"));
    }

    #[test]
    fn validates_canonical_ids() {
        assert!(valid_id("zryna-backend-native"));
        assert!(!valid_id("ZRYNA-native"));
        assert!(!valid_id("7-native"));
        assert!(!valid_id("zryna--native"));
    }

    #[test]
    fn validation_wide_diagnostic_budget_is_deterministic() -> Result<(), Box<dyn Error>> {
        let fixture = TempFixture::new()?;
        write_minimal_workspace(&fixture)?;
        for index in 0..300 {
            fixture.write(format!("extra-{index:03}.txt"), b"")?;
        }

        let first = validate_workspace(&fixture.root);
        let second = validate_workspace(&fixture.root);
        assert_eq!(first.diagnostics, second.diagnostics);
        assert_eq!(first.diagnostics.len(), 256);
        assert_eq!(
            first.diagnostics.iter().filter(|diagnostic| diagnostic.code == "ZRYNA-A1004").count(),
            255
        );
        assert_eq!(
            first.diagnostics.iter().filter(|diagnostic| diagnostic.code == "ZRYNA-A1204").count(),
            1
        );
        assert!(
            first
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("diagnostic budget"))
        );
        Ok(())
    }

    #[test]
    fn component_registry_is_bounded_before_scanning() {
        let mut contract = fixture_contract();
        contract.adapters.clear();
        contract.members = (0..257)
            .map(|index| MemberContract {
                id: format!("component-{index}"),
                root: format!("crates/component-{index}"),
                kind: MemberKind::Foundation,
                dependencies: Vec::new(),
                allowed_entries: Vec::new(),
            })
            .collect();
        let mut diagnostics = ValidationDiagnostics::default();

        validate_contract_identity(&contract, &mut diagnostics);

        assert!(diagnostics.into_vec().iter().any(|diagnostic| {
            diagnostic.code == "ZRYNA-A1001" && diagnostic.message.contains("257 components")
        }));
    }

    #[test]
    fn expected_size_and_contract_source_bind_the_scan_snapshot() -> Result<(), Box<dyn Error>> {
        let size_fixture = TempFixture::new()?;
        let controlled = size_fixture.write("controlled.zry", b"first")?;
        let expected_size = fs::metadata(&controlled)?.len();
        fs::write(&controlled, b"larger")?;
        let size_result = read_bounded_utf8_with_expected_size(
            &controlled,
            Some(Path::new("controlled.zry")),
            32,
            "ZRYNA-A1203",
            "restore the file",
            expected_size,
        );
        assert!(matches!(size_result, Err(diagnostic) if diagnostic.code == "ZRYNA-A1203"));

        let contract_fixture = TempFixture::new()?;
        contract_fixture.write("zryna.workspace.json", b"contract-b")?;
        let contract = fixture_contract();
        let policy = ScanPolicy::new(&contract, "contract-a");
        let mut scan_diagnostics = ValidationDiagnostics::default();
        let mut state = ScanState::new(FIXTURE_LIMITS);
        scan_path(
            &contract_fixture.root,
            &contract_fixture.root,
            &policy,
            0,
            &mut state,
            &mut scan_diagnostics,
        );
        assert!(state.halted);
        assert!(has_code(&scan_diagnostics.into_vec(), "ZRYNA-A1203"));

        let mut final_diagnostics = ValidationDiagnostics::default();
        validate_contract_unchanged(&contract_fixture.root, "contract-a", &mut final_diagnostics);
        assert!(has_code(&final_diagnostics.into_vec(), "ZRYNA-A1203"));
        Ok(())
    }

    #[test]
    fn excludes_only_declared_generated_directories() -> Result<(), Box<dyn Error>> {
        let fixture = TempFixture::new()?;
        fixture.write("target/not-utf8.bin", &[0xff])?;
        fixture.write(".zryna/cache/not-utf8.bin", &[0xff])?;
        fixture.write(".zryna/out/not-utf8.bin", &[0xff])?;
        fixture.write("node_modules/not-utf8.bin", &[0xff])?;
        fixture.write("adapters/typescript-6/node_modules/not-utf8.bin", &[0xff])?;

        let (completed, diagnostics) = scan_fixture(&fixture.root, FIXTURE_LIMITS);
        assert!(completed);
        assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:#?}");
        Ok(())
    }

    #[test]
    fn inspects_nested_generated_names_and_unknown_output_siblings() -> Result<(), Box<dyn Error>> {
        let fixture = TempFixture::new()?;
        fixture.write("crates/example/src/target/not-utf8.bin", &[0xff])?;
        fixture.write(".zryna/other/not-utf8.bin", &[0xff])?;

        let (_, diagnostics) = scan_fixture(&fixture.root, FIXTURE_LIMITS);
        assert_eq!(
            diagnostics.iter().filter(|diagnostic| diagnostic.code == "ZRYNA-A1203").count(),
            2
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_utf8_and_oversized_regular_files() -> Result<(), Box<dyn Error>> {
        let fixture = TempFixture::new()?;
        fixture.write("invalid.zry", &[0xff])?;
        fixture.write("large.zry", b"12345")?;
        fixture.write("z-after.zry", &[0xff])?;
        let limits = ScanLimits { file_bytes: 4, ..FIXTURE_LIMITS };

        let (completed, diagnostics) = scan_fixture(&fixture.root, limits);
        assert!(!completed);
        assert_eq!(
            diagnostics.iter().filter(|diagnostic| diagnostic.code == "ZRYNA-A1203").count(),
            1
        );
        assert!(has_code(&diagnostics, "ZRYNA-A1204"));
        Ok(())
    }

    #[test]
    fn aggregate_entry_and_depth_budgets_halt_globally() -> Result<(), Box<dyn Error>> {
        let aggregate = TempFixture::new()?;
        aggregate.write("a.zry", b"123")?;
        aggregate.write("b.zry", b"456")?;
        let (_, aggregate_diagnostics) =
            scan_fixture(&aggregate.root, ScanLimits { total_bytes: 5, ..FIXTURE_LIMITS });
        assert_eq!(aggregate_diagnostics.len(), 1);
        assert_eq!(aggregate_diagnostics[0].code, "ZRYNA-A1204");

        let invalid_aggregate = TempFixture::new()?;
        invalid_aggregate.write("a.zry", &[0xff, 0xff, 0xff])?;
        invalid_aggregate.write("b.zry", &[0xff, 0xff, 0xff])?;
        let (invalid_completed, invalid_diagnostics) =
            scan_fixture(&invalid_aggregate.root, ScanLimits { total_bytes: 5, ..FIXTURE_LIMITS });
        assert!(!invalid_completed);
        assert!(has_code(&invalid_diagnostics, "ZRYNA-A1203"));
        assert!(has_code(&invalid_diagnostics, "ZRYNA-A1204"));

        let entries = TempFixture::new()?;
        entries.write("a.zry", b"a")?;
        entries.write("b.zry", b"b")?;
        let (entry_completed, entry_diagnostics) =
            scan_fixture(&entries.root, ScanLimits { entries: 2, ..FIXTURE_LIMITS });
        assert!(!entry_completed);
        assert_eq!(entry_diagnostics.len(), 1);
        assert_eq!(entry_diagnostics[0].code, "ZRYNA-A1204");

        let depth = TempFixture::new()?;
        depth.write("a/b/value.zry", b"value")?;
        let (depth_completed, depth_diagnostics) =
            scan_fixture(&depth.root, ScanLimits { depth: 1, ..FIXTURE_LIMITS });
        assert!(!depth_completed);
        assert_eq!(depth_diagnostics.len(), 1);
        assert_eq!(depth_diagnostics[0].code, "ZRYNA-A1204");
        Ok(())
    }

    #[test]
    fn diagnostic_budget_reserves_one_terminal_diagnostic() -> Result<(), Box<dyn Error>> {
        let fixture = TempFixture::new()?;
        fixture.write("a.zry", &[0xff])?;
        fixture.write("b.zry", &[0xff])?;
        let (completed, diagnostics) =
            scan_fixture(&fixture.root, ScanLimits { diagnostics: 1, ..FIXTURE_LIMITS });

        assert!(!completed);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "ZRYNA-A1204");
        assert!(diagnostics[0].message.contains("diagnostic budget"));
        Ok(())
    }

    #[test]
    fn bounded_reader_detects_same_size_replacement() -> Result<(), Box<dyn Error>> {
        let fixture = TempFixture::new()?;
        let controlled = fixture.write("controlled.zry", b"first")?;
        let replacement = fixture.write("replacement.zry", b"other")?;
        let result = read_bounded_utf8_with_hooks(
            &controlled,
            ControlledReadPolicy {
                diagnostic_path: Some(Path::new("controlled.zry")),
                max_bytes: 32,
                unavailable_code: "ZRYNA-A1203",
                unavailable_guidance: "restore the file",
                expected_size: None,
            },
            || {
                assert!(fs::remove_file(&controlled).is_ok());
                assert!(fs::rename(&replacement, &controlled).is_ok());
            },
            || {},
        );

        assert!(matches!(result, Err(diagnostic) if diagnostic.code == "ZRYNA-A1203"));
        Ok(())
    }

    #[test]
    fn different_files_never_share_an_identity() -> Result<(), Box<dyn Error>> {
        let fixture = TempFixture::new()?;
        let left = fixture.write("left.zry", b"same")?;
        let right = fixture.write("right.zry", b"same")?;
        let left_handle = same_file::Handle::from_file(fs::File::open(left)?)?;
        let right_handle = same_file::Handle::from_file(fs::File::open(right)?)?;

        assert_ne!(left_handle, right_handle);
        Ok(())
    }

    #[test]
    fn oversized_contract_and_manifest_fail_before_parsing() -> Result<(), Box<dyn Error>> {
        let fixture = TempFixture::new()?;
        let contract_size = usize::try_from(MAX_CONTRACT_BYTES + 1)?;
        fixture.write("zryna.workspace.json", &vec![b' '; contract_size])?;
        let contract_result = load_contract(&fixture.root);
        assert!(matches!(contract_result, Err(diagnostic) if diagnostic.code == "ZRYNA-A1204"));

        let manifest_size = usize::try_from(MAX_MANIFEST_BYTES + 1)?;
        let manifest = fixture.write("Cargo.toml", &vec![b' '; manifest_size])?;
        let mut diagnostics = ValidationDiagnostics::default();
        assert!(read_toml(&manifest, &mut diagnostics).is_none());
        let diagnostics = diagnostics.into_vec();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "ZRYNA-A1204");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_and_socket_at_excluded_paths() -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;

        let symlink_fixture = TempFixture::new()?;
        let destination = symlink_fixture.directory("real-target")?;
        symlink(&destination, symlink_fixture.path("target"))?;
        let (_, symlink_diagnostics) = scan_fixture(&symlink_fixture.root, FIXTURE_LIMITS);
        assert!(has_code(&symlink_diagnostics, "ZRYNA-A1201"));

        let socket_fixture = TempFixture::new()?;
        let _listener = UnixListener::bind(socket_fixture.path("target"))?;
        let (_, socket_diagnostics) = scan_fixture(&socket_fixture.root, FIXTURE_LIMITS);
        assert!(has_code(&socket_diagnostics, "ZRYNA-A1201"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_unreadable_and_mid_read_modified_files() -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::PermissionsExt;

        let fixture = TempFixture::new()?;
        let unreadable = fixture.write("unreadable.zry", b"value")?;
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000))?;
        let unreadable_result = read_bounded_utf8(
            &unreadable,
            Some(Path::new("unreadable.zry")),
            32,
            "ZRYNA-A1203",
            "restore the file",
        );
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o600))?;
        assert!(matches!(unreadable_result, Err(diagnostic) if diagnostic.code == "ZRYNA-A1203"));

        let modified = fixture.write("modified.zry", b"value")?;
        let modified_result = read_bounded_utf8_with_hooks(
            &modified,
            ControlledReadPolicy {
                diagnostic_path: Some(Path::new("modified.zry")),
                max_bytes: 32,
                unavailable_code: "ZRYNA-A1203",
                unavailable_guidance: "restore the file",
                expected_size: None,
            },
            || {},
            || assert!(fs::write(&modified, b"changed").is_ok()),
        );
        assert!(matches!(modified_result, Err(diagnostic) if diagnostic.code == "ZRYNA-A1203"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn rejects_windows_directory_reparse_points() -> Result<(), Box<dyn Error>> {
        use std::os::windows::fs::symlink_dir;

        let fixture = TempFixture::new()?;
        let destination = fixture.directory("real-target")?;
        symlink_dir(&destination, fixture.path("target"))?;
        let (_, diagnostics) = scan_fixture(&fixture.root, FIXTURE_LIMITS);
        assert!(has_code(&diagnostics, "ZRYNA-A1201"));
        Ok(())
    }

    #[test]
    fn current_repository_satisfies_the_contract() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let report = validate_workspace(&root);
        assert!(report.is_valid(), "architecture diagnostics: {:#?}", report.diagnostics);
    }
}
