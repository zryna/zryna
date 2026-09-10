use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cap_fs_ext::DirExt as _;
use serde::{Deserialize, Serialize};

mod filesystem;
mod root;
use filesystem::{collect_inventory, link_like, read_stable_file, sha256};
pub use root::ArtifactCacheRoot;

use super::{
    BuildTargetId, CachedTargetAuthentication, CompiledDependency, CompiledOutput,
    PackageBuildError, PackageBuildRequest, PackageCompilationResult, PackageCompilationUnit,
    PublishedPackageOutput, PublishedPackageTarget, PureSourceCompiler, TargetCacheOutcome,
    plan::PreparedPlan,
};

const ENTRY_MANIFEST: &str = "entry.json";
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub(super) struct MaterializedTarget {
    pub observation: PublishedPackageTarget,
    pub outputs: Vec<CompiledOutput>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CacheEntry {
    format: String,
    plan_cache_key: String,
    target: BuildTargetId,
    target_cache_key: String,
    version: u8,
    outputs: Vec<CacheOutput>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheOutput {
    bytes: u64,
    path: String,
    sha256: String,
}

pub(super) fn execute_targets(
    request: &PackageBuildRequest<'_>,
    prepared: &PreparedPlan,
    compiler: &mut impl PureSourceCompiler,
) -> Result<Vec<MaterializedTarget>, PackageBuildError> {
    request.cache_root.revalidate()?;
    let mut materialized = Vec::with_capacity(request.configuration.targets.len());
    for target in &request.configuration.targets {
        let key = prepared
            .identity
            .target_cache_key(&target.id)
            .ok_or_else(|| PackageBuildError::plan("target cache identity is absent"))?;
        if let Some(outputs) = read_entry(request, &target.id, key, prepared.identity.cache_key())?
        {
            compiler
                .authenticate_cached_outputs(CachedTargetAuthentication {
                    resolution: request.resolution,
                    plan: &prepared.identity,
                    target,
                    outputs: &outputs,
                })
                .map_err(|_| PackageBuildError::cache("cached output authentication failed"))?;
            materialized.push(materialized_target(
                target.id.clone(),
                key,
                TargetCacheOutcome::Hit,
                outputs,
            )?);
            continue;
        }
        let outputs = compile_target(request, prepared, target, compiler)?;
        write_entry(request, &target.id, key, prepared.identity.cache_key(), &outputs)?;
        materialized.push(materialized_target(
            target.id.clone(),
            key,
            TargetCacheOutcome::Miss,
            outputs,
        )?);
    }
    Ok(materialized)
}

fn compile_target(
    request: &PackageBuildRequest<'_>,
    prepared: &PreparedPlan,
    target: &super::BuildTarget,
    compiler: &mut impl PureSourceCompiler,
) -> Result<Vec<CompiledOutput>, PackageBuildError> {
    let graph = request.resolution.graph();
    let mut results = BTreeMap::<String, Vec<u8>>::new();
    let mut root_outputs = None;
    for package_id in &prepared.package_order {
        let package = graph
            .packages()
            .iter()
            .find(|package| package.instance().manifest_id() == package_id)
            .ok_or_else(|| PackageBuildError::plan("ordered package is absent"))?;
        let sources = request
            .resolution
            .sources()
            .iter()
            .find(|source| source.package_id() == package_id)
            .ok_or_else(|| PackageBuildError::plan("ordered package sources are absent"))?;
        let dependencies = package
            .dependencies()
            .iter()
            .map(|(_, id)| {
                let bytes = results.get(id).ok_or_else(|| {
                    PackageBuildError::plan("dependency was not compiled before its importer")
                })?;
                Ok(CompiledDependency { package_id: id, bytes })
            })
            .collect::<Result<Vec<_>, PackageBuildError>>()?;
        let is_root = package_id == graph.root();
        let PackageCompilationResult { dependency_bytes, outputs } =
            compiler.compile(PackageCompilationUnit {
                package_id,
                source_sha256: sources.source_sha256(),
                sources: sources.files(),
                dependencies,
                target,
                is_root,
            })?;
        if !is_root && !outputs.is_empty() {
            return Err(PackageBuildError::compiler(
                "dependency compilation attempted to publish a root output",
            ));
        }
        if is_root {
            root_outputs = Some(outputs);
        }
        results.insert(package_id.clone(), dependency_bytes);
    }
    let outputs =
        root_outputs.ok_or_else(|| PackageBuildError::compiler("root was not compiled"))?;
    validate_outputs(request, &target.id, &outputs)?;
    Ok(outputs)
}

fn validate_outputs(
    request: &PackageBuildRequest<'_>,
    target: &BuildTargetId,
    outputs: &[CompiledOutput],
) -> Result<(), PackageBuildError> {
    let expected = request
        .configuration
        .outputs
        .iter()
        .filter(|output| &output.target == target)
        .map(|output| output.path.as_str())
        .collect::<Vec<_>>();
    if outputs.len() != expected.len()
        || outputs.iter().zip(expected).any(|(actual, expected)| actual.path != expected)
    {
        return Err(PackageBuildError::compiler(
            "compiler output inventory differs from the declared target outputs",
        ));
    }
    if outputs.iter().any(|output| output.bytes.len() > 1_073_741_824) {
        return Err(PackageBuildError::compiler("compiler output exceeds the byte limit"));
    }
    Ok(())
}

fn materialized_target(
    target: BuildTargetId,
    key: &str,
    cache: TargetCacheOutcome,
    outputs: Vec<CompiledOutput>,
) -> Result<MaterializedTarget, PackageBuildError> {
    let observations = outputs
        .iter()
        .map(|output| {
            Ok(PublishedPackageOutput {
                path: output.path.clone(),
                bytes: u64::try_from(output.bytes.len())
                    .map_err(|_| PackageBuildError::cache("output length cannot be represented"))?,
                sha256: sha256(&output.bytes),
            })
        })
        .collect::<Result<Vec<_>, PackageBuildError>>()?;
    Ok(MaterializedTarget {
        observation: PublishedPackageTarget {
            target,
            target_cache_key: key.to_owned(),
            cache,
            outputs: observations,
        },
        outputs,
    })
}

fn entry_path(request: &PackageBuildRequest<'_>, key: &str) -> PathBuf {
    request.cache_root.path().join("build-plan-v0").join(key)
}

fn read_entry(
    request: &PackageBuildRequest<'_>,
    target: &BuildTargetId,
    key: &str,
    plan_key: &str,
) -> Result<Option<Vec<CompiledOutput>>, PackageBuildError> {
    request.cache_root.revalidate()?;
    let path = entry_path(request, key);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(PackageBuildError::cache("cache entry cannot be inspected")),
        Ok(metadata) if !metadata.is_dir() || link_like(&metadata) => {
            return Err(PackageBuildError::cache("cache entry is not a real directory"));
        }
        Ok(_) => {}
    }
    let bytes = read_stable_file(&path.join(ENTRY_MANIFEST), 65_536)?;
    let entry: CacheEntry = serde_json::from_slice(&bytes)
        .map_err(|_| PackageBuildError::cache("cache metadata is malformed"))?;
    let canonical = encode_entry(&entry)?;
    if bytes != canonical
        || entry.format != "zryna.build-cache-entry.v0"
        || entry.version != 0
        || entry.plan_cache_key != plan_key
        || &entry.target != target
        || entry.target_cache_key != key
    {
        return Err(PackageBuildError::cache("cache metadata identity is incompatible"));
    }
    let expected = request
        .configuration
        .outputs
        .iter()
        .filter(|output| &output.target == target)
        .map(|output| output.path.as_str())
        .collect::<Vec<_>>();
    if entry.outputs.len() != expected.len()
        || entry.outputs.iter().zip(expected).any(|(output, expected)| output.path != expected)
    {
        return Err(PackageBuildError::cache("cache output inventory is incomplete"));
    }
    audit_inventory(&path, &entry.outputs)?;
    let mut outputs = Vec::with_capacity(entry.outputs.len());
    for output in entry.outputs {
        let material = read_stable_file(&path.join(&output.path), 1_073_741_824)?;
        if u64::try_from(material.len()).ok() != Some(output.bytes)
            || sha256(&material) != output.sha256
        {
            return Err(PackageBuildError::cache("cached output bytes are stale or corrupt"));
        }
        outputs.push(CompiledOutput { path: output.path, bytes: material });
    }
    Ok(Some(outputs))
}

fn write_entry(
    request: &PackageBuildRequest<'_>,
    target: &BuildTargetId,
    key: &str,
    plan_key: &str,
    outputs: &[CompiledOutput],
) -> Result<(), PackageBuildError> {
    let namespace = request.cache_root.path().join("build-plan-v0");
    let directory = request.cache_root.retained_build_namespace()?;
    let entry = CacheEntry {
        format: "zryna.build-cache-entry.v0".to_owned(),
        plan_cache_key: plan_key.to_owned(),
        target: target.clone(),
        target_cache_key: key.to_owned(),
        version: 0,
        outputs: outputs
            .iter()
            .map(|output| CacheOutput {
                bytes: u64::try_from(output.bytes.len()).unwrap_or(u64::MAX),
                path: output.path.clone(),
                sha256: sha256(&output.bytes),
            })
            .collect(),
    };
    let entry_bytes = encode_entry(&entry)?;
    let mut cleanup_inventory = BTreeSet::new();
    let stage_name =
        format!(".pending-{}-{}", std::process::id(), NEXT_STAGE.fetch_add(1, Ordering::Relaxed));
    let stage = namespace.join(&stage_name);
    directory
        .create_dir(&stage_name)
        .map_err(|_| PackageBuildError::cache("cache stage cannot be created"))?;
    let stage_directory = directory
        .open_dir_nofollow(&stage_name)
        .map_err(|_| PackageBuildError::cache("cache stage capability cannot be retained"))?;
    let mut stage_is_private = true;
    let outcome = (|| {
        for output in outputs {
            super::staging::write_file(
                &stage_directory,
                &output.path,
                &output.bytes,
                PackageBuildError::cache,
            )?;
            super::staging::record_path(&mut cleanup_inventory, &output.path);
        }
        super::staging::write_file(
            &stage_directory,
            ENTRY_MANIFEST,
            &entry_bytes,
            PackageBuildError::cache,
        )?;
        super::staging::record_path(&mut cleanup_inventory, ENTRY_MANIFEST);
        audit_inventory(&stage, &entry.outputs)?;
        super::staging::sync_tree(&stage_directory, PackageBuildError::cache)?;
        super::staging::revalidate_name(
            &directory,
            &stage_name,
            &stage_directory,
            PackageBuildError::cache,
        )?;
        crate::pipeline::rename_create_only(&directory, &stage_name, key)
            .map_err(|_| PackageBuildError::cache("cache entry commit failed"))?;
        stage_is_private = false;
        let verified = read_entry(request, target, key, plan_key);
        if !matches!(&verified, Ok(Some(bytes)) if bytes.as_slice() == outputs) {
            crate::pipeline::rename_create_only(&directory, key, &stage_name).map_err(|_| {
                PackageBuildError::cache(
                    "committed cache entry changed and rollback could not be completed",
                )
            })?;
            stage_is_private = true;
            return match verified {
                Err(error) => Err(error),
                Ok(_) => Err(PackageBuildError::cache("committed cache entry differs")),
            };
        }
        Ok(())
    })();
    if outcome.is_err() && stage_is_private {
        super::staging::cleanup_known_stage(
            &directory,
            &stage_name,
            &stage_directory,
            &cleanup_inventory,
            PackageBuildError::cache,
        )?;
        if let Some(winner) = read_entry(request, target, key, plan_key)? {
            if winner == outputs {
                return Ok(());
            }
            return Err(PackageBuildError::cache(
                "winning cache entry differs from the compiled outputs",
            ));
        }
    }
    outcome
}

fn encode_entry(entry: &CacheEntry) -> Result<Vec<u8>, PackageBuildError> {
    let mut bytes = serde_json::to_vec(entry)
        .map_err(|_| PackageBuildError::cache("cache metadata cannot be serialized"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn audit_inventory(root: &Path, outputs: &[CacheOutput]) -> Result<(), PackageBuildError> {
    let expected = expected_inventory(outputs);
    let mut actual = BTreeSet::new();
    collect_inventory(root, root, &mut actual)?;
    if actual != expected {
        return Err(PackageBuildError::cache(
            "cache entry inventory contains missing or extra paths",
        ));
    }
    Ok(())
}

fn expected_inventory(outputs: &[CacheOutput]) -> BTreeSet<String> {
    let mut expected = BTreeSet::from([ENTRY_MANIFEST.to_owned()]);
    for output in outputs {
        let mut prefix = String::new();
        for (index, component) in output.path.split('/').enumerate() {
            if index > 0 {
                prefix.push('/');
            }
            prefix.push_str(component);
            expected.insert(prefix.clone());
        }
    }
    expected
}
