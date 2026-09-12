use std::{cell::Cell, fs};

#[cfg(windows)]
use std::{path::Path, process::Command};

use super::*;

#[cfg(windows)]
fn is_retained_handle_denial(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(32)
}

#[cfg(not(windows))]
fn is_retained_handle_denial(_error: &std::io::Error) -> bool {
    false
}

#[cfg(windows)]
fn create_junction(link: &Path, target: &Path) {
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .expect("junction command must start");
    assert!(status.success(), "junction fixture must be created");
}

fn one_output() -> Vec<BuildOutput> {
    vec![BuildOutput { path: "javascript/app.mjs".to_owned(), target: BuildTargetId::JavaScript }]
}

fn cold_build(
    resolution: &PackageResolutionSuccess,
    cache: &ArtifactCacheRoot,
    output: &ArtifactOutputRoot,
) -> PackageBuildSuccess {
    execute_package_build(
        &PackageBuildRequest {
            resolution,
            configuration: configuration(one_output()),
            mode: PackageBuildMode::Frozen,
            cache_root: cache,
            output_root: output,
        },
        &mut RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]),
    )
    .expect("cold build")
}

#[test]
fn corrupt_cached_bytes_reject_instead_of_becoming_a_miss() {
    let (_sources, resolution) = fixture();
    let (_cache_project, cache, _first_project, first_output) = roots("corrupt");
    let cold = cold_build(&resolution, &cache, &first_output);
    let key = cold.plan().target_cache_key(&BuildTargetId::JavaScript).expect("target key");
    fs::write(cache.path().join("build-plan-v0").join(key).join("javascript/app.mjs"), b"poisoned")
        .expect("corrupt cache");
    let second_project = TemporaryRoot::new("corrupt-output-two");
    let second_output =
        ArtifactOutputRoot::prepare_for_workspace(second_project.path()).expect("test fixture");
    let mut compiler = RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]);
    let error = execute_package_build(
        &PackageBuildRequest {
            resolution: &resolution,
            configuration: configuration(one_output()),
            mode: PackageBuildMode::Frozen,
            cache_root: &cache,
            output_root: &second_output,
        },
        &mut compiler,
    )
    .expect_err("corrupt cache must reject");
    assert_eq!(error.code(), "ZRYNA-B4102");
    assert!(compiler.calls.is_empty());
    assert!(fs::read_dir(second_output.path()).expect("test fixture").next().is_none());
}

#[test]
fn self_consistent_forged_cache_bytes_fail_trusted_compiler_authentication() {
    let (_sources, resolution) = fixture();
    let (_cache_project, cache, _first_project, first_output) = roots("forged");
    let cold = cold_build(&resolution, &cache, &first_output);
    let key = cold.plan().target_cache_key(&BuildTargetId::JavaScript).expect("target key");
    let entry_root = cache.path().join("build-plan-v0").join(key);
    let output_path = entry_root.join("javascript/app.mjs");
    let original = fs::read(&output_path).expect("cached output");
    let forged = vec![b'x'; original.len()];
    let original_digest = format!("{:x}", Sha256::digest(&original));
    let forged_digest = format!("{:x}", Sha256::digest(&forged));
    fs::write(&output_path, &forged).expect("forged output");
    let metadata_path = entry_root.join("entry.json");
    let metadata = fs::read_to_string(&metadata_path).expect("cache metadata");
    let forged_metadata = metadata.replace(&original_digest, &forged_digest);
    assert_ne!(forged_metadata, metadata);
    fs::write(metadata_path, forged_metadata).expect("forged metadata");

    let second_project = TemporaryRoot::new("forged-output-two");
    let second_output =
        ArtifactOutputRoot::prepare_for_workspace(second_project.path()).expect("test fixture");
    let mut compiler =
        RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]).with_authenticated_cache(
            vec![CompiledOutput { path: "javascript/app.mjs".to_owned(), bytes: original }],
        );
    let error = execute_package_build(
        &PackageBuildRequest {
            resolution: &resolution,
            configuration: configuration(one_output()),
            mode: PackageBuildMode::Frozen,
            cache_root: &cache,
            output_root: &second_output,
        },
        &mut compiler,
    )
    .expect_err("self-consistent forged cache must reject");
    assert_eq!(error.code(), "ZRYNA-B4102");
    assert!(compiler.calls.is_empty());
    assert_eq!(compiler.cache_authentications, 1);
    assert!(fs::read_dir(second_output.path()).expect("test fixture").next().is_none());
}

