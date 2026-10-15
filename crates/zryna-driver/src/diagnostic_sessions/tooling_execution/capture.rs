use std::path::Path;

#[cfg(test)]
use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use same_file::Handle;
use zryna_diagnostics::Diagnostic;

use super::execution_error;

#[cfg(test)]
mod aggregate_tests;
mod pins;
mod source_io;
mod v4;

pub(super) use v4::{CapturedV4, V4_MODULES, V4Layout};

use pins::{require_digest, validate_graph};
pub(super) use source_io::capture_absolute;
use source_io::{cap_metadata_is_link_or_reparse, capture_file, directory_identity, open_dir};
#[cfg(test)]
use source_io::{metadata_is_link_or_reparse, open_regular, source_changed};

const MAX_WORKER_BYTES: usize = 128 * 1_024;
const MAX_MANIFEST_BYTES: usize = 8 * 1_024;
const MAX_WRAPPER_BYTES: usize = 1_024;
const MAX_TYPESCRIPT_BYTES: usize = 12 * 1_024 * 1_024;
const MAX_CLOSURE_BYTES: usize = 16 * 1_024 * 1_024;
const WRAPPER_MANIFEST_SHA256: &str =
    "d9b8fb53c67c947fece83bcb73fa8512227ea6a12b110e096fdab8ecdc02b655";
const WRAPPER_SHA256: &str = "d3f3cd2b04b7f466f4484df921b744223f7bd1f3e353ec9110bdf52695b983d5";
const TYPESCRIPT_MANIFEST_SHA256: &str =
    "9332e97c30d3e53ed54910b89207ed657fb444066484df6e5b6965bf130865e9";
const TYPESCRIPT_SHA256: &str = "569177652966bd528c319171c7dd22860dbf72bde116cbc4f644f1d02bb12e39";

const WORKER: &[&str] = &["adapters", "typescript-6", "src", "worker.mjs"];
const WORKER_V3: &[&str] = &["adapters", "typescript-6", "src", "worker-v3.mjs"];
const LIMITS_V3: &[&str] = &["adapters", "typescript-6", "src", "limits-v3.mjs"];
const WORKER_V4: &[&str] = &["adapters", "typescript-6", "src", "worker-v4.mjs"];
const LIMITS_V4: &[&str] = &["adapters", "typescript-6", "src", "limits-v4.mjs"];
const WRAPPER_MANIFEST: &[&str] = &[
    "node_modules",
    ".pnpm",
    "@typescript+typescript6@6.0.2",
    "node_modules",
    "@typescript",
    "typescript6",
    "package.json",
];
const WRAPPER: &[&str] = &[
    "node_modules",
    ".pnpm",
    "@typescript+typescript6@6.0.2",
    "node_modules",
    "@typescript",
    "typescript6",
    "lib",
    "typescript.js",
];
const TYPESCRIPT_MANIFEST: &[&str] =
    &["node_modules", ".pnpm", "typescript@6.0.3", "node_modules", "typescript", "package.json"];
const TYPESCRIPT: &[&str] = &[
    "node_modules",
    ".pnpm",
    "typescript@6.0.3",
    "node_modules",
    "typescript",
    "lib",
    "typescript.js",
];

pub(super) struct CapturedFile {
    pub(super) bytes: Vec<u8>,
    pub(super) sha256: [u8; 32],
}

pub(super) struct CapturedToolingClosure {
    pub(super) worker: CapturedFile,
    pub(super) worker_v3: CapturedFile,
    pub(super) limits_v3: CapturedFile,
    pub(super) worker_v4: CapturedFile,
    pub(super) limits_v4: CapturedFile,
    pub(super) v4: CapturedV4,
    pub(super) wrapper_manifest: CapturedFile,
    pub(super) wrapper: CapturedFile,
    pub(super) typescript_manifest: CapturedFile,
    pub(super) typescript: CapturedFile,
}

