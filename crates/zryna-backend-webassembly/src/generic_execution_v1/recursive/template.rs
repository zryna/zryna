//! Pinned source material, not an encoder-generated candidate body/signature table.
use super::{CLONE, DROP, Diagnostic, FIND, FUNCTIONS, INIT, SCHEMA, budget, invariant};
use crate::generic_execution_v1::work;
use sha2::{Digest, Sha256};
use wasmparser::{ExternalKind, FunctionBody, Parser, Payload, ValType, Validator, WasmFeatures};
pub(super) const BINARY: &[u8] = &decode_material();
// Controlled compiler sources must remain UTF-8. Decode the exact pinned material
// at Rust compile time; final Wasm bytes and runtime resource accounting are unchanged.
const fn decode_material() -> [u8; 2779] {
    let encoded = include_bytes!("kernel/traversal.hex");
    let mut decoded = [0u8; 2779];
    let mut source = 0;
    let mut destination = 0;
    let mut high = true;
    while source < encoded.len() {
        let byte = encoded[source];
        source += 1;
        if byte == b'\n' {
            continue;
        }
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("noncanonical traversal material hex"),
        };
        assert!(destination < decoded.len(), "oversized traversal material");
        if high {
            decoded[destination] = digit << 4;
        } else {
            decoded[destination] |= digit;
            destination += 1;
        }
        high = !high;
    }
    assert!(destination == decoded.len() && high, "truncated traversal material");
    decoded
}
const SOURCE: &[u8] = include_bytes!("kernel/traversal.wat");
const BINARY_SHA: &str = "9864830da5eeaa1bb754810613c040cbbd5db23446360af22b4184e0afd26801";
const SOURCE_SHA: &str = "d3aaed74b13e9f394363acac862f03b50a99a58aae1a3df226cb84a1a21b5540";
pub(super) fn load(spent: &mut usize) -> Result<Vec<FunctionBody<'static>>, Diagnostic> {
    work::take(
        spent,
        BINARY.len().checked_mul(4).and_then(|n| n.checked_add(SOURCE.len())).ok_or_else(budget)?,
    )?;
    if format!("{:x}", Sha256::digest(BINARY)) != BINARY_SHA
        || format!("{:x}", Sha256::digest(SOURCE)) != SOURCE_SHA
    {
        return Err(invariant());
    }
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(BINARY)
        .map_err(|_| invariant())?;
    work::retained::<FunctionBody<'static>>(spent, FUNCTIONS as usize)?;
    let mut bodies = Vec::new();
    bodies.try_reserve_exact(FUNCTIONS as usize).map_err(|_| budget())?;
    let mut exports = false;
    for item in Parser::new(0).parse_all(BINARY) {
        match item.map_err(|_| invariant())? {
            Payload::ExportSection(reader) => {
                if exports || reader.count() != 4 {
                    return Err(invariant());
                }
                exports = true;
                for (actual, (name, index)) in reader.into_iter().zip([
                    ("find", FIND),
                    ("init", INIT),
                    ("drop", DROP),
                    ("clone", CLONE),
                ]) {
                    let actual = actual.map_err(|_| invariant())?;
                    if actual.name != name
                        || actual.kind != ExternalKind::Func
                        || actual.index != 10 + index
                    {
                        return Err(invariant());
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let index = bodies.len();
                let (_, _, _, locals) = SCHEMA.get(index).copied().ok_or_else(invariant)?;
                let mut reader = body.get_locals_reader().map_err(|_| invariant())?;
                if locals == 0 {
                    if reader.get_count() != 0 {
                        return Err(invariant());
                    }
                } else if reader.get_count() != 1
                    || reader.read().map_err(|_| invariant())? != (locals, ValType::I32)
                {
                    return Err(invariant());
                }
                bodies.push(body);
            }
            _ => {}
        }
    }
    if !exports || bodies.len() != FUNCTIONS as usize {
        return Err(invariant());
    }
    Ok(bodies)
}