#[test]
fn mismatched_cache_target_rejects_without_compilation_or_publication() {
    let (_sources, resolution) = fixture();
    let (_cache_project, cache, _first_project, first_output) = roots("target-mismatch");
    let cold = cold_build(&resolution, &cache, &first_output);
    let key = cold.plan().target_cache_key(&BuildTargetId::JavaScript).expect("target key");
    let entry_path = cache.path().join("build-plan-v0").join(key).join("entry.json");
    let entry = fs::read_to_string(&entry_path).expect("cache metadata");
    let mismatched = entry.replace("\"target\":\"javascript\"", "\"target\":\"webassembly\"");
    assert_ne!(mismatched, entry);
    fs::write(entry_path, mismatched).expect("replace cache metadata");

    let second_project = TemporaryRoot::new("target-mismatch-output-two");
    let second_output =
        ArtifactOutputRoot::prepare_for_workspace(second_project.path()).expect("test fixture");
    let mut compiler = RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]);
    let error = execute_package_build(
        &PackageBuildRequest {
            resolution: &resolution,
            configuration: configuration(one_output()),
            mode: PackageBuildMode::Frozen,
            cache_root: &cache,
            output_root: &second_output,
        },
        &mut compiler,
    )
    .expect_err("mismatched target must reject");
    assert_eq!(error.code(), "ZRYNA-B4102");
    assert!(compiler.calls.is_empty());
    assert!(fs::read_dir(second_output.path()).expect("test fixture").next().is_none());
}

#[test]
fn partial_cache_entry_rejects_without_compilation_or_publication() {
    let (_sources, resolution) = fixture();
    let (_cache_project, cache, _first_project, first_output) = roots("partial-prime");
    let cold = cold_build(&resolution, &cache, &first_output);
    let mut changed = configuration(one_output());
    changed.execution_policy.configuration_sha256 = ZERO.to_owned();
    let probe_project = TemporaryRoot::new("partial-probe");
    let probe_output =
        ArtifactOutputRoot::prepare_for_workspace(probe_project.path()).expect("test fixture");
    let probe_cache =
        ArtifactCacheRoot::prepare_for_project(probe_project.path()).expect("test fixture");
    let mut probe_compiler = RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]);
    let probe = execute_package_build(
        &PackageBuildRequest {
            resolution: &resolution,
            configuration: changed.clone(),
            mode: PackageBuildMode::Frozen,
            cache_root: &probe_cache,
            output_root: &probe_output,
        },
        &mut probe_compiler,
    )
    .expect("probe key");
    let key =
        probe.plan().target_cache_key(&BuildTargetId::JavaScript).expect("test fixture").to_owned();
    assert_ne!(key, cold.targets()[0].target_cache_key);
    let namespace = cache.path().join("build-plan-v0");
    fs::create_dir_all(namespace.join(&key)).expect("partial entry");

    let final_project = TemporaryRoot::new("partial-output");
    let final_output =
        ArtifactOutputRoot::prepare_for_workspace(final_project.path()).expect("test fixture");
    let mut compiler = RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]);
    let error = execute_package_build(
        &PackageBuildRequest {
            resolution: &resolution,
            configuration: changed,
            mode: PackageBuildMode::Frozen,
            cache_root: &cache,
            output_root: &final_output,
        },
        &mut compiler,
    )
    .expect_err("partial cache must reject");
    assert_eq!(error.code(), "ZRYNA-B4102");
    assert!(compiler.calls.is_empty());
    assert!(fs::read_dir(final_output.path()).expect("test fixture").next().is_none());
}

