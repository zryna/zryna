use std::collections::BTreeMap;

use zryna_ownership_runtime_abi::LogicalOperation;

use super::{Errors, raw, type_at};

pub(super) fn verify_types(
    program: &raw::Program,
    function: &raw::Function,
    operation: &raw::Operation,
    borrow_types: &[u32],
    errors: &mut Errors,
) {
    let result_type = operation.result.as_ref().map(|value| value.ty);
    let first_value_type = operation.values.first().and_then(|id| value_type(function, *id));
    let first_place_type = operation
        .places
        .first()
        .and_then(|id| function.places.get(*id as usize))
        .map(|place| place.ty);
    let first_borrow_type =
        operation.borrows.first().and_then(|id| borrow_types.get(*id as usize)).copied();
    let valid = match operation.opcode {
        raw::Opcode::BeginBorrow => operation.borrow_type == first_place_type,
        raw::Opcode::BeginIndexedBorrow => {
            first_place_type.and_then(|ty| type_at(program, ty)).and_then(|ty| ty.referenced_type)
                == operation.borrow_type
        }
        raw::Opcode::ProjectBorrow => {
            first_borrow_type.and_then(|ty| type_at(program, ty)).and_then(|ty| ty.referenced_type)
                == operation.borrow_type
        }
        raw::Opcode::BindBorrow => first_borrow_type == operation.borrow_type,
        raw::Opcode::BorrowRead => first_borrow_type == result_type,
        raw::Opcode::BorrowWrite | raw::Opcode::BorrowReplace => {
            first_borrow_type == first_value_type && operation.borrow_type == first_borrow_type
        }
        raw::Opcode::EndBorrow => operation.borrow_type == first_borrow_type,
        raw::Opcode::Clone | raw::Opcode::StringClone | raw::Opcode::VecClone
            if !operation.borrows.is_empty() =>
        {
            first_borrow_type == result_type
        }
        _ => true,
    };
    if !valid {
        errors.push("ZRYNA-N3107", "native MIR operation type relation is not exact");
    }
}

fn value_type(function: &raw::Function, id: u32) -> Option<u32> {
    function
        .parameters
        .iter()
        .chain(function.blocks.iter().flat_map(|block| block.parameters.iter()))
        .chain(function.blocks.iter().flat_map(|block| {
            block.operations.iter().filter_map(|operation| operation.result.as_ref())
        }))
        .find(|value| value.id == id)
        .map(|value| value.ty)
}

pub(super) fn verify_runtime(
    operation: &raw::Operation,
    symbols: &BTreeMap<LogicalOperation, &str>,
    errors: &mut Errors,
) {
    let expected = match operation.opcode {
        raw::Opcode::String => Some(LogicalOperation::StringFromUtf8Copy),
        raw::Opcode::StringClone => Some(LogicalOperation::StringClone),
        raw::Opcode::StringConcat => Some(LogicalOperation::StringConcat),
        raw::Opcode::VecConstruct => Some(LogicalOperation::VecAllocate),
        raw::Opcode::VecPush => Some(LogicalOperation::VecReserve),
        raw::Opcode::SharedClone => Some(LogicalOperation::StrongClone),
        raw::Opcode::WeakDowngrade => Some(LogicalOperation::WeakDowngrade),
        raw::Opcode::WeakClone => Some(LogicalOperation::WeakClone),
        _ => None,
    };
    if expected.and_then(|kind| symbols.get(&kind).copied()) != operation.runtime_symbol.as_deref()
    {
        errors.push("ZRYNA-N3110", "native MIR operation has a missing or unapproved runtime call");
    }
}

pub(super) fn verify_shape(operation: &raw::Operation, errors: &mut Errors) {
    let (values, places, borrows, result) = expected_shape(operation.opcode);
    let one_source = operation.opcode != raw::Opcode::Copy
        && operation.opcode != raw::Opcode::Move
        && !matches!(
            operation.opcode,
            raw::Opcode::Clone | raw::Opcode::StringClone | raw::Opcode::VecClone
        )
        || operation.places.len() + operation.borrows.len() == 1;
    if values.is_some_and(|count| operation.values.len() != count)
        || places.is_some_and(|count| operation.places.len() != count)
        || borrows.is_some_and(|count| operation.borrows.len() != count)
        || operation.result.is_some() != result
        || operation.callee.is_some() != (operation.opcode == raw::Opcode::Call)
        || !one_source
        || !has_canonical_call_arguments(operation)
        || operation.borrow_type.is_some() != needs_borrow_type(operation)
        || !has_canonical_immediate(operation)
    {
        errors.push("ZRYNA-N3107", "native MIR operation operand shape is not canonical");
    }
}

