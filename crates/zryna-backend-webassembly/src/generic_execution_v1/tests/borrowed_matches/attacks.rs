//! Changed valid actual operators cannot launder a selected-arm loan with supplemental bodies.
use super::{artifact, code, memory, model, raw, source};
use std::ops::Range;
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{Operator as O, Parser, Payload, Validator, WasmFeatures};
struct Selection {
    function: usize,
    cell: u32,
    payload: u32,
    ordinal: u32,
    other_ordinal: u32,
    target: u32,
    other_target: u32,
}
#[derive(Clone, Copy, Debug)]
enum Attack {
    Destination,
    Payload,
    Ordinal,
    Successor,
}
#[test]
fn selected_payload_address_cell_tag_and_successor_are_actual_decoded_obligations() {
    for provider in source::PROVIDERS {
        source::with_program("nominal-exclusive-copy", provider, |program, _| {
            for observed in [false, true] {
                let plan = memory::verify(
                    memory::derive(program, observed).expect("genuine address claim"),
                    program,
                )
                .expect("same-source frame geometry");
                let model = model::Model::new(program, &plan).expect("exact retained types");
                let selection = selected(&model);
                for attack in
                    [Attack::Destination, Attack::Payload, Attack::Ordinal, Attack::Successor]
                {
                    reject(attack, &selection, &model);
                }
            }
        });
    }
}
fn selected(model: &model::Model<'_, '_, '_>) -> Selection {
    for function in &model.functions {
        for block in function.blocks() {
            let raw::Terminator::ClosedEnumMatch { ty, mode, arms, .. } = block.terminator() else {
                continue;
            };
            if *mode != raw::MatchMode::ExclusiveLoan {
                continue;
            }
            for arm in arms {
                if let Some(raw::MatchBinding::Loan(binding)) = &arm.binding {
                    let other =
                        arms.iter().find(|a| a.ordinal != arm.ordinal).expect("other variant");
                    return Selection {
                        function: function.index(),
                        cell: model.memory.claims().functions[function.index()].loans
                            [binding.id as usize]
                            .offset,
                        payload: u32::try_from(
                            model.types[*ty as usize].payload().expect("enum payload").0,
                        )
                        .expect("actual bounded payload offset"),
                        ordinal: arm.ordinal,
                        other_ordinal: other.ordinal,
                        target: arm.edge.target,
                        other_target: other.edge.target,
                    };
                }
            }
        }
    }
    panic!("genuine exclusive selected payload");
}
fn reject(attack: Attack, selection: &Selection, model: &model::Model<'_, '_, '_>) {
    let mut code = code::build(model).expect("actual pristine borrowed source code");
    let pristine = artifact::encode(model, &code).expect("actual pristine module");
    artifact::independent_audit(&pristine, model).expect("independent pristine owner");
    artifact::audit(&pristine, model, &code).expect("full pristine owner");
    let index = model.source_base as usize + selection.function;
    let operators = operators(&pristine, index);
    let (at, replacement) = mutation(attack, selection, &operators);
    let mut encoded = Vec::new();
    replacement.encode(&mut encoded);
    code[index].body.bytes.splice(operators[at].1.clone(), encoded);
    let wrong = artifact::encode(model, &code).expect("matching changed supplemental body");
    assert_ne!(wrong, pristine, "hostile must change final module bytes");
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(&wrong)
        .expect("changed address/control immediate remains structurally valid Wasm");
    assert!(artifact::independent_audit(&wrong, model).is_err(), "independent {attack:?}");
    assert!(artifact::audit(&wrong, model, &code).is_err(), "full artifact {attack:?}");
    let retry = code::build(model).expect("pristine code after hostile audit failure");
    artifact::audit(&pristine, model, &retry)
        .expect("failed decoded proof exposes no new authority");
}
fn operators(bytes: &[u8], function: usize) -> Vec<(O<'_>, Range<usize>)> {
    let body = Parser::new(0)
        .parse_all(bytes)
        .filter_map(|p| match p.expect("actual sections") {
            Payload::CodeSectionEntry(body) => Some(body),
            _ => None,
        })
        .nth(function)
        .expect("selected genuine source role");
    let origin = body.range().start;
    let mut reader = body.get_operators_reader().expect("actual source body");
    let mut result = Vec::new();
    while !reader.eof() {
        let start = reader.original_position();
        let op = reader.read().expect("valid operator");
        let begin = usize::try_from(start - origin)
            .expect("actual function body offset fits host indexing");
        let end = usize::try_from(reader.original_position() - origin)
            .expect("actual function body end fits host indexing");
        result.push((op, begin..end));
    }
    result
}
fn binding_store(selection: &Selection, ops: &[(O<'_>, Range<usize>)]) -> (usize, usize) {
    for (start, window) in ops.windows(3).enumerate() {
        if matches!(window[0].0, O::LocalGet { local_index: 0 })
            && matches!(window[1].0, O::I32Const { value } if value == selection.cell.cast_signed())
            && matches!(window[2].0, O::I32Add)
        {
            let store = ops
                .iter()
                .enumerate()
                .skip(start + 3)
                .find(|(_, (op, _))| matches!(op, O::I32Store { .. }))
                .expect("exact cell store")
                .0;
            if !matches!(ops[store - 1].0, O::I32Const { value: 0 }) {
                return (start, store);
            }
        }
    }
    panic!("selected generated binding publishes a payload address into its own cell");
}
fn mutation(
    attack: Attack,
    selection: &Selection,
    ops: &[(O<'_>, Range<usize>)],
) -> (usize, I<'static>) {
    let (start, store) = binding_store(selection, ops);
    match attack {
        Attack::Destination => {
            let O::I32Store { memarg } = ops[store].0 else { unreachable!() };
            assert_eq!(memarg.offset, 0);
            (store, I::I32Store(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }))
        }
        Attack::Payload => {
            let delta = ops[start + 3..store].windows(2).position(|w| {
                matches!(w[0].0, O::I32Const { value } if value == selection.payload.cast_signed())
                    && matches!(w[1].0, O::I32Add)
            }).expect("exact sealed enum payload address addition");
            (start + 3 + delta, I::I32Const(selection.payload.cast_signed() + 4))
        }
        Attack::Ordinal => {
            let tag = ops[..start].windows(4).rposition(|w| {
                matches!(w[0].0, O::I32Load { .. })
                    && matches!(w[1].0, O::I32Const { value } if value == selection.ordinal.cast_signed())
                    && matches!(w[2].0, O::I32Eq)
                    && matches!(w[3].0, O::If { .. })
            }).expect("selected ordinal guard before payload publication");
            (tag + 1, I::I32Const(selection.other_ordinal.cast_signed()))
        }
        Attack::Successor => {
            let edge = ops[store + 1..].windows(3).position(|w| {
                matches!(w[0].0, O::I32Const { value } if value == selection.target.cast_signed())
                    && matches!(w[1].0, O::LocalSet { local_index: 2 })
                    && matches!(w[2].0, O::Br { .. })
            }).expect("selected arm ordinary successor after binding");
            (store + 1 + edge, I::I32Const(selection.other_target.cast_signed()))
        }
    }
}