#[test]
fn wrong_or_extra_compiler_outputs_fail_before_cache_and_publication() {
    let (_sources, resolution) = fixture();
    let (_cache_project, cache, _output_project, output) = roots("wrong-output");
    let mut compiler = RecordingCompiler::new(vec!["javascript/other.mjs".to_owned()]);
    let error = execute_package_build(
        &PackageBuildRequest {
            resolution: &resolution,
            configuration: configuration(one_output()),
            mode: PackageBuildMode::Frozen,
            cache_root: &cache,
            output_root: &output,
        },
        &mut compiler,
    )
    .expect_err("wrong output inventory");
    assert_eq!(error.code(), "ZRYNA-B4104");
    assert!(fs::read_dir(output.path()).expect("test fixture").next().is_none());
    let namespace = cache.path().join("build-plan-v0");
    assert!(!namespace.exists() || fs::read_dir(namespace).expect("test fixture").next().is_none());
}

#[test]
fn reserved_and_ancestor_conflicting_outputs_reject_before_compilation() {
    let (_sources, resolution) = fixture();
    let cases = [
        vec![BuildOutput { path: "entry.json".to_owned(), target: BuildTargetId::JavaScript }],
        vec![BuildOutput {
            path: "zryna-resolved-build-plan-v0.json".to_owned(),
            target: BuildTargetId::JavaScript,
        }],
        vec![BuildOutput {
            path: PACKAGE_BUILD_MANIFEST_NAME.to_owned(),
            target: BuildTargetId::JavaScript,
        }],
        vec![
            BuildOutput { path: "javascript/app".to_owned(), target: BuildTargetId::JavaScript },
            BuildOutput {
                path: "javascript/app/module.mjs".to_owned(),
                target: BuildTargetId::JavaScript,
            },
        ],
    ];
    for (index, outputs) in cases.into_iter().enumerate() {
        let (_cache_project, cache, _output_project, output) =
            roots(&format!("output-collision-{index}"));
        let mut compiler = RecordingCompiler::new(vec![]);
        let error = execute_package_build(
            &PackageBuildRequest {
                resolution: &resolution,
                configuration: configuration(outputs),
                mode: PackageBuildMode::Frozen,
                cache_root: &cache,
                output_root: &output,
            },
            &mut compiler,
        )
        .expect_err("reserved or conflicting output path");
        assert_eq!(error.code(), "ZRYNA-B4101");
        assert!(compiler.calls.is_empty());
        assert!(fs::read_dir(output.path()).expect("test fixture").next().is_none());
    }
}

#[test]
fn compiler_failure_leaves_no_cache_entry_or_bundle() {
    let (_sources, resolution) = fixture();
    let (_cache_project, cache, _output_project, output) = roots("failure");
    let mut compiler = RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]);
    compiler.fail = true;
    let error = execute_package_build(
        &PackageBuildRequest {
            resolution: &resolution,
            configuration: configuration(one_output()),
            mode: PackageBuildMode::Frozen,
            cache_root: &cache,
            output_root: &output,
        },
        &mut compiler,
    )
    .expect_err("compiler failure");
    assert_eq!(error.code(), "ZRYNA-B4104");
    assert!(fs::read_dir(output.path()).expect("test fixture").next().is_none());
}

