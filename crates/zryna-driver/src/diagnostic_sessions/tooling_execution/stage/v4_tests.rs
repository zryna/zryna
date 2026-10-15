//! Modular staging keeps the same original-object authority as the legacy layout.

use sha2::{Digest, Sha256};

use super::super::capture::CapturedFile;
use super::{ROOT, V4Layout, creation, creation_tests::Fault, v4};

fn fixture() -> CapturedFile {
    let bytes = b"inert module ownership fixture\n".to_vec();
    CapturedFile { sha256: Sha256::digest(&bytes).into(), bytes }
}

fn modular_stage() -> super::ToolingStage {
    let mut stage = creation::create_root().expect("owned root");
    stage.v4_layout = V4Layout::Modular;
    creation::create_directory(&mut stage, ROOT, "v4", v4::V4).expect("module parent");
    creation::create_directory(&mut stage, v4::V4, "boundary", v4::BOUNDARY)
        .expect("boundary parent");
    stage
}

#[test]
fn modular_partial_leaf_faults_abort_using_retained_originals() {
    for point in [
        "file-created",
        "file-prefix-written",
        "file-written",
        "file-flushed",
        "file-synced",
        "file-identity",
        "file-metadata",
        "file-readback",
        "file-readonly-retained",
    ] {
        let mut stage = modular_stage();
        let path = stage.physical_path().to_path_buf();
        let _fault = Fault::arm(point);
        creation::stage_file(&mut stage, v4::BOUNDARY, "configuration.mjs", &fixture())
            .expect_err("module creation fault");
        let retained = stage
            .files
            .get("v4/boundary/configuration.mjs")
            .expect("module original registered before fallible work");
        assert!(retained.identity.is_none(), "partial module cannot execute");
        assert!(retained.original.metadata().expect("retained original").is_file());
        stage.abort().expect("explicit partial modular cleanup");
        assert!(!path.exists(), "module directories removed after {point}");
    }
}

#[test]
fn modular_partial_replacement_and_foreign_entries_survive_explicit_abort() {
    let mut stage = modular_stage();
    let path = stage.physical_path().to_path_buf();
    let _fault = Fault::arm("file-prefix-written");
    creation::stage_file(&mut stage, v4::BOUNDARY, "configuration.mjs", &fixture())
        .expect_err("partial module original retained");
    let module = path.join("v4/boundary/configuration.mjs");
    std::fs::rename(&module, path.join("v4/boundary/retained-original"))
        .expect("relocate owned original");
    std::fs::write(&module, b"foreign replacement").expect("replacement fixture");
    std::fs::write(path.join("v4/boundary/foreign"), b"foreign sibling")
        .expect("unknown sibling fixture");
    let error = stage.abort().expect_err("foreign object is never cleanup authority");
    assert!(error.message().contains("file v4/boundary/configuration.mjs:"));
    assert!(error.message().contains("cleanup incomplete"));
    assert_eq!(std::fs::read(&module).expect("foreign retained"), b"foreign replacement");
    assert_eq!(
        std::fs::read(path.join("v4/boundary/foreign")).expect("sibling retained"),
        b"foreign sibling"
    );
    assert_eq!(
        std::fs::read(path.join("v4/boundary/retained-original")).expect("original retained"),
        b"i"
    );
    std::fs::remove_dir_all(path).expect("remove only test-owned retained fixture");
}

#[test]
fn legacy_stage_cannot_create_modular_directory_or_unknown_module() {
    let mut stage = creation::create_root().expect("owned root");
    let path = stage.physical_path().to_path_buf();
    creation::create_directory(&mut stage, ROOT, "v4", v4::V4)
        .expect_err("legacy layout never permits modular mutation");
    stage.v4_layout = V4Layout::Modular;
    creation::stage_file(&mut stage, ROOT, "unknown-module.mjs", &fixture())
        .expect_err("unknown executable never enters inventory");
    assert_eq!(std::fs::read_dir(&path).expect("unchanged inventory").count(), 0);
    stage.abort().expect("empty root cleanup");
    assert!(!path.exists());
}
