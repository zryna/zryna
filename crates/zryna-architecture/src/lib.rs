//! Canonical fail-closed Zryna repository architecture engine.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use zryna_diagnostics::Diagnostic;

const CONTRACT_FILE: &str = "zryna.workspace.json";
const CONTRACT_VERSION: u32 = 1;
const CONTRACT_PROFILE: &str = "zryna-compiler-workspace-v1";
const MAX_CONTRACT_BYTES: u64 = 1024 * 1024;
const MAX_SCAN_ENTRIES: usize = 50_000;
const MAX_SCAN_DEPTH: usize = 32;
const IGNORED_DIRECTORY_NAMES: &[&str] = &[".git", ".zry", "dist", "node_modules", "target"];

const ALLOWED_ROOT_ENTRIES: &[&str] = &[
    ".cargo",
    ".git",
    ".github",
    ".gitattributes",
    ".gitignore",
    ".zry",
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

/// Validates a complete Zryna workspace and fails closed on incomplete inspection.
#[must_use]
pub fn validate_workspace(root: &Path) -> ValidationReport {
    let mut diagnostics = Vec::new();
    let canonical_root = match canonical_workspace_root(root) {
        Ok(value) => value,
        Err(diagnostic) => return ValidationReport { diagnostics: vec![diagnostic] },
    };
    let contract = match load_contract(&canonical_root) {
        Ok(value) => value,
        Err(diagnostic) => return ValidationReport { diagnostics: vec![diagnostic] },
    };

    validate_contract_identity(&contract, &mut diagnostics);
    validate_root_entries(&canonical_root, &mut diagnostics);
    validate_paths(&canonical_root, &contract, &mut diagnostics);
    validate_component_entries(&canonical_root, &contract, &mut diagnostics);
    validate_members(&canonical_root, &contract, &mut diagnostics);
    validate_adapters(&canonical_root, &contract, &mut diagnostics);
    validate_dependency_graph(&contract, &mut diagnostics);
    validate_bounded_filesystem(&canonical_root, &mut diagnostics);

    diagnostics.sort_by(|left, right| {
        (&left.code, &left.path, &left.message).cmp(&(&right.code, &right.path, &right.message))
    });
    ValidationReport { diagnostics }
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
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
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

fn load_contract(root: &Path) -> Result<WorkspaceContract, Diagnostic> {
    let path = root.join(CONTRACT_FILE);
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        architecture_error(
            "ZRYNA-A1001",
            Some(&path),
            format!("workspace contract is unavailable: {error}"),
            "restore the canonical zryna.workspace.json file",
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(architecture_error(
            "ZRYNA-A1201",
            Some(&path),
            "workspace contract must be a regular, non-symlink file",
            "replace it with a regular UTF-8 JSON file",
        ));
    }
    if metadata.len() > MAX_CONTRACT_BYTES {
        return Err(architecture_error(
            "ZRYNA-A1204",
            Some(&path),
            "workspace contract exceeds the one MiB safety limit",
            "remove generated or unrelated data from the contract",
        ));
    }
    let source = fs::read_to_string(&path).map_err(|error| {
        architecture_error(
            "ZRYNA-A1203",
            Some(&path),
            format!("workspace contract is not readable UTF-8: {error}"),
            "save the contract as stable UTF-8 text",
        )
    })?;
    serde_json::from_str(&source).map_err(|error| {
        architecture_error(
            "ZRYNA-A1001",
            Some(&path),
            format!("workspace contract is invalid: {error}"),
            "match schemas/zryna-workspace-v1.schema.json exactly; unknown fields are forbidden",
        )
    })
}

fn validate_contract_identity(contract: &WorkspaceContract, diagnostics: &mut Vec<Diagnostic>) {
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

fn validate_root_entries(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let allowed: BTreeSet<&str> = ALLOWED_ROOT_ENTRIES.iter().copied().collect();
    let entries = match fs::read_dir(root) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(architecture_error(
                "ZRYNA-A1205",
                Some(root),
                format!("workspace root scan failed: {error}"),
                "restore read access; incomplete scans never pass",
            ));
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(value) => value,
            Err(error) => {
                diagnostics.push(architecture_error(
                    "ZRYNA-A1205",
                    Some(root),
                    format!("workspace entry could not be inspected: {error}"),
                    "restore directory consistency and retry",
                ));
                continue;
            }
        };
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

fn validate_paths(root: &Path, contract: &WorkspaceContract, diagnostics: &mut Vec<Diagnostic>) {
    let mut identities = BTreeSet::new();
    let mut roots = BTreeSet::new();
    for (id, member_root) in contract
        .members
        .iter()
        .map(|member| (&member.id, &member.root))
        .chain(contract.adapters.iter().map(|adapter| (&adapter.id, &adapter.root)))
    {
        if !valid_id(id) || !identities.insert(id.to_ascii_lowercase()) {
            diagnostics.push(architecture_error(
                "ZRYNA-A1003",
                Some(Path::new(CONTRACT_FILE)),
                format!("component id '{id}' is invalid or collides case-insensitively"),
                "use one unique lowercase kebab-case identifier",
            ));
        }
        if !safe_relative_path(member_root) || !roots.insert(member_root.to_ascii_lowercase()) {
            diagnostics.push(architecture_error(
                "ZRYNA-A1003",
                Some(Path::new(member_root)),
                format!("component root '{member_root}' is unsafe or duplicated"),
                "use a unique normalized workspace-relative path without traversal or backslashes",
            ));
            continue;
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
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (component_root, allowed_entries) in
        contract.members.iter().map(|member| (&member.root, &member.allowed_entries)).chain(
            contract.adapters.iter().map(|adapter| (&adapter.root, &adapter.allowed_entries)),
        )
    {
        let mut allowed = BTreeSet::new();
        for entry in allowed_entries {
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
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries {
            let entry = match entry {
                Ok(value) => value,
                Err(error) => {
                    diagnostics.push(architecture_error(
                        "ZRYNA-A1205",
                        Some(Path::new(component_root)),
                        format!("component entry could not be inspected: {error}"),
                        "restore directory consistency and retry",
                    ));
                    continue;
                }
            };
            let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                diagnostics.push(architecture_error(
                    "ZRYNA-A1203",
                    entry.path().strip_prefix(root).ok(),
                    "component entry name is not valid UTF-8",
                    "rename the entry with a portable UTF-8 name",
                ));
                continue;
            };
            if IGNORED_DIRECTORY_NAMES.contains(&name.as_str()) {
                continue;
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

fn validate_members(root: &Path, contract: &WorkspaceContract, diagnostics: &mut Vec<Diagnostic>) {
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

    for member in &contract.members {
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

fn validate_adapters(root: &Path, contract: &WorkspaceContract, diagnostics: &mut Vec<Diagnostic>) {
    for adapter in &contract.adapters {
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

fn validate_dependency_graph(contract: &WorkspaceContract, diagnostics: &mut Vec<Diagnostic>) {
    let members: BTreeMap<&str, &MemberContract> =
        contract.members.iter().map(|member| (member.id.as_str(), member)).collect();
    for member in &contract.members {
        for dependency in &member.dependencies {
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

fn validate_bounded_filesystem(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let mut entries_seen = 0usize;
    scan_path(root, root, 0, &mut entries_seen, diagnostics);
}

fn scan_path(
    root: &Path,
    path: &Path,
    depth: usize,
    entries_seen: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if depth > 0
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| IGNORED_DIRECTORY_NAMES.contains(&name))
    {
        return;
    }
    if depth > MAX_SCAN_DEPTH || *entries_seen >= MAX_SCAN_ENTRIES {
        diagnostics.push(architecture_error(
            "ZRYNA-A1204",
            path.strip_prefix(root).ok(),
            "architecture scan exceeded its deterministic safety budget",
            "reduce repository depth or entry count; incomplete scans never pass",
        ));
        return;
    }
    *entries_seen += 1;
    let metadata = match fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(architecture_error(
                "ZRYNA-A1205",
                path.strip_prefix(root).ok(),
                format!("filesystem entry could not be inspected: {error}"),
                "restore a stable readable entry and retry",
            ));
            return;
        }
    };
    if metadata.file_type().is_symlink() {
        diagnostics.push(architecture_error(
            "ZRYNA-A1201",
            path.strip_prefix(root).ok(),
            "symlinks are forbidden inside controlled Zryna components",
            "replace the link with a real in-workspace file or directory",
        ));
        return;
    }
    if metadata.is_file() {
        return;
    }
    if !metadata.is_dir() {
        diagnostics.push(architecture_error(
            "ZRYNA-A1201",
            path.strip_prefix(root).ok(),
            "non-regular filesystem entries are forbidden",
            "remove sockets, devices, and FIFOs from controlled source roots",
        ));
        return;
    }
    let children = match fs::read_dir(path) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(architecture_error(
                "ZRYNA-A1205",
                path.strip_prefix(root).ok(),
                format!("directory scan failed: {error}"),
                "restore read access; incomplete scans never pass",
            ));
            return;
        }
    };
    for child in children {
        match child {
            Ok(entry) => scan_path(root, &entry.path(), depth + 1, entries_seen, diagnostics),
            Err(error) => diagnostics.push(architecture_error(
                "ZRYNA-A1205",
                path.strip_prefix(root).ok(),
                format!("directory entry could not be read: {error}"),
                "restore directory consistency and retry",
            )),
        }
    }
}

fn read_toml(path: &Path, diagnostics: &mut Vec<Diagnostic>) -> Option<toml::Value> {
    let source = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(architecture_error(
                "ZRYNA-A1005",
                Some(path),
                format!("Cargo manifest is unavailable: {error}"),
                "restore the canonical component Cargo.toml",
            ));
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

fn read_json(path: &Path, diagnostics: &mut Vec<Diagnostic>) -> Option<serde_json::Value> {
    let source = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(architecture_error(
                "ZRYNA-A1010",
                Some(path),
                format!("adapter package manifest is unavailable: {error}"),
                "restore the registered adapter package.json",
            ));
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
    use super::{safe_relative_path, valid_id};

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
    fn current_repository_satisfies_the_contract() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let report = super::validate_workspace(&root);
        assert!(report.is_valid(), "architecture diagnostics: {:#?}", report.diagnostics);
    }
}