#[test]
fn substituted_stage_is_never_recursively_cleaned() {
    let root = TemporaryRoot::new("substituted-stage");
    let stage = root.path().join("stage");
    let displaced = root.path().join("displaced");
    let outside = root.path().join("outside");
    fs::write(&outside, b"retain").expect("outside sentinel");
    let parent = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("parent capability");
    let stage_directory =
        staging::WritableStage::create(&parent, "stage", PackageBuildError::cache)
            .expect("stage capability")
            .seal();
    let substitution_denied = Cell::new(false);
    let result = staging::cleanup_known_stage_with_substitution_hook(
        &parent,
        stage_directory,
        &std::collections::BTreeSet::new(),
        PackageBuildError::cache,
        || match fs::rename(&stage, &displaced) {
            Ok(()) => {
                fs::create_dir(&stage).expect("replacement stage");
                fs::write(stage.join("foreign"), b"retain").expect("foreign entry");
            }
            Err(error) if is_retained_handle_denial(&error) => {
                substitution_denied.set(true);
            }
            Err(error) => panic!("displace genuine stage: {error}"),
        },
    );
    if substitution_denied.get() {
        result.expect("an OS-protected stage remains the authenticated stage");
        assert!(!stage.exists());
        assert!(!displaced.exists());
    } else {
        let error = result.expect_err("substituted stage must not be cleaned");
        assert_eq!(error.code(), "ZRYNA-B4102");
        assert_eq!(fs::read(stage.join("foreign")).expect("test fixture"), b"retain");
        assert!(displaced.exists());
    }
    assert_eq!(fs::read(outside).expect("outside sentinel"), b"retain");
}

#[test]
fn unexpected_stage_entry_is_never_cleaned() {
    let root = TemporaryRoot::new("unexpected-stage-entry");
    let stage = root.path().join("stage");
    let parent = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("parent capability");
    let stage_directory =
        staging::WritableStage::create(&parent, "stage", PackageBuildError::cache)
            .expect("stage capability");
    staging::write_file(
        stage_directory.directory(),
        "foreign",
        b"retain",
        PackageBuildError::cache,
    )
    .expect("foreign entry");
    let error = staging::cleanup_known_stage(
        &parent,
        stage_directory.seal(),
        &std::collections::BTreeSet::new(),
        PackageBuildError::cache,
    )
    .expect_err("unexpected entry must block cleanup");
    assert_eq!(error.code(), "ZRYNA-B4102");
    assert_eq!(fs::read(stage.join("foreign")).expect("test fixture"), b"retain");
}

#[cfg(windows)]
#[test]
fn expected_junction_is_rejected_without_touching_its_target() {
    let root = TemporaryRoot::new("expected-stage-junction");
    let stage = root.path().join("stage");
    let outside = root.path().join("outside");
    let junction = stage.join("expected");
    fs::create_dir(&outside).expect("outside directory");
    fs::write(outside.join("sentinel"), b"retain").expect("outside sentinel");
    let parent = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("parent capability");
    let stage_directory =
        staging::WritableStage::create(&parent, "stage", PackageBuildError::cache)
            .expect("stage capability");
    create_junction(&junction, &outside);
    let expected = std::collections::BTreeSet::from(["expected".to_owned()]);
    let error = staging::cleanup_known_stage(
        &parent,
        stage_directory.seal(),
        &expected,
        PackageBuildError::cache,
    )
    .expect_err("an expected-name junction must block cleanup");
    assert_eq!(error.code(), "ZRYNA-B4102");
    assert_eq!(fs::read(outside.join("sentinel")).expect("outside sentinel"), b"retain");
    assert!(stage.exists());
    assert!(junction.exists());
    fs::remove_dir(&junction).expect("junction cleanup");
}

#[cfg(windows)]
#[test]
fn held_descendant_blocks_commit_without_losing_the_stage() {
    let root = TemporaryRoot::new("held-stage-descendant");
    let parent = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("parent capability");
    let writable = staging::WritableStage::create(&parent, "stage", PackageBuildError::cache)
        .expect("stage capability");
    writable.directory().create_dir("nested").expect("nested directory");
    let descendant = writable.directory().open_dir("nested").expect("descendant capability");
    let mut sealed = writable.seal();
    let error = sealed
        .rename_noreplace(&parent, "final", "cache entry commit failed", PackageBuildError::cache)
        .expect_err("a live descendant must block the ancestor rename");
    assert_eq!(error.code(), "ZRYNA-B4102");
    assert!(root.path().join("stage").is_dir());
    assert!(!root.path().join("final").exists());
    drop(descendant);
    let expected = std::collections::BTreeSet::from(["nested".to_owned()]);
    staging::cleanup_known_stage(&parent, sealed, &expected, PackageBuildError::cache)
        .expect("stage cleanup after descendant close");
}

