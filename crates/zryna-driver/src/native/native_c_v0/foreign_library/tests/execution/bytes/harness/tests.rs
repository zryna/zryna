//! Linked byte observations remain separate from executable approval.
use super::*;

#[test]
fn linked_observation_retains_original_byte_runtime_without_executing_it() {
    let fixture = Fixture::new();
    let requirements = requirements(&capture::reference(), "copied");
    let foreign = fixture.compile_observed(&library_source()).expect("real foreign fixture object");
    let library = accept(&requirements, foreign.bytes(), &["free", "malloc"]);
    linked_observer::check_disaffiliated_control(&fixture, &requirements, &library, &foreign);
    let observation =
        observe_link(&fixture, &requirements, &library, &foreign, "int main(void){abort();}")
            .unwrap_or_else(|failure| {
                let directory = std::env::var_os("ZRYNA_LINKED_OUTPUT_EVIDENCE_DIR").map_or_else(
                    || {
                        fixture
                            .acquired_path()
                            .parent()
                            .expect("independent fixture root exists")
                            .with_extension("failed-byte")
                    },
                    |root| PathBuf::from(root).join("failed-byte"),
                );
                failure
                    .export(&directory)
                    .expect("retain actual failed byte artifacts create-only");
                panic!(
                    "byte observation failed at {:?}; diagnostics {:?}; retained {}; artifacts {:?}; report {}",
                    failure.comparator_stage,
                    failure
                        .diagnostics
                        .iter()
                        .map(|diagnostic| (diagnostic.code(), diagnostic.message()))
                        .collect::<Vec<_>>(),
                    directory.display(),
                    failure.artifacts.iter().map(|(name, bytes)| (*name, bytes.len())).collect::<Vec<_>>(),
                    failure.report()
                );
            });
    fixture.empty();
    let report = observation.report();
    assert_eq!(report["target_executed"], false);
    if std::env::var_os("ZRYNA_LINK_INVENTORY_EXPECTATION_FILE").is_some() {
        assert_eq!(report["inventory_qualification_requested"], true);
        assert!(report["selected_material_comparison"]["materials"].is_array());
        assert!(report["declared_provider_closure_comparison"]["providers"].is_array());
        eprintln!("m5-private-byte-inventory-qualified {report}");
    } else {
        assert_eq!(report["inventory_qualification_requested"], false);
        eprintln!("m5-private-byte-inventory-not-requested");
    }
    assert!(report["private_runtime_object_sha256"].is_string());
    assert_eq!(report["runtime_compile"]["tools"], report["foreign_compile"]["tools"]);
    let directory = std::env::var_os("ZRYNA_LINKED_OUTPUT_EVIDENCE_DIR").map_or_else(
        || fixture.acquired_path().with_file_name("observed-byte"),
        |root| PathBuf::from(root).join("byte"),
    );
    observation.export(&directory).expect("actual retained runtime and final ELF evidence");
    eprintln!("417-linked-output-observation {report}");
}

#[test]
fn linked_observation_byte_mismatched_object_rejects_before_runtime_compilation() {
    let fixture = Fixture::new();
    let requirements = requirements(&capture::reference(), "copied");
    let foreign = fixture
        .compile_observed(&library_source())
        .expect("independent observed fixture prerequisite");
    let library = accept(&requirements, foreign.bytes(), &["free", "malloc"]);
    let other = fixture
        .compile_observed("int unrelated(void){return 1;}")
        .expect("independent observed fixture prerequisite");
    let failure =
        observe_link(&fixture, &requirements, &library, &other, "int main(void){abort();}")
            .expect_err("expected observational rejection");
    assert!(failure.invocations.is_empty());
    fixture.empty();
}
