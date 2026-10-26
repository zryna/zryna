//! Private raw module surgery; bytes are never promoted to a validated artifact.
use wasm_encoder::{Encode, Module, RawSection};
use wasmparser::{BinaryReader, Parser, Payload};
pub(super) fn sections(bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
    Parser::new(0)
        .parse_all(bytes)
        .filter_map(|p| {
            p.expect("actual audited module")
                .as_section()
                .map(|(id, r)| (id, slice(bytes, r).to_vec()))
        })
        .collect()
}
pub(super) fn counted(bytes: &[u8]) -> (u32, &[u8]) {
    let mut r = BinaryReader::new(bytes, 0);
    let count = r.read_var_u32().expect("bounded section count");
    (count, &bytes[offset(r.original_position())..])
}
pub(super) fn append(bytes: &[u8], extra: u32, tail: &[u8]) -> Vec<u8> {
    let (count, body) = counted(bytes);
    let mut out = Vec::new();
    count.checked_add(extra).expect("fixed small test additions").encode(&mut out);
    out.extend_from_slice(body);
    out.extend_from_slice(tail);
    out
}
pub(super) fn bodies(bytes: &[u8]) -> Vec<Vec<u8>> {
    Parser::new(0)
        .parse_all(bytes)
        .filter_map(|p| match p.expect("actual body") {
            Payload::CodeSectionEntry(body) => Some(slice(bytes, body.range()).to_vec()),
            _ => None,
        })
        .collect()
}
pub(super) fn code(bodies: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    u32::try_from(bodies.len()).expect("bounded test code count").encode(&mut out);
    for body in bodies {
        u32::try_from(body.len()).expect("bounded body size").encode(&mut out);
        out.extend_from_slice(body);
    }
    out
}
pub(super) fn module(sections: Vec<(u8, Vec<u8>)>) -> Vec<u8> {
    let mut module = Module::new();
    for (id, data) in sections {
        module.section(&RawSection { id, data: &data });
    }
    module.finish()
}

pub(super) fn offset(value: u64) -> usize {
    usize::try_from(value).expect("validated module position fits the host index")
}
fn slice(bytes: &[u8], range: std::ops::Range<u64>) -> &[u8] {
    &bytes[offset(range.start)..offset(range.end)]
}
