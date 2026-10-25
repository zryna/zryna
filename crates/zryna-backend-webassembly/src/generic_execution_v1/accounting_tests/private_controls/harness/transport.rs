//! Byte-preserving untrusted test-module transport; no production authority.
use sha2::{Digest, Sha256};
use wasm_encoder::{Module, RawSection};
use wasmparser::{Parser, Payload};
pub(in crate::generic_execution_v1::accounting_tests) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(in crate::generic_execution_v1::accounting_tests) fn bodies(bytes: &[u8]) -> Vec<String> {
    Parser::new(0)
        .parse_all(bytes)
        .filter_map(|payload| match payload.expect("fixed module") {
            Payload::CodeSectionEntry(body) => {
                let range = body.range();
                Some(hash(
                    &bytes[usize::try_from(range.start).expect("body start")
                        ..usize::try_from(range.end).expect("body end")],
                ))
            }
            _ => None,
        })
        .collect()
}
pub(in crate::generic_execution_v1::accounting_tests) fn replace(
    bytes: &[u8],
    id: u8,
    change: impl FnOnce(&[u8]) -> Vec<u8>,
) -> Vec<u8> {
    let mut change = Some(change);
    let mut module = Module::new();
    for payload in Parser::new(0).parse_all(bytes) {
        if let Some((section, range)) = payload.expect("fixed module").as_section() {
            let bytes = &bytes[usize::try_from(range.start).expect("start")
                ..usize::try_from(range.end).expect("end")];
            if section == id {
                module.section(&RawSection {
                    id: section,
                    data: &change.take().expect("single section")(bytes),
                });
            } else {
                module.section(&RawSection { id: section, data: bytes });
            }
        }
    }
    module.finish()
}
pub(in crate::generic_execution_v1::accounting_tests) fn body_replacement(
    bytes: &[u8],
    target: usize,
    change: impl FnOnce(&[u8]) -> Vec<u8>,
) -> Vec<u8> {
    let mut change = Some(change);
    let mut encoded = crate::generic_execution_v1::bytes::Bytes::new();
    let mut bodies = Vec::new();
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::CodeSectionEntry(body) = payload.expect("actual code") {
            let range = body.range();
            bodies.push(
                bytes[usize::try_from(range.start).expect("start")
                    ..usize::try_from(range.end).expect("end")]
                    .to_vec(),
            );
        }
    }
    encoded.count(bodies.len()).expect("fixed code count");
    for (index, body) in bodies.into_iter().enumerate() {
        let body =
            if index == target { change.take().expect("one actual body")(&body) } else { body };
        encoded.count(body.len()).expect("fixed body length");
        encoded.extend(&body).expect("fixed body");
    }
    replace(bytes, 10, |_| encoded.bytes)
}

pub(in crate::generic_execution_v1::accounting_tests) fn type_map(
    bytes: &[u8],
) -> Vec<(u8, String)> {
    Parser::new(0)
        .parse_all(bytes)
        .filter_map(|payload| {
            let payload = payload.expect("actual module");
            payload.as_section().and_then(|(id, range)| {
                if id == 1 || id == 3 {
                    Some((
                        id,
                        hash(
                            &bytes[usize::try_from(range.start).expect("start")
                                ..usize::try_from(range.end).expect("end")],
                        ),
                    ))
                } else {
                    None
                }
            })
        })
        .collect()
}
