//! Three nonignored actual Rust result-fault selectors, each with fresh pristine recovery.
use super::*;
use std::os::unix::process::ExitStatusExt as _;

fn prepare() -> (material::OrdinaryTools, PathBuf, HandleLinkRequirements) {
    let parent = material::evidence_parent().expect("external result-fault evidence parent");
    let tools = material::OrdinaryTools::acquire().expect("fresh ordinary C tools before baseline");
    let captured = capture::capture();
    let issued = capture::issue(&captured);
    let requirements =
        handle_link_requirements(&issued.byte_import).expect("original source issuer");
    (tools, parent, requirements)
}
fn recovery(tools: &material::OrdinaryTools, parent: &Path, requirements: &HandleLinkRequirements) {
    let pristine =
        rust_measurement::fresh(tools, parent).expect("new pristine Rust compile after control");
    let library = pristine.capture(requirements).expect("pristine original selected capture");
    let request = ImportRequest { requirements, library: &library, measurement: &pristine };
    let (_, session) =
        observe_import(&request, tools, parent, Mode::Bytes { missing_increment: false })
            .expect("actual fresh pristine byte recovery");
    super::super::tests::successful(&session, b"ordinary-rust-bytes:4entered4prechecks\n");
}
fn rejected_reissue(request: &Request<'_>, tools: &material::OrdinaryTools, parent: &Path) {
    let independent = capture::capture();
    let reissued = handle_link_requirements(&capture::issue(&independent).byte_import)
        .expect("independent equal-document genuine issuer");
    assert_eq!(reissued.object_sha256(), request.requirements.object_sha256());
    let before = process::attempts();
    assert!(
        observe::run(
            &Request {
                requirements: &reissued,
                library: request.library,
                hostile: request.hostile
            },
            tools,
            parent
        )
        .is_err()
    );
    assert_eq!(process::attempts(), before, "reissued source refused before new C effects");
    request.check().expect("original hostile observed source/capture remains bound");
}
fn fault(kind: Kind) {
    let (tools, parent, requirements) = prepare();
    let hostile =
        measurement::fresh(&tools, &parent, kind).expect("actual closed hostile Rust compile");
    assert_ne!(hostile.source(), capture::FOREIGN_RUST_SOURCE.as_bytes());
    assert!(material::check_source(hostile.source()).is_err(), "not pristine source admission");
    assert!(
        targets::compare_functions(reference::ORIGINAL_RUST_OBJECT, hostile.object()).is_err(),
        "hostile actual code never inherits pristine function certificate"
    );
    let library = hostile.capture(&requirements).expect("unchanged data-profile hostile capture");
    let request = Request { requirements: &requirements, library: &library, hostile: &hostile };
    rejected_reissue(&request, &tools, &parent);
    let (_, session) =
        observe::run(&request, &tools, &parent).expect("actual hostile Rust observer");
    assert!(session.targets_checked && session.measurement.is_none());
    let marker = format!("ordinary-rust-fault:{}\n", kind.name());
    super::super::tests::successful(&session, marker.as_bytes());
    recovery(&tools, &parent, &requirements);
}
#[test]
fn ordinary_rust_unknown_status_is_host_abi_failure_without_output_exposure() {
    fault(Kind::UnknownStatus);
}
#[test]
fn ordinary_rust_error_status_output_write_is_host_abi_failure() {
    fault(Kind::WriteOnError);
}
#[test]
fn rust_failure_oracle_rejects_nonfaulting_control_then_pristine_recovers() {
    let (tools, parent, requirements) = prepare();
    let actual = measurement::fresh(&tools, &parent, Kind::NonfaultingControl)
        .expect("genuine new nonfaulting Rust source compile");
    material::check_source(actual.source()).expect("nonfaulting control source is original");
    targets::compare_functions(reference::ORIGINAL_RUST_OBJECT, actual.object())
        .expect("actual nonfaulting source function baseline");
    let library = actual.capture(&requirements).expect("separate control-origin data capture");
    let request = Request { requirements: &requirements, library: &library, hostile: &actual };
    let (_, session) =
        observe::run(&request, &tools, &parent).expect("actual fault-oracle sensitivity run");
    assert!(session.diagnostics.is_empty() && session.targets_checked);
    let output = session
        .records
        .iter()
        .find(|r| r.name == "run")
        .and_then(|r| r.output.as_ref())
        .expect("real nonzero sensitivity target result, not compilation/comparator failure");
    assert_eq!(output.status.signal(), Some(libc::SIGABRT));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fault_tag==3"));
    assert!(output.stdout.is_empty());
    recovery(&tools, &parent, &requirements);
}