impl CapturedToolingClosure {
    pub(super) fn capture_installed(root: &Path) -> Result<Self, Diagnostic> {
        if !root.is_absolute() {
            return Err(execution_error("installed tooling root must be absolute"));
        }
        let root = capture_absolute(root)?;
        let bootstrap = open_dir(&root, &["lib", "zryna", "bootstrap"])?;
        let worker = capture_file(&bootstrap, &["worker.mjs"], MAX_WORKER_BYTES)?;
        let expected_worker = include_bytes!("../../../../../adapters/typescript-6/src/worker.mjs");
        if worker.bytes != expected_worker {
            return Err(execution_error("installed worker differs from this tooling build"));
        }
        let worker_v3 = capture_file(&bootstrap, &["worker-v3.mjs"], MAX_WORKER_BYTES)?;
        let limits_v3 = capture_file(&bootstrap, &["limits-v3.mjs"], MAX_WORKER_BYTES)?;
        verify_v3_workers(&worker_v3, &limits_v3)?;
        let worker_v4 = capture_file(&bootstrap, &["worker-v4.mjs"], MAX_WORKER_BYTES)?;
        let limits_v4 = capture_file(&bootstrap, &["limits-v4.mjs"], MAX_WORKER_BYTES)?;
        let v4 = v4::capture_installed(&bootstrap, &worker_v4, &limits_v4)?;
        let wrapper_manifest = capture_file(
            &bootstrap,
            &["node_modules", "@typescript", "typescript6", "package.json"],
            MAX_MANIFEST_BYTES,
        )?;
        let wrapper = capture_file(
            &bootstrap,
            &["node_modules", "@typescript", "typescript6", "lib", "typescript.js"],
            MAX_WRAPPER_BYTES,
        )?;
        let typescript_manifest = capture_file(
            &bootstrap,
            &["node_modules", "@typescript", "old", "package.json"],
            MAX_MANIFEST_BYTES,
        )?;
        let typescript = capture_file(
            &bootstrap,
            &["node_modules", "@typescript", "old", "lib", "typescript.js"],
            MAX_TYPESCRIPT_BYTES,
        )?;
        require_digest(&wrapper_manifest, WRAPPER_MANIFEST_SHA256, "compatibility manifest")?;
        require_digest(&wrapper, WRAPPER_SHA256, "compatibility wrapper")?;
        require_digest(
            &typescript_manifest,
            TYPESCRIPT_MANIFEST_SHA256,
            "implementation manifest",
        )?;
        require_digest(&typescript, TYPESCRIPT_SHA256, "TypeScript runtime")?;
        validate_graph(&wrapper_manifest.bytes, &wrapper.bytes, &typescript_manifest.bytes)?;
        let captured = Self {
            worker,
            worker_v3,
            limits_v3,
            worker_v4,
            limits_v4,
            v4,
            wrapper_manifest,
            wrapper,
            typescript_manifest,
            typescript,
        };
        captured.validate_size()?;
        Ok(captured)
    }

    pub(super) fn capture(root: &Path) -> Result<Self, Diagnostic> {
        if !root.is_absolute() {
            return Err(execution_error("tooling compiler root must be absolute"));
        }
        let root_path = root;
        let root = capture_absolute(root_path)?;
        authenticate_adapter_link(&root, root_path)?;
        let worker = capture_file(&root, WORKER, MAX_WORKER_BYTES)?;
        let worker_v3 = capture_file(&root, WORKER_V3, MAX_WORKER_BYTES)?;
        let limits_v3 = capture_file(&root, LIMITS_V3, MAX_WORKER_BYTES)?;
        verify_v3_workers(&worker_v3, &limits_v3)?;
        let worker_v4 = capture_file(&root, WORKER_V4, MAX_WORKER_BYTES)?;
        let limits_v4 = capture_file(&root, LIMITS_V4, MAX_WORKER_BYTES)?;
        let v4 = v4::capture(&root, &WORKER_V4[..WORKER_V4.len() - 1], &worker_v4, &limits_v4)?;
        let wrapper_manifest = capture_file(&root, WRAPPER_MANIFEST, MAX_MANIFEST_BYTES)?;
        let wrapper = capture_file(&root, WRAPPER, MAX_WRAPPER_BYTES)?;
        let typescript_manifest = capture_file(&root, TYPESCRIPT_MANIFEST, MAX_MANIFEST_BYTES)?;
        let typescript = capture_file(&root, TYPESCRIPT, MAX_TYPESCRIPT_BYTES)?;

        validate_graph(&wrapper_manifest.bytes, &wrapper.bytes, &typescript_manifest.bytes)?;
        require_digest(
            &wrapper_manifest,
            WRAPPER_MANIFEST_SHA256,
            "TypeScript compatibility manifest",
        )?;
        require_digest(&wrapper, WRAPPER_SHA256, "TypeScript compatibility wrapper")?;
        require_digest(
            &typescript_manifest,
            TYPESCRIPT_MANIFEST_SHA256,
            "TypeScript implementation manifest",
        )?;
        require_digest(&typescript, TYPESCRIPT_SHA256, "TypeScript 6.0.3 runtime bundle")?;
        let captured = Self {
            worker,
            worker_v3,
            limits_v3,
            worker_v4,
            limits_v4,
            v4,
            wrapper_manifest,
            wrapper,
            typescript_manifest,
            typescript,
        };
        captured.validate_size()?;
        Ok(captured)
    }

