//! Valid changed indexed operators cannot borrow authority from matching emitter body copies.
use super::{artifact, code, memory, model, program};
use std::ops::Range;
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};
#[derive(Clone, Copy, Debug)]
enum Attack {
    Bounds,
    Index,
    Cell,
    OldDrop,
}
#[test]
fn indexed_bounds_index_origin_child_cell_and_old_drop_are_independent_actual_byte_obligations() {
    for attack in [Attack::Bounds, Attack::Index, Attack::Cell, Attack::OldDrop] {
        let case = if matches!(attack, Attack::OldDrop) {
            "vec-indexed-replace"
        } else {
            "array-shared-copy"
        };
        program(case, "native", |program, _| {
            for observed in [false, true] {
                let plan = memory::verify(
                    memory::derive(program, observed).expect("genuine indexed memory"),
                    program,
                )
                .expect("independent memory");
                let m = model::Model::new(program, &plan).expect("same source model");
                let op = program
                    .operations()
                    .find(|op| op.indexed_loan().is_some())
                    .expect("actual indexed Begin");
                let view = op.indexed_loan().expect("bound indexed metadata");
                let mut code = code::build(&m).expect("actual indexed bodies");
                let pristine = artifact::encode(&m, &code).expect("actual module");
                artifact::independent_audit(&pristine, &m)
                    .expect("pristine independent byte owner");
                let target = usize::try_from(m.source_base).expect("bounded source index")
                    + op.site().function;
                let operators = operators(&pristine, target);
                let cell = m.memory.claims().functions[op.site().function].loans
                    [view.loan().id() as usize]
                    .offset;
                let drop = m.drop_index(view.loan().referent());
                let i32_type = m
                    .types
                    .iter()
                    .find(|ty| ty.category() == zryna_layout::TypeCategory::I32)
                    .expect("actual i32 leaf")
                    .id()
                    .index();
                let (at, replacement) = operators
                    .iter()
                    .enumerate()
                    .find_map(|(index, (op, _))| match (attack, op) {
                        (Attack::Bounds, O::I32GeU) => Some((index, I::I32GtU)),
                        (Attack::Index, O::LocalTee { local_index: 5 }) => {
                            Some((index, I::LocalTee(4)))
                        }
                        (Attack::Cell, O::I32Const { value }) if *value == cell.cast_signed() => {
                            Some((
                                index,
                                I::I32Const(
                                    cell.checked_add(4)
                                        .expect("bounded adjacent immediate")
                                        .cast_signed(),
                                ),
                            ))
                        }
                        (Attack::OldDrop, O::Call { function_index })
                            if *function_index == drop =>
                        {
                            Some((index, I::Call(m.drop_index(i32_type))))
                        }
                        _ => None,
                    })
                    .expect("actual owned operator must be changed");
                let mut replacement_bytes = Vec::new();
                replacement.encode(&mut replacement_bytes);
                code[target].body.bytes.splice(operators[at].1.clone(), replacement_bytes);
                let wrong =
                    artifact::encode(&m, &code).expect("matching hostile supplemental body");
                assert_ne!(wrong, pristine);
                Validator::new_with_features(WasmFeatures::WASM1)
                    .validate_all(&wrong)
                    .expect("changed immediate remains valid Wasm");
                assert!(artifact::independent_audit(&wrong, &m).is_err(), "independent {attack:?}");
                assert!(artifact::audit(&wrong, &m, &code).is_err(), "full {attack:?}");
                let retry = code::build(&m).expect("pristine retry after rejected hostile");
                artifact::audit(&pristine, &m, &retry)
                    .expect("failed proof publishes no authority");
            }
        });
    }
}
fn operators(bytes: &[u8], target: usize) -> Vec<(O<'_>, Range<usize>)> {
    let body = Parser::new(0)
        .parse_all(bytes)
        .filter_map(|payload| match payload.expect("actual sections") {
            Payload::CodeSectionEntry(body) => Some(body),
            _ => None,
        })
        .nth(target)
        .expect("actual source body");
    let origin = body.range().start;
    let mut reader = body.get_operators_reader().expect("decoded actual operations");
    let mut out = Vec::new();
    while !reader.eof() {
        let start = reader.original_position();
        let op = reader.read().expect("actual operator");
        out.push((
            op,
            usize::try_from(start - origin).expect("bounded start")
                ..usize::try_from(reader.original_position() - origin).expect("bounded end"),
        ));
    }
    out
}
