//! Complete post-build metadata coverage, without a self-hashing inventory cycle.

use serde::Deserialize;
use zryna_diagnostics::Diagnostic;

use super::{
    admission_error,
    filesystem::InstallationTree,
    manifest::{Distribution, FileRecord, validate_files},
    wire,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileList {
    format: String,
    files: Vec<FileRecord>,
}

pub(super) fn validate_metadata(
    tree: &mut InstallationTree,
    distribution: &Distribution,
) -> Result<(), Diagnostic> {
    let inventory: FileList = wire::parse(tree.bytes("metadata/inventory.json")?)?;
    if inventory.format != "zryna.distribution-inventory.v1" {
        return Err(admission_error("installed inventory format mismatch"));
    }
    validate_files(&inventory.files, distribution)?;
    let cli = inventory
        .files
        .iter()
        .find(|file| file.path == distribution.cli_path())
        .ok_or_else(|| admission_error("installed CLI is absent from inventory"))?;
    tree.capture_file(distribution.cli_path(), cli.size, false)?;
    let mut expected = distribution.files.clone();
    expected.push(FileRecord {
        path: distribution.cli_path().to_owned(),
        size: tree.size(distribution.cli_path())?,
        sha256: tree.digest(distribution.cli_path())?.to_owned(),
        role: "cli".to_owned(),
        mode: 0o755,
        material: "source".to_owned(),
        licenses: distribution
            .files
            .iter()
            .filter(|file| {
                file.role == "license"
                    && (file.path == "LICENSE" || file.path.starts_with("licenses/rust/"))
            })
            .map(|file| file.path.clone())
            .collect(),
    });
    expected.push(FileRecord {
        path: "metadata/distribution.json".to_owned(),
        size: tree.size("metadata/distribution.json")?,
        sha256: tree.digest("metadata/distribution.json")?.to_owned(),
        role: "metadata".to_owned(),
        mode: 0o644,
        material: "source".to_owned(),
        licenses: vec!["LICENSE".to_owned()],
    });
    expected.sort_by(|left, right| left.path.cmp(&right.path));
    if inventory.files != expected {
        return Err(admission_error("installed inventory differs from build-bound payload"));
    }
    for record in &inventory.files {
        tree.matches(record)?;
    }
    let mut checksums = inventory
        .files
        .iter()
        .map(|file| (file.path.clone(), file.sha256.clone()))
        .collect::<Vec<_>>();
    checksums.push((
        "metadata/inventory.json".to_owned(),
        tree.digest("metadata/inventory.json")?.to_owned(),
    ));
    checksums.sort_by(|left, right| left.0.cmp(&right.0));
    let expected =
        checksums.iter().map(|(path, digest)| format!("{digest}  {path}\n")).collect::<String>();
    if tree.bytes("metadata/checksums.sha256")? != expected.as_bytes() {
        return Err(admission_error("installed checksum coverage mismatch"));
    }
    let materials: FileList = wire::parse(tree.bytes("metadata/materials.json")?)?;
    if materials.format != "zryna.distribution-materials.v1"
        || materials.files
            != distribution
                .files
                .iter()
                .filter(|file| file.role != "metadata")
                .cloned()
                .collect::<Vec<_>>()
    {
        return Err(admission_error("installed material record differs from build-bound payload"));
    }
    source_receipt(tree.bytes("metadata/architecture-receipt.json")?, distribution)?;
    if tree.bytes("VERSION")? != format!("{}\n", distribution.version).as_bytes() {
        return Err(admission_error("installed VERSION differs from compiled version"));
    }
    Ok(())
}

fn source_receipt(bytes: &[u8], distribution: &Distribution) -> Result<(), Diagnostic> {
    use serde_json::{Value, json};
    let receipt: Value = wire::parse(bytes)?;
    exact_keys(&receipt, &["format", "source", "command", "toolchain", "inputs", "report"])?;
    if receipt["format"] != "zryna.source-build-receipt.v1"
        || receipt["source"]
            != json!({ "repository": distribution.source.repository,
            "commit": distribution.source.commit, "tree": distribution.source.tree })
        || receipt["command"]
            != json!([
                "cargo",
                "run",
                "--locked",
                "-p",
                "zryna",
                "--",
                "architecture",
                "check",
                "--json"
            ])
        || receipt["report"] != json!({ "diagnostics": [] })
    {
        return Err(admission_error("installed source architecture receipt mismatch"));
    }
    exact_keys(&receipt["toolchain"], &["channel", "cargoVersion", "rustcVersion"])?;
    if receipt["toolchain"]["channel"] != "1.97.1" {
        return Err(admission_error("installed source toolchain mismatch"));
    }
    for tool in ["cargo", "rustc"] {
        let version = receipt["toolchain"][format!("{tool}Version")]
            .as_str()
            .ok_or_else(|| admission_error("installed source toolchain version is absent"))?;
        if !version.starts_with(&format!("{tool} 1.97.1 "))
            || version.len() > 96
            || !version.bytes().all(|byte| (32..=126).contains(&byte))
        {
            return Err(admission_error("installed source toolchain version mismatch"));
        }
    }
    let inputs =
        receipt["inputs"].as_array().ok_or_else(|| admission_error("source inputs absent"))?;
    let paths = ["Cargo.lock", "Cargo.toml", "rust-toolchain.toml", "zryna.workspace.json"];
    if inputs.len() != paths.len() {
        return Err(admission_error("source input inventory mismatch"));
    }
    for (input, path) in inputs.iter().zip(paths) {
        exact_keys(input, &["logicalPath", "size", "sha256"])?;
        if input["logicalPath"] != path
            || !input["size"].as_u64().is_some_and(|size| (1..=262_144).contains(&size))
            || !input["sha256"].as_str().is_some_and(|digest| super::manifest::hex(digest, 64))
        {
            return Err(admission_error("source input identity mismatch"));
        }
    }
    Ok(())
}

fn exact_keys(value: &serde_json::Value, expected: &[&str]) -> Result<(), Diagnostic> {
    let object =
        value.as_object().ok_or_else(|| admission_error("source receipt object absent"))?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err(admission_error("source receipt keys mismatch"));
    }
    Ok(())
}