    pub(super) fn validate_size(&self) -> Result<(), Diagnostic> {
        validate_sizes(
            [
                self.worker.bytes.len(),
                self.worker_v3.bytes.len(),
                self.limits_v3.bytes.len(),
                self.worker_v4.bytes.len(),
                self.limits_v4.bytes.len(),
                self.wrapper_manifest.bytes.len(),
                self.wrapper.bytes.len(),
                self.typescript_manifest.bytes.len(),
                self.typescript.bytes.len(),
            ]
            .into_iter()
            .chain(self.v4.modules().iter().map(|file| file.bytes.len())),
        )
    }
}

pub(super) fn validate_sizes(sizes: impl IntoIterator<Item = usize>) -> Result<(), Diagnostic> {
    let total = sizes
        .into_iter()
        .try_fold(0_usize, usize::checked_add)
        .ok_or_else(|| execution_error("tooling executable closure byte count overflowed"))?;
    if total > MAX_CLOSURE_BYTES {
        return Err(execution_error("tooling executable closure exceeds its byte limit"));
    }
    Ok(())
}

fn verify_v3_workers(worker: &CapturedFile, limits: &CapturedFile) -> Result<(), Diagnostic> {
    if worker.bytes != include_bytes!("../../../../../adapters/typescript-6/src/worker-v3.mjs")
        || limits.bytes != include_bytes!("../../../../../adapters/typescript-6/src/limits-v3.mjs")
    {
        return Err(execution_error("protocol-v3 tooling worker differs from this tooling build"));
    }
    Ok(())
}

fn authenticate_adapter_link(root: &Dir, root_path: &Path) -> Result<(), Diagnostic> {
    let expected = open_dir(root, &WRAPPER_MANIFEST[..WRAPPER_MANIFEST.len() - 1])?;
    let adapter = root
        .open_dir("adapters")
        .and_then(|dir| dir.open_dir("typescript-6"))
        .and_then(|dir| dir.open_dir("node_modules"))
        .and_then(|dir| dir.open_dir("@typescript"))
        .map_err(|_| execution_error("TypeScript adapter dependency link is unavailable"))?;
    let metadata = adapter
        .symlink_metadata("typescript6")
        .map_err(|_| execution_error("TypeScript adapter dependency link is unavailable"))?;
    if cap_metadata_is_link_or_reparse(&metadata) {
        let linked_path =
            root_path.join("adapters/typescript-6/node_modules/@typescript/typescript6");
        let linked = Handle::from_path(&linked_path).map_err(|_| {
            execution_error("TypeScript adapter dependency link cannot be resolved")
        })?;
        if linked != directory_identity(&expected)? {
            return Err(execution_error(
                "TypeScript adapter dependency link resolved outside the pin",
            ));
        }
    } else {
        #[cfg(not(test))]
        return Err(execution_error(
            "TypeScript adapter dependency must be the pinned pnpm package link",
        ));
        #[cfg(test)]
        {
            let linked = adapter.open_dir_nofollow("typescript6").map_err(|_| {
                execution_error("TypeScript adapter dependency entry cannot be resolved")
            })?;
            let linked_state =
                directory_identity(&linked)?.as_file().metadata().map_err(|_| source_changed())?;
            if !linked_state.is_dir() || metadata_is_link_or_reparse(&linked_state) {
                return Err(source_changed());
            }
            let expected_manifest = open_regular(&expected, "package.json")?;
            let linked_manifest = open_regular(&linked, "package.json")?;
            let expected_lib = expected.open_dir_nofollow("lib").map_err(|_| source_changed())?;
            let linked_lib = linked.open_dir_nofollow("lib").map_err(|_| source_changed())?;
            let expected_runtime = open_regular(&expected_lib, "typescript.js")?;
            let linked_runtime = open_regular(&linked_lib, "typescript.js")?;
            if linked_manifest != expected_manifest || linked_runtime != expected_runtime {
                return Err(execution_error(
                    "TypeScript adapter dependency files are not the pinned package identities",
                ));
            }
        }
    }
    Ok(())
}