fn expected_shape(opcode: raw::Opcode) -> (Option<usize>, Option<usize>, Option<usize>, bool) {
    match opcode {
        raw::Opcode::BoolLiteral | raw::Opcode::I32Literal | raw::Opcode::String => {
            (Some(0), Some(0), Some(0), true)
        }
        raw::Opcode::I32Add
        | raw::Opcode::I32Sub
        | raw::Opcode::I32Mul
        | raw::Opcode::Eq
        | raw::Opcode::Ne
        | raw::Opcode::I32LtS
        | raw::Opcode::I32LeS
        | raw::Opcode::I32GtS
        | raw::Opcode::I32GeS => (Some(2), Some(0), Some(0), true),
        raw::Opcode::I32Neg | raw::Opcode::SharedConstruct => (Some(1), Some(0), Some(0), true),
        raw::Opcode::Call | raw::Opcode::Construct | raw::Opcode::VecConstruct => {
            (None, Some(0), None, true)
        }
        raw::Opcode::Copy
        | raw::Opcode::Move
        | raw::Opcode::Clone
        | raw::Opcode::StringClone
        | raw::Opcode::VecClone => (Some(0), None, None, true),
        raw::Opcode::Initialize | raw::Opcode::Replace | raw::Opcode::VecPush => {
            (Some(1), Some(1), Some(0), false)
        }
        raw::Opcode::Drop => (Some(0), Some(1), Some(0), false),
        raw::Opcode::Discriminant => (Some(0), Some(1), Some(0), true),
        raw::Opcode::Index => (Some(1), Some(1), Some(0), true),
        raw::Opcode::StringConcat => (Some(0), Some(2), Some(0), true),
        raw::Opcode::SharedClone | raw::Opcode::WeakDowngrade | raw::Opcode::WeakClone => {
            (Some(0), Some(1), Some(0), true)
        }
        raw::Opcode::BeginBorrow => (Some(0), Some(1), Some(1), false),
        raw::Opcode::BeginIndexedBorrow => (Some(1), Some(1), Some(1), false),
        raw::Opcode::ProjectBorrow => (Some(1), Some(0), Some(2), false),
        raw::Opcode::BindBorrow => (Some(0), Some(0), Some(2), false),
        raw::Opcode::BorrowReplace | raw::Opcode::BorrowWrite => (Some(1), Some(0), Some(1), false),
        raw::Opcode::BorrowRead => (Some(0), Some(0), Some(1), true),
        raw::Opcode::EndBorrow => (Some(0), Some(0), Some(1), false),
    }
}

fn has_canonical_call_arguments(operation: &raw::Operation) -> bool {
    if operation.opcode == raw::Opcode::Call {
        operation
            .call_arguments
            .iter()
            .filter_map(|argument| match argument {
                raw::CallArgument::Value(id) => Some(*id),
                raw::CallArgument::Borrow(_) => None,
            })
            .eq(operation.values.iter().copied())
            && operation
                .call_arguments
                .iter()
                .filter_map(|argument| match argument {
                    raw::CallArgument::Value(_) => None,
                    raw::CallArgument::Borrow(id) => Some(*id),
                })
                .eq(operation.borrows.iter().copied())
    } else {
        operation.call_arguments.is_empty()
    }
}

fn needs_borrow_type(operation: &raw::Operation) -> bool {
    matches!(
        operation.opcode,
        raw::Opcode::BeginBorrow
            | raw::Opcode::BeginIndexedBorrow
            | raw::Opcode::ProjectBorrow
            | raw::Opcode::BindBorrow
            | raw::Opcode::BorrowReplace
            | raw::Opcode::BorrowRead
            | raw::Opcode::BorrowWrite
            | raw::Opcode::EndBorrow
    ) || (matches!(
        operation.opcode,
        raw::Opcode::Clone | raw::Opcode::StringClone | raw::Opcode::VecClone
    ) && !operation.borrows.is_empty())
}

fn has_canonical_immediate(operation: &raw::Operation) -> bool {
    matches!(
        (&operation.opcode, &operation.immediate),
        (raw::Opcode::BoolLiteral, raw::Immediate::Bool(_))
            | (raw::Opcode::I32Literal, raw::Immediate::I32(_))
            | (raw::Opcode::String, raw::Immediate::Utf8(_))
            | (raw::Opcode::Construct, raw::Immediate::None | raw::Immediate::Variant(_))
            | (
                raw::Opcode::I32Add
                    | raw::Opcode::I32Sub
                    | raw::Opcode::I32Mul
                    | raw::Opcode::I32Neg
                    | raw::Opcode::Eq
                    | raw::Opcode::Ne
                    | raw::Opcode::I32LtS
                    | raw::Opcode::I32LeS
                    | raw::Opcode::I32GtS
                    | raw::Opcode::I32GeS
                    | raw::Opcode::Call
                    | raw::Opcode::Copy
                    | raw::Opcode::Move
                    | raw::Opcode::Clone
                    | raw::Opcode::StringClone
                    | raw::Opcode::VecClone
                    | raw::Opcode::Initialize
                    | raw::Opcode::Replace
                    | raw::Opcode::Drop
                    | raw::Opcode::Discriminant
                    | raw::Opcode::Index
                    | raw::Opcode::StringConcat
                    | raw::Opcode::VecConstruct
                    | raw::Opcode::VecPush
                    | raw::Opcode::SharedConstruct
                    | raw::Opcode::SharedClone
                    | raw::Opcode::WeakDowngrade
                    | raw::Opcode::WeakClone
                    | raw::Opcode::BeginBorrow
                    | raw::Opcode::BeginIndexedBorrow
                    | raw::Opcode::ProjectBorrow
                    | raw::Opcode::BindBorrow
                    | raw::Opcode::BorrowReplace
                    | raw::Opcode::BorrowRead
                    | raw::Opcode::BorrowWrite
                    | raw::Opcode::EndBorrow,
                raw::Immediate::None
            )
    )
}
