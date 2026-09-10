//! Exact prepared distribution schema; no filesystem or execution authority is created here.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use zryna_diagnostics::Diagnostic;

use super::{admission_error, wire};

pub(super) const PROVIDERS: [&str; 9] = [
    "lib/zryna/bootstrap/limits-v3.mjs",
    "lib/zryna/bootstrap/limits-v4.mjs",
    "lib/zryna/bootstrap/node_modules/@typescript/old/lib/typescript.js",
    "lib/zryna/bootstrap/node_modules/@typescript/old/package.json",
    "lib/zryna/bootstrap/node_modules/@typescript/typescript6/lib/typescript.js",
    "lib/zryna/bootstrap/node_modules/@typescript/typescript6/package.json",
    "lib/zryna/bootstrap/worker-v3.mjs",
    "lib/zryna/bootstrap/worker-v4.mjs",
    "lib/zryna/bootstrap/worker.mjs",
];

pub(super) const TEXT: [&str; 9] = [
    "LICENSE",
    "NOTICE",
    "README.md",
    "SUPPORT.md",
    "VERSION",
    "licenses/node-LICENSE",
    "licenses/typescript-LICENSE.txt",
    "licenses/typescript-ThirdPartyNoticeText.txt",
    "licenses/typescript6-LICENSE.txt",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct FileRecord {
    pub(super) path: String,
    pub(super) size: u64,
    pub(super) sha256: String,
    pub(super) role: String,
    pub(super) mode: u32,
    pub(super) material: String,
    pub(super) licenses: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Source {
    pub(super) repository: String,
    pub(super) r#ref: String,
    pub(super) commit: String,
    pub(super) tree: String,
    pub(super) source_date_epoch: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Target {
    pub(super) triple: String,
    pub(super) archive_format: String,
    pub(super) platform_baseline: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Recipe {
    pub(super) format: String,
    pub(super) sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Distribution {
    pub(super) format: String,
    pub(super) version: String,
    pub(super) source: Source,
    pub(super) target: Target,
    pub(super) recipe: Recipe,
    pub(super) files: Vec<FileRecord>,
}

impl Distribution {
    pub(super) fn parse(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = wire::parse(bytes)?;
        value.validate()?;
        Ok(value)
    }

    pub(super) fn node_path(&self) -> &'static str {
        if self.target.triple == "x86_64-pc-windows-msvc" {
            "runtime/node/node.exe"
        } else {
            "runtime/node/bin/node"
        }
    }

    pub(super) fn cli_path(&self) -> &'static str {
        if self.target.triple == "x86_64-pc-windows-msvc" { "bin/zryna.exe" } else { "bin/zryna" }
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.format != "zryna.distribution.v1"
            || self.version != env!("CARGO_PKG_VERSION")
            || self.source.repository != "https://github.com/zryna/zryna"
            || self.source.r#ref != format!("refs/tags/v{}", self.version)
            || !hex(&self.source.commit, 40)
            || !hex(&self.source.tree, 40)
            || self.source.source_date_epoch > u64::from(u32::MAX)
            || self.recipe.format != "zryna.distribution-recipe.v1"
            || !hex(&self.recipe.sha256, 64)
        {
            return Err(admission_error("distribution source, version or recipe mismatch"));
        }
        self.validate_target()?;
        validate_files(&self.files, self)?;
        if self.files.len() > 508 {
            return Err(admission_error("complete archive would exceed 512 regular files"));
        }
        let paths = self.files.iter().map(|file| file.path.as_str()).collect::<BTreeSet<_>>();
        for path in PROVIDERS.into_iter().chain(TEXT).chain([
            self.node_path(),
            "metadata/materials.json",
            "metadata/architecture-receipt.json",
        ]) {
            if !paths.contains(path) {
                return Err(admission_error("required installed payload is missing"));
            }
        }
        if !paths.iter().any(|path| path.starts_with("licenses/rust/"))
            || paths.contains(self.cli_path())
            || paths.contains("metadata/distribution.json")
            || paths.contains("metadata/inventory.json")
            || paths.contains("metadata/checksums.sha256")
        {
            return Err(admission_error("distribution payload is incomplete or cyclic"));
        }
        Ok(())
    }

    fn validate_target(&self) -> Result<(), Diagnostic> {
        let (triple, format, baseline): (&str, &str, &[(&str, &str)]) =
            if cfg!(target_os = "windows") {
                (
                    "x86_64-pc-windows-msvc",
                    "zip",
                    &[
                        ("architecture", "x86_64"),
                        ("os", "windows"),
                        ("product", "windows-server"),
                        ("runtime", "operating-system-ucrt"),
                        ("version", "2022"),
                    ],
                )
            } else {
                (
                    "x86_64-unknown-linux-gnu",
                    "tar-gzip",
                    &[
                        ("architecture", "x86_64"),
                        ("distribution", "ubuntu"),
                        ("os", "linux"),
                        ("version", "24.04"),
                    ],
                )
            };
        let expected = baseline
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<_, _>>();
        if !cfg!(target_arch = "x86_64")
            || !cfg!(any(target_os = "linux", target_os = "windows"))
            || self.target.triple != triple
            || self.target.archive_format != format
            || self.target.platform_baseline != expected
        {
            return Err(admission_error("distribution target does not match this compiler"));
        }
        Ok(())
    }
}

pub(super) fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn portable(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 200
        && path.split('/').count() <= 12
        && path.split('/').all(|part| {
            let Some(first) = part.bytes().next() else { return false };
            let reserved = part.split('.').next().unwrap_or_default().to_ascii_lowercase();
            (first.is_ascii_alphanumeric() || first == b'@')
                && part.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"@+._-".contains(&byte))
                && !part.ends_with('.')
                && !["con", "prn", "aux", "nul"].contains(&reserved.as_str())
                && !(reserved.len() == 4
                    && (reserved.starts_with("com") || reserved.starts_with("lpt"))
                    && reserved.as_bytes()[3].is_ascii_digit())
        })
}

pub(super) fn validate_files(
    files: &[FileRecord],
    distribution: &Distribution,
) -> Result<(), Diagnostic> {
    if files.len() > 512 {
        return Err(admission_error("distribution file count exceeds 512"));
    }
    let mut previous = "";
    let mut prefixes = BTreeMap::new();
    let mut paths = BTreeSet::new();
    let mut total = 0_u64;
    for file in files {
        if !portable(&file.path) || file.path.as_str() <= previous || !hex(&file.sha256, 64) {
            return Err(admission_error("invalid, unordered or duplicate distribution path"));
        }
        previous = &file.path;
        let parts = file.path.split('/').collect::<Vec<_>>();
        for index in 1..=parts.len() {
            let prefix = parts[..index].join("/");
            let folded = prefix.to_ascii_lowercase();
            if prefixes.get(&folded).is_some_and(|value| value != &prefix)
                || (index < parts.len() && paths.contains(&folded))
            {
                return Err(admission_error("distribution path collision"));
            }
            prefixes.insert(folded, prefix);
        }
        paths.insert(file.path.to_ascii_lowercase());
        let role = role(&file.path, distribution)?;
        let executable = role == "cli" || role == "runtime";
        let limit = if executable {
            256 * 1024 * 1024
        } else if role == "provider" {
            16 * 1024 * 1024
        } else {
            2 * 1024 * 1024
        };
        if file.role != role
            || file.mode != if executable { 0o755 } else { 0o644 }
            || file.size == 0
            || file.size > limit
            || file.material.is_empty()
            || file.material.len() > 64
            || !file
                .material
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        {
            return Err(admission_error("distribution file role, mode or resource bound mismatch"));
        }
        total += file.size;
        if total > 512 * 1024 * 1024 {
            return Err(admission_error("distribution expansion exceeds 536870912 bytes"));
        }
        if file.licenses.is_empty()
            || file.licenses.len() > 512
            || file.licenses.windows(2).any(|pair| pair[0] >= pair[1])
            || file
                .licenses
                .iter()
                .any(|path| !files.iter().any(|item| &item.path == path && item.role == "license"))
        {
            return Err(admission_error("distribution license references are incomplete"));
        }
    }
    Ok(())
}

fn role(path: &str, distribution: &Distribution) -> Result<&'static str, Diagnostic> {
    if path == distribution.cli_path() {
        return Ok("cli");
    }
    if path == distribution.node_path() {
        return Ok("runtime");
    }
    if PROVIDERS.contains(&path) {
        return Ok("provider");
    }
    if [
        "metadata/architecture-receipt.json",
        "metadata/materials.json",
        "metadata/distribution.json",
        "metadata/inventory.json",
        "metadata/checksums.sha256",
    ]
    .contains(&path)
    {
        return Ok("metadata");
    }
    if TEXT.contains(&path) {
        return Ok(if path == "LICENSE" || path.starts_with("licenses/") {
            "license"
        } else {
            "notice"
        });
    }
    if path.starts_with("licenses/rust/") && path.split('/').count() == 4 {
        return Ok("license");
    }
    Err(admission_error("path has no admitted distribution role"))
}
