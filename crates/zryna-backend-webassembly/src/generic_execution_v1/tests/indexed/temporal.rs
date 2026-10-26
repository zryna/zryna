//! Genuine actual bytes cannot launder untracked source frame/result/index/PC rewrites.
use super::{artifact, code, memory, model, program, raw};
use std::ops::Range;
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};
mod fixture;
#[derive(Clone, Copy, Debug)]
enum Rewrite {
    IndexBefore,
    IndexAfter,
    ParameterSet(u32),
    ParameterTee(u32),
    PcTee,
}
#[test]
fn source_index_scratch_cannot_be_rewritten_before_or_after_its_checked_bounds() {
    for case in ["array-shared-copy", "copy-read-control"] {
        for action in [Rewrite::IndexBefore, Rewrite::IndexAfter] {
            fixture::reject(case, action);
        }
    }
}
#[test]
fn source_frame_and_result_pointer_parameters_cannot_be_set_or_tee_reassigned() {
    for parameter in [0, 1] {
        for action in [Rewrite::ParameterSet(parameter), Rewrite::ParameterTee(parameter)] {
            fixture::reject("array-shared-copy", action);
        }
    }
}
#[test]
fn source_pc_cannot_be_tee_reassigned_without_the_checked_edge_event() {
    fixture::reject("array-indexed-rhs-match", Rewrite::PcTee);
}
fn operators(bytes: &[u8], target: usize) -> Vec<(O<'_>, Range<usize>)> {
    let body = Parser::new(0)
        .parse_all(bytes)
        .filter_map(|payload| match payload.expect("actual sections") {
            Payload::CodeSectionEntry(body) => Some(body),
            _ => None,
        })
        .nth(target)
        .expect("genuine source body");
    let origin = body.range().start;
    let mut reader = body.get_operators_reader().expect("actual source operators");
    let mut ops = Vec::new();
    while !reader.eof() {
        let start = reader.original_position();
        let op = reader.read().expect("actual operator");
        ops.push((
            op,
            usize::try_from(start - origin).expect("bounded start")
                ..usize::try_from(reader.original_position() - origin).expect("bounded end"),
        ));
    }
    ops
}
fn bounds(ops: &[(O<'_>, Range<usize>)]) -> (usize, usize) {
    let tee = ops
        .iter()
        .position(|(op, _)| matches!(op, O::LocalTee { local_index: 5 }))
        .expect("actual index activation");
    let compare = tee
        + 1
        + ops[tee + 1..]
            .iter()
            .position(|(op, _)| matches!(op, O::I32GeU))
            .expect("actual unsigned bounds");
    assert!(matches!(ops[compare + 1].0, O::If { .. }));
    let mut depth = 1;
    for (op, range) in &ops[compare + 2..] {
        match op {
            O::If { .. } | O::Block { .. } | O::Loop { .. } => depth += 1,
            O::End => {
                depth -= 1;
                if depth == 0 {
                    return (ops[tee].1.end, range.end);
                }
            }
            _ => {}
        }
    }
    panic!("whole actual bounds guard");
}