#[cfg(windows)]
#[test]
fn commit_collision_preserves_the_owned_stage_and_foreign_destination() {
    let root = TemporaryRoot::new("stage-commit-collision");
    let parent = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("parent capability");
    let writable = staging::WritableStage::create(&parent, "stage", PackageBuildError::cache)
        .expect("stage capability");
    staging::write_file(writable.directory(), "owned", b"owned", PackageBuildError::cache)
        .expect("owned stage file");
    fs::create_dir(root.path().join("final")).expect("foreign destination");
    fs::write(root.path().join("final/foreign"), b"foreign").expect("foreign sentinel");
    let mut sealed = writable.seal();
    sealed
        .rename_noreplace(&parent, "final", "cache entry commit failed", PackageBuildError::cache)
        .expect_err("an existing destination must block commit");
    assert_eq!(fs::read(root.path().join("stage/owned")).expect("owned file"), b"owned");
    assert_eq!(fs::read(root.path().join("final/foreign")).expect("foreign sentinel"), b"foreign");
    let expected = std::collections::BTreeSet::from(["owned".to_owned()]);
    staging::cleanup_known_stage(&parent, sealed, &expected, PackageBuildError::cache)
        .expect("owned stage cleanup");
}

#[cfg(windows)]
#[test]
fn postcommit_mutation_rolls_back_the_exact_stage_before_cleanup_rejects() {
    let root = TemporaryRoot::new("stage-postcommit-mutation");
    let parent = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("parent capability");
    let writable = staging::WritableStage::create(&parent, "stage", PackageBuildError::cache)
        .expect("stage capability");
    staging::write_file(writable.directory(), "owned", b"owned", PackageBuildError::cache)
        .expect("owned stage file");
    let mut sealed = writable.seal();
    sealed
        .rename_noreplace(&parent, "final", "cache entry commit failed", PackageBuildError::cache)
        .expect("stage commit");
    sealed.directory().write("foreign", b"foreign").expect("postcommit mutation");
    sealed
        .rename_noreplace(&parent, "stage", "cache entry rollback failed", PackageBuildError::cache)
        .expect("exact rollback");
    assert!(!root.path().join("final").exists());
    let expected = std::collections::BTreeSet::from(["owned".to_owned()]);
    staging::cleanup_known_stage(&parent, sealed, &expected, PackageBuildError::cache)
        .expect_err("unexpected postcommit content must preserve the rolled-back stage");
    assert_eq!(fs::read(root.path().join("stage/owned")).expect("owned file"), b"owned");
    assert_eq!(fs::read(root.path().join("stage/foreign")).expect("foreign file"), b"foreign");
}

