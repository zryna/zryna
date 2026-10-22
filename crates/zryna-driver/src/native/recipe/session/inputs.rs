//! Map the retained authenticated plan into the one supported fixed compiler layout.
use super::super::{approval::ApprovedNativeRecipe, archive};
use super::config::{self, ENV};
use super::toolroot::{self, NativeToolRootApproval};
use super::{Result, fail};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;

pub(super) fn prepare(
    approved: &ApprovedNativeRecipe,
    root: &NativeToolRootApproval,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let plan = approved.plan();
    let configuration = plan.configuration();
    if approved.operator().tool_root_sha256 != toolroot::identity(root)?
        || approved.operator().image_sha256 != root.image_sha256
        || configuration.execution_policy != config::policy_identity()
        || plan.step().tool != "gcc"
        || plan.step().output != "adapter"
        || plan.output_path() != "native/adapter.o"
    {
        return Err(fail("approved recipe does not match the supported host mechanism"));
    }
    let expected_env: BTreeMap<_, _> =
        ENV.iter().filter_map(|entry| entry.split_once('=')).collect();
    let actual_env: BTreeMap<_, _> = configuration
        .host
        .environment
        .iter()
        .map(|entry| (entry.name.as_str(), entry.value.as_str()))
        .collect();
    if actual_env != expected_env
        || actual_env.len() != configuration.host.environment.len()
        || root.tools.iter().any(|(path, hash)| {
            let name = path.rsplit('/').next().unwrap_or("");
            !configuration.host_tools.iter().any(|tool| {
                tool.name == name
                    && tool.sha256 == *hash
                    && tool.runs_on == "x86_64-unknown-linux-gnu"
                    && tool
                        .targets
                        .contains(&crate::package_build::BuildTargetId::NativeLinuxX86_64)
            })
        })
    {
        return Err(fail("exact compiler and complete environment authority required"));
    }
    let mut files = BTreeMap::from([("library.c".to_owned(), plan.source_bytes().to_vec())]);
    for material in &plan.step().sysroots {
        for (path, body) in &material.files {
            if !matches!(path.as_str(), "library.h" | "sqlite3.h")
                || files.insert(path.clone(), body.clone()).is_some()
            {
                return Err(fail(
                    "only exact declared fixed-layout readonly headers are supported",
                ));
            }
        }
    }
    if files.len() != 3
        || files.values().any(Vec::is_empty)
        || files.values().map(Vec::len).sum::<usize>() > 2 * 1024 * 1024 - 4096
    {
        return Err(fail("complete fixed-layout input inventory exceeds volume bound"));
    }
    let mut total = files.values().map(Vec::len).sum::<usize>();
    let mut readonly = total;
    let mut materials = files.len();
    for (path, entry) in &root.entries {
        if let archive::RootKind::File { bytes, .. } = entry.kind {
            if bytes != 0 {
                materials += 1;
            }
            total =
                total.checked_add(bytes).ok_or_else(|| fail("aggregate material size overflow"))?;
            if !root.tools.contains_key(path) {
                readonly = readonly
                    .checked_add(bytes)
                    .ok_or_else(|| fail("readonly material size overflow"))?;
            }
        }
    }
    if total > 128 * 1024 * 1024 || readonly > 32 * 1024 * 1024 || materials > 64 {
        return Err(fail("aggregate root and input material bounds exceeded"));
    }
    let library = plan
        .requirements()
        .libraries()
        .first()
        .ok_or_else(|| fail("original FFI library issuer absent"))?;
    if <[u8; 32]>::from(Sha256::digest(&files["library.h"])) != *library.header_sha256() {
        return Err(fail("actual compile header differs from original FFI issuer"));
    }
    Ok(files)
}
pub(super) fn expected_input(files: &BTreeMap<String, Vec<u8>>) -> archive::Expected {
    let metadata = archive::Metadata { uid: 0, gid: 0, mode: 0o444 };
    archive::Expected {
        root: archive::Root::Input,
        directory: archive::Metadata { mode: 0o555, ..metadata },
        files: files
            .iter()
            .map(|(name, body)| {
                (
                    name.clone(),
                    archive::FileIdentity {
                        metadata,
                        bytes: body.len(),
                        sha256: Sha256::digest(body).into(),
                    },
                )
            })
            .collect(),
    }
}
pub(super) fn expected_output(approved: &ApprovedNativeRecipe) -> archive::Expected {
    archive::Expected {
        root: archive::Root::Output,
        directory: archive::Metadata { uid: 65534, gid: 65534, mode: 0o700 },
        files: BTreeMap::from([(
            "adapter.o".to_owned(),
            archive::FileIdentity {
                metadata: archive::Metadata { uid: 65534, gid: 65534, mode: 0o644 },
                bytes: approved.operator().output_bytes,
                sha256: approved.operator().output_sha256,
            },
        )]),
    }
}
