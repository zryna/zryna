use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cap_fs_ext::DirExt as _;
use serde::Serialize;
use sha2::{Digest as _, Sha256};

use super::{
    PACKAGE_BUILD_MANIFEST_NAME, PackageBuildError, PackageBuildMode, PackageBuildRequest,
    TargetCacheOutcome, cache::MaterializedTarget, plan::PreparedPlan,
};

const PLAN_NAME: &str = "zryna-resolved-build-plan-v0.json";
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageBuildManifest<'a> {
    format: &'static str,
    version: u8,
    mode: &'static str,
    package_lock_sha256: &'a str,
    root_package: PackageIdentity<'a>,
    plan: PlanRecord<'a>,
    targets: Vec<TargetRecord<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageIdentity<'a> {
    id: &'a str,
    source_sha256: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanRecord<'a> {
    path: &'static str,
    cache_key: &'a str,
    bytes: u64,
    sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetRecord<'a> {
    id: &'static str,
    target_cache_key: &'a str,
    cache: &'static str,
    outputs: &'a [super::PublishedPackageOutput],
}

pub(super) fn publish(
    request: &PackageBuildRequest<'_>,
    prepared: &PreparedPlan,
    targets: &[MaterializedTarget],
) -> Result<PathBuf, PackageBuildError> {
    request
        .output_root
        .revalidate()
        .map_err(|_| PackageBuildError::publication("project output root cannot be revalidated"))?;
    let manifest_bytes = manifest_bytes(request, prepared, targets)?;
    let stage_name = format!(
        ".zryna-package-build-{}-{}",
        std::process::id(),
        NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
    );
    let stage = request.output_root.path().join(&stage_name);
    let directory = request
        .output_root
        .retained_directory()
        .map_err(|_| PackageBuildError::publication("project output root cannot be retained"))?;
    directory
        .create_dir(&stage_name)
        .map_err(|_| PackageBuildError::publication("private output stage cannot be created"))?;
    let stage_directory = directory.open_dir_nofollow(&stage_name).map_err(|_| {
        PackageBuildError::publication("private output stage capability cannot be retained")
    })?;
    let mut cleanup_inventory = BTreeSet::new();
    let final_name = format!("{}.package-build", prepared.identity.cache_key());
    let final_path = request.output_root.path().join(&final_name);
    let mut stage_is_private = true;
    let outcome = (|| {
        super::staging::write_file(
            &stage_directory,
            PLAN_NAME,
            prepared.identity.bytes(),
            PackageBuildError::publication,
        )?;
        super::staging::record_path(&mut cleanup_inventory, PLAN_NAME);
        for target in targets {
            for output in &target.outputs {
                super::staging::write_file(
                    &stage_directory,
                    &output.path,
                    &output.bytes,
                    PackageBuildError::publication,
                )?;
                super::staging::record_path(&mut cleanup_inventory, &output.path);
            }
        }
        super::staging::write_file(
            &stage_directory,
            PACKAGE_BUILD_MANIFEST_NAME,
            &manifest_bytes,
            PackageBuildError::publication,
        )?;
        super::staging::record_path(&mut cleanup_inventory, PACKAGE_BUILD_MANIFEST_NAME);
        audit(&stage, targets, prepared.identity.bytes(), &manifest_bytes)?;
        super::staging::sync_tree(&stage_directory, PackageBuildError::publication)?;
        request.output_root.revalidate().map_err(|_| {
            PackageBuildError::publication("project output root changed before publication")
        })?;
        super::staging::revalidate_name(
            &directory,
            &stage_name,
            &stage_directory,
            PackageBuildError::publication,
        )?;
        crate::pipeline::rename_create_only(&directory, &stage_name, &final_name).map_err(
            |_| PackageBuildError::publication("create-only package bundle commit failed"),
        )?;
        stage_is_private = false;
        if let Err(error) = audit(&final_path, targets, prepared.identity.bytes(), &manifest_bytes)
        {
            crate::pipeline::rename_create_only(&directory, &final_name, &stage_name).map_err(
                |_| {
                    PackageBuildError::publication(
                        "published package bundle changed and rollback could not be completed",
                    )
                },
            )?;
            stage_is_private = true;
            return Err(error);
        }
        Ok(())
    })();
    if outcome.is_err() && stage_is_private {
        super::staging::cleanup_known_stage(
            &directory,
            &stage_name,
            &stage_directory,
            &cleanup_inventory,
            PackageBuildError::publication,
        )?;
    }
    outcome?;
    Ok(final_path.join(PACKAGE_BUILD_MANIFEST_NAME))
}