#[cfg(windows)]
#[test]
fn rollback_collision_preserves_committed_and_foreign_roots() {
    let root = TemporaryRoot::new("stage-rollback-collision");
    let parent = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("parent capability");
    let writable = staging::WritableStage::create(&parent, "stage", PackageBuildError::cache)
        .expect("stage capability");
    staging::write_file(writable.directory(), "owned", b"owned", PackageBuildError::cache)
        .expect("owned stage file");
    let mut sealed = writable.seal();
    sealed
        .rename_noreplace(&parent, "final", "cache entry commit failed", PackageBuildError::cache)
        .expect("stage commit");
    fs::create_dir(root.path().join("stage")).expect("foreign rollback destination");
    fs::write(root.path().join("stage/foreign"), b"foreign").expect("foreign sentinel");
    sealed
        .rename_noreplace(&parent, "stage", "cache entry rollback failed", PackageBuildError::cache)
        .expect_err("rollback collision must fail closed");
    assert_eq!(fs::read(root.path().join("final/owned")).expect("owned file"), b"owned");
    assert_eq!(fs::read(root.path().join("stage/foreign")).expect("foreign sentinel"), b"foreign");

    fs::remove_file(root.path().join("stage/foreign")).expect("foreign file cleanup");
    fs::remove_dir(root.path().join("stage")).expect("foreign directory cleanup");
    sealed
        .rename_noreplace(&parent, "stage", "cache entry rollback failed", PackageBuildError::cache)
        .expect("rollback after collision removal");
    let expected = std::collections::BTreeSet::from(["owned".to_owned()]);
    staging::cleanup_known_stage(&parent, sealed, &expected, PackageBuildError::cache)
        .expect("owned stage cleanup");
}

#[test]
fn staged_writes_remain_bound_to_the_retained_directory() {
    let root = TemporaryRoot::new("substituted-stage-write");
    let stage = root.path().join("stage");
    let displaced = root.path().join("displaced");
    let outside = root.path().join("outside");
    fs::write(&outside, b"retain").expect("outside sentinel");
    let parent = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("parent capability");
    let stage_directory =
        staging::WritableStage::create(&parent, "stage", PackageBuildError::cache)
            .expect("stage capability");
    let substitution_denied = match fs::rename(&stage, &displaced) {
        Ok(()) => {
            fs::create_dir(&stage).expect("replacement stage");
            fs::write(stage.join("foreign"), b"retain").expect("foreign entry");
            false
        }
        Err(error) if is_retained_handle_denial(&error) => true,
        Err(error) => panic!("displace genuine stage: {error}"),
    };

    staging::write_file(
        stage_directory.directory(),
        "nested/output",
        b"trusted",
        PackageBuildError::cache,
    )
    .expect("capability-relative write");
    if substitution_denied {
        assert_eq!(fs::read(stage.join("nested/output")).expect("test fixture"), b"trusted");
        assert!(!displaced.exists());
        let mut expected = std::collections::BTreeSet::new();
        staging::record_path(&mut expected, "nested/output");
        staging::cleanup_known_stage(
            &parent,
            stage_directory.seal(),
            &expected,
            PackageBuildError::cache,
        )
        .expect("OS-protected stage cleanup");
        assert!(!stage.exists());
    } else {
        assert_eq!(fs::read(displaced.join("nested/output")).expect("test fixture"), b"trusted");
        assert_eq!(fs::read(stage.join("foreign")).expect("test fixture"), b"retain");
        assert!(!stage.join("nested").exists());
    }
    assert_eq!(fs::read(outside).expect("outside sentinel"), b"retain");
}

