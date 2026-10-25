//! Real original sources reach all ordinary backends and the guarded create-only publisher.
use super::*;
use crate::{ArtifactOutputRoot, generic_closure, generic_manifest, source_scope};
use generic_closure::test_source::{CurrentSyntax, MemoryRoot};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use zryna_source::NormalizedSourcePath;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Output(PathBuf);
impl Output {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "m416-generic-publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).expect("unique output workspace");
        Self(path)
    }
    fn root(&self) -> ArtifactOutputRoot {
        ArtifactOutputRoot::prepare_for_workspace(&self.0).expect("retained output root")
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("own test workspace cleanup");
    }
}
fn source(case: &str) -> MemoryRoot {
    let prefix = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests");
    let rows: Vec<(&str, &str)> = match case {
        "scalar" => vec![(
            "main.zry",
            "m7-generic-full-profile-first-vertical/literals/scalar-i32-clone.zry",
        )],
        "entry-one" => vec![
            ("a.zry", "m7-generic-execution-entry-one/literals/a.zry"),
            ("main.zry", "m7-generic-execution-entry-one/literals/main.zry"),
        ],
        "mixed" => {
            vec![("main.zry", "m7-generic-execution-recursive-owner/mixed/mixed-catalogue.zry")]
        }
        _ => panic!("closed genuine fixture"),
    };
    MemoryRoot::new(rows.into_iter().map(|(name, path)| {
        (name.to_owned(), fs::read_to_string(prefix.join(path)).expect("unchanged original source"))
    }))
}
fn entry() -> NormalizedSourcePath {
    NormalizedSourcePath::new("main.zry").expect("entry")
}

#[test]
fn generic_ordinary_source_build_publishes_all_targets_and_replays_without_overwrite() {
    for case in ["scalar", "entry-one", "mixed"] {
        let source = source(case);
        let output = Output::new();
        let output = output.root();
        let build = || {
            generic_closure::with_closure_terminal(
                &source,
                entry(),
                &CurrentSyntax::ordinary(),
                &|| Ok(()),
                |closure, checkpoint| {
                    source_scope::with_execution(
                        closure,
                        checkpoint,
                        |program, receipt, checkpoint| {
                            prepare(program, receipt, TargetSelection::All, checkpoint)
                        },
                    )
                },
                |prepared, checkpoint| {
                    assert!(source.active.load(Ordering::SeqCst));
                    publish(prepared, &output, "sample", checkpoint)
                },
            )
        };
        let published = build().expect("same-owner complete build publication");
        assert!(!source.active.load(Ordering::SeqCst));
        assert_eq!(
            published.manifest_path(),
            published.bundle_path().join(generic_manifest::MANIFEST_NAME)
        );
        let bytes = fs::read(published.manifest_path()).expect("actual committed manifest");
        assert_eq!(bytes, published.manifest_bytes());
        let metadata = generic_manifest::decode(&bytes).expect("strict generic metadata");
        assert_eq!(metadata.source.entry_module, u32::from(case == "entry-one"));
        assert_eq!(metadata.artifacts.len(), 3);
        assert!(metadata.native.is_some());
        for artifact in metadata.artifacts {
            use sha2::{Digest, Sha256};
            let artifact_bytes =
                fs::read(published.bundle_path().join(artifact.path)).expect("ordinary artifact");
            assert_eq!(u64::try_from(artifact_bytes.len()).expect("bounded"), artifact.bytes);
            assert_eq!(format!("{:x}", Sha256::digest(&artifact_bytes)), artifact.sha256);
        }
        let failure = build().expect_err("same-name create-only replay must reject");
        assert_eq!(failure.kind(), crate::CommandFailureKind::Preparation);
        assert_eq!(fs::read(published.manifest_path()).expect("original untouched"), bytes);
        assert_eq!(fs::read_dir(output.path()).expect("closed output inventory").count(), 1);
    }
}

#[test]
fn generic_source_drift_blocks_finish_and_late_guard_cleans_every_staged_file() {
    for late in [false, true] {
        let source = source("scalar");
        let output = Output::new();
        let output = output.root();
        let failure = generic_closure::with_closure_terminal(
            &source,
            entry(),
            &CurrentSyntax::ordinary(),
            &|| Ok(()),
            |closure, checkpoint| {
                source_scope::with_execution(closure, checkpoint, |program, receipt, checkpoint| {
                    let prepared = prepare(program, receipt, TargetSelection::All, checkpoint)?;
                    if !late {
                        source.drift.store(true, Ordering::SeqCst);
                    }
                    Ok(prepared)
                })
            },
            |prepared, checkpoint| {
                let checks = std::cell::Cell::new(0);
                publish(prepared, &output, "sample", &|| {
                    checks.set(checks.get() + 1);
                    // Entry, three artifacts and manifest preparation have passed.
                    if checks.get() == 6 {
                        source.drift.store(true, Ordering::SeqCst);
                    }
                    checkpoint()
                })
            },
        )
        .expect_err("retained source drift cannot publish");
        assert_eq!(failure.kind(), crate::CommandFailureKind::Source);
        assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-S1001");
        assert_eq!(fs::read_dir(output.path()).expect("clean output").count(), 0);
        assert!(!source.active.load(Ordering::SeqCst));
    }
}
