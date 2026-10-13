//! Five actual attempts through the explicit current owner profile; code admission pending.
use super::*;
fn control(session: &Session<'_>, case: Case) {
    assert!(
        session.diagnostics.is_empty(),
        "helper/comparator/cleanup errors are not control credit"
    );
    assert!(session.targets_checked);
    let output = session
        .records
        .iter()
        .find(|r| r.name == "run")
        .and_then(|r| r.output.as_ref())
        .expect("complete actual Output; helper Err is unavailable");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let text = std::str::from_utf8(&output.stderr).expect("bounded actual sanitizer diagnostic");
    match case {
        Case::HeapOverrun => {
            assert!(text.contains("m5-c-sanitizer:heap-overrun\n"));
            assert!(text.contains("AddressSanitizer: heap-buffer-overflow"));
            assert!(text.contains("READ of size 1"));
        }
        Case::SignedOverflow => {
            assert!(text.contains("m5-c-sanitizer:signed-overflow\n"));
            assert!(text.contains("runtime error: signed integer overflow:"));
        }
        Case::Positive => panic!("positive is not a negative control"),
    }
}
pub(super) fn measure(expected_parent: &Path) -> Result<serde_json::Value, Diagnostic> {
    let parent = material::evidence_parent().expect("external sanitizer evidence parent");
    assert_eq!(parent.as_path(), expected_parent, "owned evidence parent binding");
    let tools = material::OrdinaryTools::acquire().expect("actual selected ordinary C tools");
    let profile = policy::RuntimeProfile::acquire(&tools)?;
    let measured =
        rust_measurement::fresh(&tools, &parent).expect("genuine new pristine Rust prerequisite");
    let original = capture::capture();
    let issued = capture::issue(&original);
    let requirements =
        handle_link_requirements(&issued.byte_import).expect("original genuine issuer");
    let library = measured.capture(&requirements).expect("original unchanged profile data");
    let request =
        ImportRequest { requirements: &requirements, library: &library, measurement: &measured };
    let rust_sha = sha(&measured.object);
    let zryna_sha = sha(requirements.object().bytes());
    let mut attempts = Vec::new();
    for case in
        [Case::Positive, Case::HeapOverrun, Case::Positive, Case::SignedOverflow, Case::Positive]
    {
        let (evidence, session) =
            observe::observe_reviewed(&request, &tools, &parent, case, profile.case(case)?)
                .expect("bounded observation retained");
        assert_eq!(sha(&measured.object), rust_sha);
        assert_eq!(sha(requirements.object().bytes()), zryna_sha);
        request.check().expect("original source/material issuer preserved");
        if case == Case::Positive {
            super::super::tests::successful(&session, b"ordinary-rust-bytes:4entered4prechecks\n");
        } else {
            control(&session, case);
        }
        profile.case(case)?.check(case, &tools)?;
        let receipt = material::read(&evidence.join("receipt.json"), 1024 * 1024)?;
        let run = session
            .records
            .iter()
            .find(|r| r.name == "run")
            .and_then(|r| r.output.as_ref())
            .ok_or_else(rejected)?;
        attempts.push(serde_json::json!({
            "case":case.name(), "evidence":evidence,
            "receipt_bytes":receipt.len(),"receipt_sha256":digest(&receipt),
            "records":session.records.iter().map(export::record).collect::<Vec<_>>(),
            "products":session.artifacts.iter().map(|(name,bytes)|serde_json::json!({
                "name":name,"bytes":bytes.len(),"sha256":digest(bytes)
            })).collect::<Vec<_>>(),
            "targets_checked":session.targets_checked,"diagnostics_empty":session.diagnostics.is_empty(),
            "actual_run_status_raw":std::os::unix::process::ExitStatusExt::into_raw(run.status),
            "actual_run_stdout":run.stdout,"actual_run_stderr":run.stderr,
            "detached_call_redirect_execution":"NEVER_RUN"
        }));
    }
    request.check()?;
    let receipt = material::read(&measured.evidence.join("receipt.json"), 1024 * 1024)?;
    Ok(serde_json::json!({
        "format":"zryna.m5.ordinary-explicit-sanitizer-five-attempt-observation.v1",
        "fresh_rust_evidence":measured.evidence,"fresh_rust_object_sha256":sha(&measured.object),
        "genuine_zryna_object_sha256":sha(requirements.object().bytes()),
        "source_bytes":measured.source.len(),"source_sha256":digest(&measured.source),
        "fresh_rust_receipt_bytes":receipt.len(),"fresh_rust_receipt_sha256":digest(&receipt),
        "rust_records":measured.records.iter().map(export::record).collect::<Vec<_>>(),
        "ordinary_tools":tools.report(),"attempts":attempts,"existing_assertions_completed":true,
        "sanitizer_instrumented":"C client/private runtime only","Rust_instrumented":false,
        "Zryna_instrumented":false,"private_recipe_qualified":false,"production_seal":false,
        "loader_selection_proven":false,"complete_input_use":false,"owning_runtime_qualification":"Requires authenticated owner and complete five-attempt receipts",
        "provider_script_references":"UNRESOLVED_OPAQUE_DATA",
        "profile_evidence":profile.report()
    }))
}