#[test]
fn replaced_cache_and_output_roots_reject_before_parent_capture() {
    let cache_project = TemporaryRoot::new("replaced-cache-root");
    let cache = ArtifactCacheRoot::prepare_for_project(cache_project.path()).expect("cache");
    let cache_path = cache.path().to_path_buf();
    let cache_identity = same_file::Handle::from_path(&cache_path).expect("cache identity");
    let displaced_cache = cache_path.with_file_name("cache-displaced");
    let cache_outside = cache_project.path().join("cache-outside");
    fs::write(&cache_outside, b"retain").expect("cache outside sentinel");
    let cache_substitution_denied = Cell::new(false);
    let cache_result = cache.retained_directory_with_substitution_hook(|| {
        match fs::rename(&cache_path, &displaced_cache) {
            Ok(()) => fs::create_dir(&cache_path).expect("replacement cache root"),
            Err(error) if is_retained_handle_denial(&error) => {
                cache_substitution_denied.set(true);
            }
            Err(error) => panic!("displace cache root: {error}"),
        }
    });
    if cache_substitution_denied.get() {
        drop(cache_result.expect("OS-protected cache root remains authenticated"));
        cache.revalidate().expect("original cache root remains valid");
        assert_eq!(
            same_file::Handle::from_path(&cache_path).expect("current cache identity"),
            cache_identity
        );
        assert!(!displaced_cache.exists());
    } else {
        let cache_error = cache_result.expect_err("replaced cache root must reject");
        assert_eq!(cache_error.code(), "ZRYNA-B4102");
        assert!(displaced_cache.exists());
    }
    assert!(fs::read_dir(&cache_path).expect("test fixture").next().is_none());
    assert_eq!(fs::read(cache_outside).expect("cache outside sentinel"), b"retain");

    let output_project = TemporaryRoot::new("replaced-output-root");
    let output = ArtifactOutputRoot::prepare_for_workspace(output_project.path()).expect("output");
    let output_path = output.path().to_path_buf();
    let output_identity = same_file::Handle::from_path(&output_path).expect("output identity");
    let displaced_output = output_path.with_file_name("out-displaced");
    let output_outside = output_project.path().join("output-outside");
    fs::write(&output_outside, b"retain").expect("output outside sentinel");
    let output_substitution_denied = Cell::new(false);
    let output_result = output.retained_directory_with_substitution_hook(|| {
        match fs::rename(&output_path, &displaced_output) {
            Ok(()) => fs::create_dir(&output_path).expect("replacement output root"),
            Err(error) if is_retained_handle_denial(&error) => {
                output_substitution_denied.set(true);
            }
            Err(error) => panic!("displace output root: {error}"),
        }
    });
    if output_substitution_denied.get() {
        drop(output_result.expect("OS-protected output root remains authenticated"));
        output.revalidate().expect("original output root remains valid");
        assert_eq!(
            same_file::Handle::from_path(&output_path).expect("current output identity"),
            output_identity
        );
        assert!(!displaced_output.exists());
    } else {
        let output_error = output_result.expect_err("replaced output root must reject");
        assert_eq!(output_error.code(), "ZRYNA-D2002");
        assert!(displaced_output.exists());
    }
    assert!(fs::read_dir(&output_path).expect("test fixture").next().is_none());
    assert_eq!(fs::read(output_outside).expect("output outside sentinel"), b"retain");
}

#[test]
fn replaced_cache_namespace_rejects_before_stage_creation() {
    let project = TemporaryRoot::new("replaced-cache-namespace");
    let cache = ArtifactCacheRoot::prepare_for_project(project.path()).expect("cache");
    let namespace = cache.path().join("build-plan-v0");
    let displaced = cache.path().join("build-plan-v0-displaced");
    let error = cache
        .retained_build_namespace_with_substitution_hook(|| {
            fs::rename(&namespace, &displaced).expect("displace namespace");
            fs::write(&namespace, b"foreign").expect("replacement namespace");
        })
        .expect_err("replaced namespace must reject");
    assert_eq!(error.code(), "ZRYNA-B4102");
    assert_eq!(fs::read(&namespace).expect("test fixture"), b"foreign");
    assert!(fs::read_dir(displaced).expect("test fixture").next().is_none());
}

#[test]
fn frozen_build_rejects_an_update_resolution_before_compilation() {
    let (sources, _frozen) = fixture();
    let update = resolved(&sources, PackageLockMode::Update);
    let (_cache_project, cache, _output_project, output) = roots("frozen-update");
    let mut compiler = RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]);
    let error = execute_package_build(
        &PackageBuildRequest {
            resolution: &update,
            configuration: configuration(one_output()),
            mode: PackageBuildMode::Frozen,
            cache_root: &cache,
            output_root: &output,
        },
        &mut compiler,
    )
    .expect_err("updated lock cannot masquerade as frozen");
    assert_eq!(error.code(), "ZRYNA-B4101");
    assert!(compiler.calls.is_empty());
}
