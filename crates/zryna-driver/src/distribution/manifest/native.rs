//! The native candidate is a separate installation grammar, without ambient runtimes.

use zryna_diagnostics::Diagnostic;

use super::super::admission_error;
use super::{Distribution, TEXT as LEGACY_TEXT, source_ref_matches_version};

pub(super) const FORMAT: &str = "zryna.native-distribution.v2";
pub(super) const RECIPE_FORMAT: &str = "zryna.native-distribution-recipe.v2";
pub(super) const RECIPE_SHA256: &str =
    "fbc5eb291f8e878e07d3385f5a17c4f29d4bf1542c8ca5f918dea9514702416e";
const SECURITY_RECIPE_SHA256: &str =
    "bd94d51417811098caf0fda18add45070d12cf499cc8a13d3d9d895b24cd5b49";
const TEXT: [&str; 5] = ["LICENSE", "NOTICE", "README.md", "SUPPORT.md", "VERSION"];
const REQUIRED: [&str; 7] = [
    "LICENSE",
    "NOTICE",
    "README.md",
    "SUPPORT.md",
    "VERSION",
    "metadata/materials.json",
    "metadata/architecture-receipt.json",
];

impl Distribution {
    pub(in crate::distribution) fn is_native_provider(&self) -> bool {
        self.format == FORMAT
    }

    pub(in crate::distribution) fn requires_bootstrap(&self) -> bool {
        !self.is_native_provider()
    }

    pub(in crate::distribution) fn node_path(&self) -> Result<&'static str, Diagnostic> {
        if self.is_native_provider() {
            return Err(admission_error("native installation contains no Node.js runtime"));
        }
        Ok(if self.target.triple == "x86_64-pc-windows-msvc" {
            "runtime/node/node.exe"
        } else {
            "runtime/node/bin/node"
        })
    }

    pub(super) fn source_ref_matches(&self) -> bool {
        if self.is_native_provider() {
            source_ref_is_candidate(&self.source.r#ref)
        } else {
            source_ref_matches_version(&self.source.r#ref, &self.version)
        }
    }

    pub(super) fn required_payloads(&self) -> Result<Vec<&'static str>, Diagnostic> {
        if self.is_native_provider() {
            return Ok(required_paths().to_vec());
        }
        Ok(self
            .provider_paths()?
            .iter()
            .copied()
            .chain(LEGACY_TEXT)
            .chain([
                self.node_path()?,
                "metadata/materials.json",
                "metadata/architecture-receipt.json",
            ])
            .collect())
    }

    pub(super) fn validate_native_recipe(&self) -> Result<(), Diagnostic> {
        if self.is_native_provider()
            && ![RECIPE_SHA256, SECURITY_RECIPE_SHA256].contains(&self.recipe.sha256.as_str())
        {
            return Err(admission_error("native installation requires the exact source recipe"));
        }
        Ok(())
    }
}

pub(super) fn source_ref_is_candidate(value: &str) -> bool {
    if value == "refs/heads/main" {
        return true;
    }
    let Some(branch) = value
        .strip_prefix("refs/heads/codex/")
        .or_else(|| value.strip_prefix("refs/heads/fix/"))
        .or_else(|| value.strip_prefix("refs/heads/test/"))
    else {
        return false;
    };
    value.len() <= 200
        && value.split('/').count() <= 12
        && branch.split('/').all(|part| {
            !part.is_empty()
                && part.len() <= 64
                && part.as_bytes()[0].is_ascii_alphanumeric()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase() || b"._-".contains(&b))
                && !part.contains("..")
                && !part.ends_with('.')
                && !part.as_bytes().ends_with(b".lock")
        })
}

pub(super) fn required_paths() -> &'static [&'static str] {
    &REQUIRED
}

pub(super) fn role(path: &str) -> Result<&'static str, Diagnostic> {
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
        return Ok(if path == "LICENSE" { "license" } else { "notice" });
    }
    if path.starts_with("licenses/rust/") && path.split('/').count() == 4 {
        return Ok("license");
    }
    Err(admission_error("path has no admitted native installation role"))
}

#[cfg(test)]
mod tests;
