//! Genuine decoded carrier mutations cannot rely only on successful Wasm validation.
use super::*;
use wasm_encoder::{Module, RawSection};
pub(super) fn check(carrier: &ValidatedInvocationArtifact<'_, '_>) {
    let body = Parser::new(0)
        .parse_all(carrier.as_bytes())
        .filter_map(|payload| match payload.expect("actual module") {
            Payload::CodeSectionEntry(body) => Some(body),
            _ => None,
        })
        .last()
        .expect("private carrier is last function");
    let mut ops = body.get_operators_reader().expect("actual operators");
    let mut witnesses = [None; 4];
    while !ops.eof() {
        let at = usize::try_from(ops.original_position()).expect("bounded actual byte offset");
        match ops.read().expect("actual operator") {
            Operator::I64Const { value: 32 } => witnesses[0] = Some((at + 1, 31)),
            Operator::GlobalGet { global_index: 1 } => witnesses[1] = Some((at + 1, 0)),
            Operator::GlobalGet { global_index: 8 } => witnesses[2] = Some((at + 1, 9)),
            Operator::I32Const { value: 5 } => witnesses[3] = Some((at + 1, 4)),
            _ => {}
        }
    }
    for witness in witnesses {
        let (at, value) = witness.expect("source-status/cleanup/packing witness");
        let mut bad = carrier.as_bytes().to_vec();
        bad[at] = value;
        wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::WASM1)
            .validate_all(&bad)
            .expect("welltyped actual hostile bytes");
        assert!(carrier.audit_bytes(&bad).is_err());
    }
    let mut changed = Module::new();
    for payload in Parser::new(0).parse_all(carrier.as_bytes()) {
        if let Some((id, range)) = payload.expect("actual sections").as_section() {
            if id == 7 {
                // Exporting memory instead of the sole private function is never permitted.
                let mut data = vec![1, 6];
                data.extend_from_slice(b"memory");
                data.extend_from_slice(&[2, 0]);
                changed.section(&RawSection { id, data: &data });
            } else {
                let start = usize::try_from(range.start).expect("bounded section start");
                let end = usize::try_from(range.end).expect("bounded section end");
                changed.section(&RawSection { id, data: &carrier.as_bytes()[start..end] });
            }
        }
    }
    assert!(carrier.audit_bytes(&changed.finish()).is_err());
    let mut extra = carrier.as_bytes().to_vec();
    extra.push(0);
    assert!(carrier.audit_bytes(&extra).is_err());
    carrier.audit_bytes(carrier.as_bytes()).expect("pristine retry after mutations");
}
