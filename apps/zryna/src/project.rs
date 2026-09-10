use std::{
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

use serde_json::json;
use sha2::{Digest as _, Sha256};
use zryna_package::{
    LockMode, PackageFile, PackageMaterial, PackageSource, PackageSourceKind,
    PackageSourceProvider, ResolveError,
};

use crate::project_filesystem;

const SOURCE_PATH: &str = "src/main.zry";
const SOURCE_BYTES: &[u8] = b"export function main(): i32 {\n  return 42;\n}\n";

#[derive(Clone, Debug, clap::Args)]
pub(crate) struct NewOptions {
    /// New project directory; its final component becomes the package name.
    path: PathBuf,
    /// Emit one machine-readable JSON response.
    #[arg(long)]
    json: bool,
}

pub(crate) fn create(options: &NewOptions) -> ExitCode {
    match scaffold(&options.path) {
        Ok(path) => {
            if options.json {
                println!(
                    "{}",
                    json!({
                        "version": 1,
                        "ok": true,
                        "command": "new",
                        "project": path,
                    })
                );
            } else {
                println!("created {}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            if options.json {
                println!(
                    "{}",
                    json!({
                        "version": 1,
                        "ok": false,
                        "command": "new",
                        "project": null,
                        "diagnostics": [{ "code": error.code, "message": error.message }],
                    })
                );
            } else {
                eprintln!("error[{}]: {}", error.code, error.message);
            }
            ExitCode::from(error.exit)
        }
    }
}

fn scaffold(requested: &Path) -> Result<PathBuf, ProjectError> {
    let destination = absolute_destination(requested)?;
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ProjectError::request("project name must be portable UTF-8"))?;
    let source = PackageSource {
        kind: PackageSourceKind::Local,
        locator: name.to_owned(),
        revision: String::new(),
    };
    zryna_package::validate_source(&source)
        .map_err(|_| ProjectError::request("project name must be lowercase and portable"))?;
    let (manifest, lock) = scaffold_records(&source)?;
    project_filesystem::publish(&destination, name, &manifest, &lock, SOURCE_BYTES)?;
    Ok(destination)
}

fn scaffold_records(source: &PackageSource) -> Result<(Vec<u8>, Vec<u8>), ProjectError> {
    let manifest = canonical_json(&json!({
        "format": "zryna.package.v1",
        "name": source.locator,
        "version": "0.1.0",
        "source": source,
        "compatibility": {
            "compiler": env!("CARGO_PKG_VERSION"),
            "profile": "i32-v1",
            "targets": ["javascript", "native-linux-x86_64", "webassembly"],
        },
        "files": [{
            "path": SOURCE_PATH,
            "size": SOURCE_BYTES.len(),
            "sha256": format!("{:x}", Sha256::digest(SOURCE_BYTES)),
        }],
        "dependencies": [],
    }))?;
    let mut provider = ScaffoldProvider {
        source: source.clone(),
        material: Some(PackageMaterial {
            manifest: manifest.clone(),
            files: vec![PackageFile { path: SOURCE_PATH.to_owned(), bytes: SOURCE_BYTES.to_vec() }],
        }),
    };
    let graph = zryna_package::resolve(&mut provider, source.clone(), LockMode::Update)
        .map_err(|error| ProjectError::authority(&error))?;
    Ok((manifest, graph.lock_bytes().to_vec()))
}

fn canonical_json(value: &serde_json::Value) -> Result<Vec<u8>, ProjectError> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|_| ProjectError::authority(&ResolveError::source("manifest serialization")))?;
    bytes.push(b'\n');
    Ok(bytes)
}

struct ScaffoldProvider {
    source: PackageSource,
    material: Option<PackageMaterial>,
}

impl PackageSourceProvider for ScaffoldProvider {
    fn load(&mut self, source: &PackageSource) -> Result<PackageMaterial, ResolveError> {
        if source != &self.source {
            return Err(ResolveError::source("scaffold requested an unexpected package source"));
        }
        self.material
            .take()
            .ok_or_else(|| ResolveError::source("scaffold package was requested more than once"))
    }
}

fn absolute_destination(path: &Path) -> Result<PathBuf, ProjectError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProjectError::request(
            "project destination must not contain current- or parent-directory components",
        ));
    }
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|root| root.join(path))
            .map_err(|_| ProjectError::request("current directory is unavailable"))
    }
}

#[derive(Debug)]
pub(super) struct ProjectError {
    pub(super) code: &'static str,
    pub(super) message: String,
    pub(super) exit: u8,
}

impl ProjectError {
    pub(super) fn request(message: impl Into<String>) -> Self {
        Self { code: "ZRYNA-C2001", message: message.into(), exit: 2 }
    }

    pub(super) fn collision(message: impl Into<String>) -> Self {
        Self { code: "ZRYNA-C2002", message: message.into(), exit: 4 }
    }

    pub(super) fn publication(message: impl Into<String>) -> Self {
        Self { code: "ZRYNA-C2003", message: message.into(), exit: 4 }
    }

    pub(super) fn cleanup(message: impl Into<String>) -> Self {
        Self { code: "ZRYNA-C2004", message: message.into(), exit: 6 }
    }

    fn authority(error: &ResolveError) -> Self {
        Self { code: error.code(), message: error.detail().to_owned(), exit: 70 }
    }
}
