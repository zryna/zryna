//! Fixed immutable-image manifest and complete sealed material-volume audit.

use cap_std::{ambient_authority, fs::Dir};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Write;

mod filesystem;
mod manifest;
#[cfg(test)]
mod tests;

const ERROR: &str = "PLAYGROUND-MATERIAL-AUDIT";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    version: u32,
    root_sha256: String,
    files: usize,
    bytes: u64,
}

pub(super) fn entry() -> std::process::ExitCode {
    let result = (|| {
        let root = Dir::open_ambient_dir("/", ambient_authority()).map_err(|_| ERROR)?;
        let resources = filesystem::child(&root, "resources")?;
        let captured = filesystem::read_manifest(&resources)?;
        let materials = filesystem::child(&root, "materials")?;
        let receipt = audit(&materials, &captured.bytes, 0, 0)?;
        captured.revalidate(&resources)?;
        // Recheck both named roots after every data byte has been authenticated.
        filesystem::same_directory(&materials, &filesystem::child(&root, "materials")?)?;
        filesystem::same_directory(&resources, &filesystem::child(&root, "resources")?)?;
        let mut output = std::io::stdout().lock();
        serde_json::to_writer(&mut output, &receipt).map_err(|_| ERROR)?;
        output.write_all(b"\n").map_err(|_| ERROR)
    })();
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("{reason}");
            std::process::ExitCode::from(1)
        }
    }
}

fn audit(root: &Dir, bytes: &[u8], uid: u32, gid: u32) -> Result<Receipt, &'static str> {
    let manifest = manifest::parse(bytes)?;
    filesystem::walk(root, "", &manifest, uid, gid)?;
    Ok(Receipt {
        version: 1,
        root_sha256: format!("{:x}", Sha256::digest(bytes)),
        files: manifest.files.len(),
        bytes: manifest.files.iter().map(|file| file.bytes).sum(),
    })
}
