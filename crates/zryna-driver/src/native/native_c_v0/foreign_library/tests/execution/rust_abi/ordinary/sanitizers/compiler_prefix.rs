//! Compile/inspection-only ordinary observations; unavailable policy prevents all target runs.
use super::*;
use object::{Object as _, ObjectSymbol as _};
pub(super) fn measure(parent: &Path) -> Result<serde_json::Value, Diagnostic> {
    let tools = material::OrdinaryTools::acquire()?;
    let measured = rust_measurement::fresh(&tools, parent)?;
    let original = capture::capture();
    let issued = capture::issue(&original);
    let requirements = handle_link_requirements(&issued.byte_import)?;
    let library = measured.capture(&requirements)?;
    let request =
        ImportRequest { requirements: &requirements, library: &library, measurement: &measured };
    let rust_hash = sha(&measured.object);
    let zryna_hash = sha(requirements.object().bytes());
    let mut cases = Vec::new();
    for case in [Case::Positive, Case::HeapOverrun, Case::SignedOverflow] {
        if policy::reviewed(case).is_some() {
            return Err(rejected());
        }
        request.check()?;
        let (evidence, session) = observe::observe(&request, &tools, parent, case)?;
        if session.targets_checked
            || session.phase != "unavailable-independent-sanitizer-bindings"
            || session.diagnostics != [rejected()]
            || session.records.iter().map(|r| r.name).collect::<Vec<_>>()
                != ["compile-client", "compile-runtime", "link", "inspect-shim"]
        {
            return Err(rejected());
        }
        let observed = observation::Observation::capture(&session)?;
        for name in [
            "program.o",
            "foreign.o",
            "client.c",
            "private-runtime.c",
            "private-runtime-header.h",
            "compile-client.o",
            "compile-runtime.o",
            "final.elf",
        ] {
            if observed.product(name)?.is_empty() {
                return Err(rejected());
            }
        }
        let final_bytes = observed.product("final.elf")?;
        let file = machine::parse(final_bytes)?;
        let mut undefined = file
            .dynamic_symbols()
            .filter(object::ObjectSymbol::is_undefined)
            .map(|s| s.name().map(str::to_owned).map_err(|_| rejected()))
            .collect::<Result<Vec<_>, _>>()?;
        undefined.retain(|s| !s.is_empty());
        undefined.sort();
        request.check()?;
        if sha(&measured.object) != rust_hash || sha(requirements.object().bytes()) != zryna_hash {
            return Err(rejected());
        }
        let receipt = material::read(&evidence.join("receipt.json"), 1024 * 1024)?;
        cases.push(serde_json::json!({
            "case":case.name(), "evidence":evidence,
            "receipt_bytes":receipt.len(), "receipt_sha256":digest(&receipt),
            "compiler_inspection_records_complete":true,
            "records":session.records.iter().map(export::record).collect::<Vec<_>>(),
            "products":session.artifacts.iter().map(|(name,bytes)|serde_json::json!({
                "name":name,"bytes":bytes.len(),"sha256":digest(bytes)
            })).collect::<Vec<_>>(),
            "observed_undefined_dynamic_symbols":undefined,
            "independently_reviewed_bindings":false,
            "expected_none_refusal":"ZRYNA-C4102",
            "phase":session.phase,"targets_checked":false,"sanitizer_target_execution":"UNRUN",
            "cleanup_errors":false,"whole_sanitizer_test_pass":false
        }));
    }
    request.check()?;
    let receipt = material::read(&measured.evidence.join("receipt.json"), 1024 * 1024)?;
    Ok(serde_json::json!({
        "format":"zryna.m5.ordinary-initial-sanitizer-compiler-observation.v1",
        "source_bytes":measured.source.len(),"source_sha256":digest(&measured.source),
        "fresh_rust_evidence":measured.evidence,
        "fresh_rust_receipt_bytes":receipt.len(),"fresh_rust_receipt_sha256":digest(&receipt),
        "rust_records":measured.records.iter().map(export::record).collect::<Vec<_>>(),
        "rust_observations":measured.rust_observations,
        "ordinary_tools":tools.report(),"fresh_rust_object_sha256":digest(&measured.object),
        "genuine_zryna_object_sha256":digest(requirements.object().bytes()),
        "cases":cases,"policy":"None","targets_checked":false,
        "sanitizer_target_execution":"UNRUN","test_pass":false,
        "scope":"Fresh ordinary trusted compiler/inspection materials only; no Rust/Zryna instrumentation, production/native/private recipe/host/SQLite/sanitizer execution grant"
    }))
}