fn manifest_bytes(
    request: &PackageBuildRequest<'_>,
    prepared: &PreparedPlan,
    targets: &[MaterializedTarget],
) -> Result<Vec<u8>, PackageBuildError> {
    let root = request
        .resolution
        .graph()
        .packages()
        .iter()
        .find(|package| package.instance().manifest_id() == request.resolution.graph().root())
        .ok_or_else(|| PackageBuildError::publication("root package identity is unavailable"))?;
    let manifest = PackageBuildManifest {
        format: "zryna.package-build-manifest.v1",
        version: 1,
        mode: match request.mode {
            PackageBuildMode::Offline => "offline",
            PackageBuildMode::Frozen => "frozen",
        },
        package_lock_sha256: request.resolution.graph().lock_sha256(),
        root_package: PackageIdentity {
            id: root.instance().manifest_id(),
            source_sha256: root.instance().source_sha256(),
        },
        plan: PlanRecord {
            path: PLAN_NAME,
            cache_key: prepared.identity.cache_key(),
            bytes: u64::try_from(prepared.identity.bytes().len())
                .map_err(|_| PackageBuildError::publication("plan length cannot be represented"))?,
            sha256: sha256(prepared.identity.bytes()),
        },
        targets: targets
            .iter()
            .map(|target| TargetRecord {
                id: target.observation.target.as_str(),
                target_cache_key: &target.observation.target_cache_key,
                cache: match target.observation.cache {
                    TargetCacheOutcome::Hit => "hit",
                    TargetCacheOutcome::Miss => "miss",
                },
                outputs: &target.observation.outputs,
            })
            .collect(),
    };
    let mut bytes = serde_json::to_vec(&manifest)
        .map_err(|_| PackageBuildError::publication("package manifest cannot be serialized"))?;
    bytes.push(b'\n');
    if bytes.len() > 262_144 {
        return Err(PackageBuildError::publication("package manifest exceeds its byte limit"));
    }
    Ok(bytes)
}

fn audit(
    root: &Path,
    targets: &[MaterializedTarget],
    plan: &[u8],
    manifest: &[u8],
) -> Result<(), PackageBuildError> {
    let expected = expected_inventory(targets);
    for target in targets {
        for output in &target.outputs {
            let bytes = fs::read(root.join(&output.path))
                .map_err(|_| PackageBuildError::publication("published output cannot be read"))?;
            let observation = target
                .observation
                .outputs
                .iter()
                .find(|item| item.path == output.path)
                .ok_or_else(|| PackageBuildError::publication("published output is unrecorded"))?;
            if u64::try_from(bytes.len()).ok() != Some(observation.bytes)
                || sha256(&bytes) != observation.sha256
            {
                return Err(PackageBuildError::publication("published output bytes changed"));
            }
        }
    }
    if fs::read(root.join(PLAN_NAME))
        .map_err(|_| PackageBuildError::publication("resolved plan cannot be read"))?
        != plan
        || fs::read(root.join(PACKAGE_BUILD_MANIFEST_NAME))
            .map_err(|_| PackageBuildError::publication("package manifest cannot be read"))?
            != manifest
    {
        return Err(PackageBuildError::publication(
            "resolved plan or package manifest bytes changed",
        ));
    }
    let mut actual = BTreeSet::new();
    collect(root, root, &mut actual)?;
    if actual != expected {
        return Err(PackageBuildError::publication(
            "package bundle inventory contains missing or extra paths",
        ));
    }
    Ok(())
}

fn expected_inventory(targets: &[MaterializedTarget]) -> BTreeSet<String> {
    let mut expected =
        BTreeSet::from([PLAN_NAME.to_owned(), PACKAGE_BUILD_MANIFEST_NAME.to_owned()]);
    for target in targets {
        for output in &target.outputs {
            let mut prefix = String::new();
            for (index, part) in output.path.split('/').enumerate() {
                if index > 0 {
                    prefix.push('/');
                }
                prefix.push_str(part);
                expected.insert(prefix.clone());
            }
        }
    }
    expected
}

fn collect(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeSet<String>,
) -> Result<(), PackageBuildError> {
    for item in fs::read_dir(directory)
        .map_err(|_| PackageBuildError::publication("package bundle cannot be enumerated"))?
    {
        let path = item
            .map_err(|_| PackageBuildError::publication("package bundle entry is unavailable"))?
            .path();
        let metadata = fs::symlink_metadata(&path).map_err(|_| {
            PackageBuildError::publication("package bundle entry cannot be inspected")
        })?;
        if link_like(&metadata) || (!metadata.is_file() && !metadata.is_dir()) {
            return Err(PackageBuildError::publication("package bundle contains an unsafe path"));
        }
        #[cfg(unix)]
        if metadata.is_file() {
            use std::os::unix::fs::MetadataExt as _;
            if metadata.nlink() != 1 {
                return Err(PackageBuildError::publication(
                    "package bundle contains a hard-link alias",
                ));
            }
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| PackageBuildError::publication("package bundle path escaped"))?;
        entries.insert(relative.to_string_lossy().replace('\\', "/"));
        if metadata.is_dir() {
            collect(root, &path, entries)?;
        }
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(windows)]
fn link_like(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn link_like(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
