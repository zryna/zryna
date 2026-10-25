//! Independently specified raw claims and immutable literal fixtures.
use zryna_ir::generic_v1::{owned_v2 as owned, raw as core};
use zryna_source::UntrustedSpan;

pub(super) const VECTOR: &[u8] =
    &decode_hex::<440>(include_str!("../../../../tests/m7-generic-struct-v4/wire-v4.hex"));
pub(super) const CORE: &[u8] =
    &decode_hex::<343>(include_str!("../../../../tests/m7-generic-struct-v4/embedded-core-v1.hex"));
pub(super) const FROZEN_COPY: &[u8] =
    include_bytes!("../../../../tests/m7-generic-copy-fixtures/wire-v1.zir");
pub(super) const OLD_V2: &[u8] =
    &decode_hex::<191>(include_str!("../../../../tests/m7-generic-struct-v4/wire-owned-v2.hex"));
pub(super) const OLD_V3: &[u8] =
    &decode_hex::<191>(include_str!("../../../../tests/m7-generic-struct-v4/wire-owned-v3.hex"));

// Decode immutable literals at compile time, independently of production wire encoders.
// The fixed output size rejects short, odd-length, and oversized vectors.
const fn decode_hex<const N: usize>(text: &str) -> [u8; N] {
    let text = text.as_bytes();
    let mut bytes = [0; N];
    let mut index = 0;
    let mut digits = 0;
    while index < text.len() {
        let digit = match text[index] {
            b'0'..=b'9' => text[index] - b'0',
            b'a'..=b'f' => text[index] - b'a' + 10,
            b' ' | b'\t' | b'\r' | b'\n' => {
                index += 1;
                continue;
            }
            _ => panic!("invalid literal hex digit"),
        };
        assert!(digits < N * 2, "literal hex exceeds fixed byte length");
        bytes[digits / 2] = (bytes[digits / 2] << 4) | digit;
        digits += 1;
        index += 1;
    }
    assert!(digits == N * 2, "literal hex has wrong byte length");
    bytes
}

fn at(start: u32, end: u32) -> UntrustedSpan {
    UntrustedSpan { file: 0, start, end }
}
fn ins(id: u32, ty: u32, start: u32, end: u32, operation: core::Operation) -> core::Instruction {
    core::Instruction {
        result: core::Definition { id, ty: core::Type::Stored(ty) },
        span: at(start, end),
        operation,
    }
}
pub(super) fn literal_claim() -> owned::raw::Program {
    owned::raw::Program {
        graph: core::Program {
            modules: vec![core::Module { id: 0, functions: 1 }],
            declarations: vec![core::Declaration {
                module: 0,
                function: 0,
                parameters: 0,
                span: at(70, 159),
            }],
            type_keys: vec![
                vec![0],
                vec![1],
                vec![2],
                vec![0x12, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1],
            ],
            universe: [11; 32],
            linear32: [22; 32],
            linux_x86_64: [33; 32],
            functions: vec![core::Function {
                key: vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0],
                span: at(70, 159),
                public_export: None,
                parameters: vec![],
                result: core::Type::Stored(1),
                blocks: vec![core::Block {
                    id: 0,
                    parameters: vec![],
                    instructions: vec![
                        ins(0, 1, 134, 135, core::Operation::I32Literal(7)),
                        ins(1, 3, 116, 138, core::Operation::Unit),
                        ins(2, 1, 147, 156, core::Operation::Unit),
                    ],
                    span: at(140, 157),
                    terminator: core::Terminator::Return(2),
                }],
            }],
        },
        extensions: vec![vec![
            owned::raw::Extension {
                result: 1,
                operation: owned::raw::Operation::StructConstruct { ty: 3, fields: vec![0] },
            },
            owned::raw::Extension {
                result: 2,
                operation: owned::raw::Operation::CopyStructField { value: 1, ordinal: 0 },
            },
        ]],
        plans: vec![owned::raw::Plan {
            steps: vec![owned::raw::Step {
                block: 0,
                position: 3,
                failure: false,
                end_loans: vec![],
                cleanup: vec![],
            }],
        }],
    }
}
pub(super) fn empty_claim() -> owned::raw::Program {
    let mut c = literal_claim();
    c.graph.modules.clear();
    c.graph.declarations.clear();
    c.graph.type_keys.clear();
    c.graph.functions.clear();
    c.extensions.clear();
    c.plans.clear();
    c
}
pub(super) fn rebrand(bytes: &[u8], header: &[u8], version: u32) -> Vec<u8> {
    assert_eq!(header.len(), 26);
    let mut b = bytes.to_vec();
    b[..26].copy_from_slice(header);
    b[26..30].copy_from_slice(&version.to_le_bytes());
    b
}
