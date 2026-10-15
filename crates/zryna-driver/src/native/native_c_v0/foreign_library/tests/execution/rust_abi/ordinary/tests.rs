//! Six nonignored owning selectors; each provision really compiles its trusted Rust prerequisite.
use super::*;
fn setup() -> (material::OrdinaryTools, PathBuf, material::OrdinaryRustMeasurement) {
    let parent = material::evidence_parent().expect("external ordinary evidence parent");
    let tools = material::OrdinaryTools::acquire().expect("actual ordinary GCC acquisition/probes");
    let measured = rust_measurement::fresh(&tools, &parent)
        .expect("mandatory fresh ordinary Rust measurement; no old-receipt fallback");
    (tools, parent, measured)
}
fn requirement(
    object: &zryna_backend_native::native_c_v0::resources::ValidatedHandleEntries,
) -> HandleLinkRequirements {
    handle_link_requirements(object).expect("genuine original source/backend issuer")
}
pub(super) fn successful(session: &Session<'_>, marker: &[u8]) {
    assert!(
        session.diagnostics.is_empty(),
        "actual comparator or cleanup failure: {:?}",
        session.diagnostics
    );
    let output = session
        .records
        .iter()
        .find(|record| record.name == "run")
        .and_then(|r| r.output.as_ref())
        .expect("actual bounded run output");
    assert!(
        output.status.success(),
        "actual run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, marker);
    assert!(output.stderr.is_empty());
}
#[test]
fn ordinary_rust_scalar_wrapper_matches_independent_counter_oracle() {
    let (tools, parent, measured) = setup();
    let original = capture::capture();
    let issued = capture::issue(&original);
    let requirements = requirement(&issued.scalar_import);
    let library = measured.capture(&requirements).expect("fresh Rust profile data");
    let request =
        ImportRequest { requirements: &requirements, library: &library, measurement: &measured };
    let (_, session) = observe_import(&request, &tools, &parent, Mode::Scalar)
        .expect("ordinary trusted scalar observer");
    assert!(session.targets_checked);
    successful(&session, b"ordinary-rust-scalar:9calls\n");
}
#[test]
fn ordinary_rust_byte_wrapper_enforces_packed_boundary_and_cleanup() {
    let (tools, parent, measured) = setup();
    let original = capture::capture();
    let issued = capture::issue(&original);
    let requirements = requirement(&issued.byte_import);
    let library = measured.capture(&requirements).expect("fresh Rust profile data");
    let request =
        ImportRequest { requirements: &requirements, library: &library, measurement: &measured };
    let (_, session) =
        observe_import(&request, &tools, &parent, Mode::Bytes { missing_increment: false })
            .expect("ordinary trusted byte observer");
    assert!(session.targets_checked);
    successful(&session, b"ordinary-rust-bytes:4entered4prechecks\n");
}
#[test]
fn ordinary_zryna_reverse_export_matches_independent_c_client() {
    let (tools, parent, _measured) = setup();
    let original = capture::capture();
    let issued = capture::issue(&original);
    let (_, session) = observe_reverse(&issued.reverse_export, &tools, &parent)
        .expect("ordinary pure reverse observer");
    assert!(session.measurement.is_none());
    successful(&session, b"");
    // A separately compiled caller of the actual header must reject wrong arity.
    let workspace = Workspace::new(&parent).expect("independent header-control workspace");
    let stages = workspace.stages(1).expect("header-control stage");
    let mut session = Session::new(&tools);
    let source = format!(
        "{}\nint main(void){{return zryna_c_v0_e_add(1);}}",
        issued.reverse_export.header()
    );
    let result = process::compile(&mut session, &stages[0], "wrong-arity", source.as_bytes());
    assert!(result.is_err());
    let output = session
        .records
        .last()
        .and_then(|r| r.output.as_ref())
        .expect("actual nonzero GCC result, not process error");
    assert!(!output.status.success() && !output.stderr.is_empty());
    cleanup(&mut session, &stages, &[]);
    complete(&mut session, result, &workspace, "reverse-header-control");
}
#[test]
fn rust_observer_rejects_reissued_or_cross_selected_requirements_before_process() {
    let (tools, parent, measured) = setup();
    let original = capture::capture();
    let issued = capture::issue(&original);
    let scalar = requirement(&issued.scalar_import);
    let bytes = requirement(&issued.byte_import);
    let library = measured.capture(&scalar).expect("original data binding");
    controls::wrong_modes(&measured, &scalar, &bytes, &tools, &parent);
    let independent = capture::capture();
    let reissued = requirement(&capture::issue(&independent).scalar_import);
    assert_eq!(reissued.object_sha256(), scalar.object_sha256());
    let before = process::attempts();
    for wrong in [&bytes, &reissued] {
        let request =
            ImportRequest { requirements: wrong, library: &library, measurement: &measured };
        assert!(observe_import(&request, &tools, &parent, Mode::Scalar).is_err());
    }
    assert_eq!(
        process::attempts(),
        before,
        "no new runtime/client/link/metadata attempt after known prerequisites"
    );
    let request =
        ImportRequest { requirements: &scalar, library: &library, measurement: &measured };
    let (_, session) = observe_import(&request, &tools, &parent, Mode::Scalar)
        .expect("pristine original issuer recovery");
    successful(&session, b"ordinary-rust-scalar:9calls\n");
}
#[test]
fn rust_observer_rejects_substituted_measurement_and_tools_before_process() {
    let (tools, parent, mut measured) = setup();
    let original = capture::capture();
    let issued = capture::issue(&original);
    let requirements = requirement(&issued.scalar_import);
    let library = measured.capture(&requirements).expect("original measured source/object");
    controls::provenance(&measured, &requirements, &library, &tools, &parent);
    controls::artifact_bounds();
    let before = process::attempts();
    let pristine = measured.source[0];
    measured.source[0] ^= 1;
    assert!(
        observe_import(
            &ImportRequest {
                requirements: &requirements,
                library: &library,
                measurement: &measured
            },
            &tools,
            &parent,
            Mode::Scalar
        )
        .is_err()
    );
    measured.source[0] = pristine;
    let pristine = measured.object[0];
    measured.object[0] ^= 1;
    assert!(
        observe_import(
            &ImportRequest {
                requirements: &requirements,
                library: &library,
                measurement: &measured
            },
            &tools,
            &parent,
            Mode::Scalar
        )
        .is_err()
    );
    measured.object[0] = pristine;
    let substituted = tools.substituted_control();
    let request =
        ImportRequest { requirements: &requirements, library: &library, measurement: &measured };
    assert!(observe_import(&request, &substituted, &parent, Mode::Scalar).is_err());
    assert_eq!(process::attempts(), before);
    let mut hostile = reference::hostile_object(&["add", "not_sum"]);
    assert!(targets::compare_functions(reference::ORIGINAL_RUST_OBJECT, &hostile).is_err());
    hostile.clear();
    assert_eq!(digest(reference::ORIGINAL_RUST_OBJECT), material::OBJECT_SHA);
    assert!(
        material::check_source(&vec![0; zryna_backend_native::MAX_NATIVE_OBJECT_BYTES + 1])
            .is_err()
    );
    assert!(
        targets::compare_target(b"  4000: call 4010 <not_sum>\n", 0x4010, "sum_bytes").is_err()
    );
    assert!(
        targets::compare_target(b"  4000: call 4010 <sum_bytes>\n", 0x4011, "sum_bytes").is_err()
    );
    assert!(
        targets::compare_target(b"  4000: call 4010 <sum_bytes>\n", 0x4010, "sum_bytes").is_ok()
    );
    let (_, session) = observe_import(&request, &tools, &parent, Mode::Scalar)
        .expect("pristine material/tool recovery");
    successful(&session, b"ordinary-rust-scalar:9calls\n");
    let final_elf = &session
        .artifacts
        .iter()
        .find(|(name, _)| name == "final.elf")
        .expect("actual retained ELF")
        .1;
    let changed = targets::substituted_function_control(final_elf, "add")
        .expect("bounded hostile function-byte control");
    assert!(targets::compare_functions(reference::ORIGINAL_RUST_OBJECT, &changed).is_err());
    assert!(targets::compare_functions(reference::ORIGINAL_RUST_OBJECT, final_elf).is_ok());
}
#[test]
fn rust_entry_counter_mutant_fails_and_pristine_recovers() {
    use std::os::unix::process::ExitStatusExt as _;
    let (tools, parent, measured) = setup();
    let original = capture::capture();
    let issued = capture::issue(&original);
    let requirements = requirement(&issued.byte_import);
    let library = measured.capture(&requirements).expect("fresh measured Rust data");
    let request =
        ImportRequest { requirements: &requirements, library: &library, measurement: &measured };
    let (_, mutant) =
        observe_import(&request, &tools, &parent, Mode::Bytes { missing_increment: true })
            .expect("actual mutant observer result");
    assert!(mutant.diagnostics.is_empty() && mutant.targets_checked);
    let output = mutant
        .records
        .iter()
        .find(|r| r.name == "run")
        .and_then(|r| r.output.as_ref())
        .expect("actual nonzero mutation run");
    assert_eq!(output.status.signal(), Some(libc::SIGABRT));
    assert!(String::from_utf8_lossy(&output.stderr).contains("shim_calls==1"));
    let (_, recovery) =
        observe_import(&request, &tools, &parent, Mode::Bytes { missing_increment: false })
            .expect("pristine observer recovery");
    successful(&recovery, b"ordinary-rust-bytes:4entered4prechecks\n");
}
