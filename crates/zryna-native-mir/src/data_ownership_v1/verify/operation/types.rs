use super::super::{Errors, raw, type_at};

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1::verify) struct BorrowInfo {
    pub(in crate::data_ownership_v1::verify) ty: u32,
    pub(in crate::data_ownership_v1::verify) access: raw::BorrowAccess,
}

pub(in crate::data_ownership_v1::verify) fn verify_types(
    program: &raw::Program,
    function: &raw::Function,
    operation: &raw::Operation,
    borrow_types: &[Option<BorrowInfo>],
    errors: &mut Errors,
) {
    let result_type = operation.result.as_ref().map(|value| value.ty);
    let value = |index: usize| operation.values.get(index).and_then(|id| value_type(function, *id));
    let place = |index: usize| {
        operation
            .places
            .get(index)
            .and_then(|id| function.places.get(*id as usize))
            .map(|place| place.ty)
    };
    let category = |ty| type_at(program, ty).map(|ty| ty.category);
    let result_category = result_type.and_then(category);
    let valid = scalar_types_are_exact(program, function, operation).unwrap_or_else(|| {
        borrow_types_are_exact(program, function, operation, borrow_types).unwrap_or_else(|| {
            match operation.opcode {
        raw::Opcode::Call => call_types_are_exact(program, function, operation, borrow_types),
        raw::Opcode::Construct => construct_types_are_exact(program, function, operation),
        raw::Opcode::Copy | raw::Opcode::Move => place(0) == result_type,
        raw::Opcode::Clone => source_type(function, operation, borrow_types) == result_type,
        raw::Opcode::StringClone => {
            source_type(function, operation, borrow_types) == result_type
                && result_category == Some(raw::TypeCategory::String)
        }
        raw::Opcode::VecClone => {
            source_type(function, operation, borrow_types) == result_type
                && result_category == Some(raw::TypeCategory::Vec)
        }
        raw::Opcode::Initialize | raw::Opcode::Replace => place(0) == value(0),
        raw::Opcode::Drop => place(0).is_some(),
        raw::Opcode::Discriminant => {
            place(0).and_then(category) == Some(raw::TypeCategory::Enum)
                && result_category == Some(raw::TypeCategory::I32)
        }
        raw::Opcode::Index => indexed_types_are_exact(program, place(0), value(0), result_type),
        raw::Opcode::String => {
            result_category == Some(raw::TypeCategory::String)
                && matches!(&operation.immediate, raw::Immediate::Utf8(bytes) if std::str::from_utf8(bytes).is_ok())
        }
        raw::Opcode::StringConcat => {
            place(0) == result_type
                && place(1) == result_type
                && result_category == Some(raw::TypeCategory::String)
        }
        raw::Opcode::VecConstruct => container_values_are_exact(
            program,
            function,
            &operation.values,
            result_type,
            raw::TypeCategory::Vec,
        ),
        raw::Opcode::VecPush => place(0).and_then(|ty| type_at(program, ty)).is_some_and(|ty| {
            ty.category == raw::TypeCategory::Vec && ty.referenced_type == value(0)
        }),
        raw::Opcode::SharedConstruct => {
            result_type.and_then(|ty| type_at(program, ty)).is_some_and(|ty| {
                ty.category == raw::TypeCategory::Shared && ty.referenced_type == value(0)
            })
        }
        raw::Opcode::SharedClone => {
            place(0) == result_type && result_category == Some(raw::TypeCategory::Shared)
        }
        raw::Opcode::WeakDowngrade => place(0)
            .and_then(|source| type_at(program, source))
            .zip(result_type.and_then(|result| type_at(program, result)))
            .is_some_and(|(shared, weak)| {
                shared.category == raw::TypeCategory::Shared
                    && weak.category == raw::TypeCategory::Weak
                    && shared.referenced_type == weak.referenced_type
            }),
        raw::Opcode::WeakClone => {
            place(0) == result_type && result_category == Some(raw::TypeCategory::Weak)
        }
                _ => false,
            }
        })
    });
    if !valid {
        errors.push("ZRYNA-N3107", "native MIR operation type relation is not exact");
    }
}

fn scalar_types_are_exact(
    program: &raw::Program,
    function: &raw::Function,
    operation: &raw::Operation,
) -> Option<bool> {
    let result = operation.result.map(|value| value.ty);
    let value = |index: usize| operation.values.get(index).and_then(|id| value_type(function, *id));
    let category = |ty| type_at(program, ty).map(|ty| ty.category);
    let result_category = result.and_then(category);
    match operation.opcode {
        raw::Opcode::BoolLiteral => Some(result_category == Some(raw::TypeCategory::Bool)),
        raw::Opcode::I32Literal => Some(result_category == Some(raw::TypeCategory::I32)),
        raw::Opcode::I32Add | raw::Opcode::I32Sub | raw::Opcode::I32Mul => Some(
            value(0).and_then(category) == Some(raw::TypeCategory::I32)
                && value(1).and_then(category) == Some(raw::TypeCategory::I32)
                && result_category == Some(raw::TypeCategory::I32),
        ),
        raw::Opcode::I32Neg => Some(
            value(0).and_then(category) == Some(raw::TypeCategory::I32)
                && result_category == Some(raw::TypeCategory::I32),
        ),
        raw::Opcode::Eq | raw::Opcode::Ne => Some(
            value(0) == value(1)
                && value(0).is_some_and(|ty| {
                    matches!(category(ty), Some(raw::TypeCategory::Bool | raw::TypeCategory::I32))
                })
                && result_category == Some(raw::TypeCategory::Bool),
        ),
        raw::Opcode::I32LtS | raw::Opcode::I32LeS | raw::Opcode::I32GtS | raw::Opcode::I32GeS => {
            Some(
                value(0).and_then(category) == Some(raw::TypeCategory::I32)
                    && value(1).and_then(category) == Some(raw::TypeCategory::I32)
                    && result_category == Some(raw::TypeCategory::Bool),
            )
        }
        _ => None,
    }
}

fn borrow_types_are_exact(
    program: &raw::Program,
    function: &raw::Function,
    operation: &raw::Operation,
    borrows: &[Option<BorrowInfo>],
) -> Option<bool> {
    let result = operation.result.map(|value| value.ty);
    let value = |index: usize| operation.values.get(index).and_then(|id| value_type(function, *id));
    let place = |index: usize| {
        operation
            .places
            .get(index)
            .and_then(|id| function.places.get(*id as usize))
            .map(|place| place.ty)
    };
    let borrow = |index: usize| {
        operation.borrows.get(index).and_then(|id| borrows.get(*id as usize)).copied().flatten()
    };
    let index_is_i32 = || {
        value(0).and_then(|ty| type_at(program, ty)).map(|ty| ty.category)
            == Some(raw::TypeCategory::I32)
    };
    match operation.opcode {
        raw::Opcode::BeginBorrow => Some(operation.borrow_type == place(0)),
        raw::Opcode::BeginIndexedBorrow => Some(
            index_is_i32()
                && place(0).and_then(|ty| type_at(program, ty)).and_then(|ty| ty.referenced_type)
                    == operation.borrow_type,
        ),
        raw::Opcode::ProjectBorrow => Some(
            index_is_i32()
                && borrow(0)
                    .and_then(|info| type_at(program, info.ty))
                    .and_then(|ty| ty.referenced_type)
                    == operation.borrow_type,
        ),
        raw::Opcode::BindBorrow => Some(borrow(0).map(|info| info.ty) == operation.borrow_type),
        raw::Opcode::BorrowRead => Some(borrow(0).is_some_and(|info| {
            Some(info.ty) == result && type_at(program, info.ty).is_some_and(|ty| ty.drop_kind == 0)
        })),
        raw::Opcode::BorrowWrite | raw::Opcode::BorrowReplace => {
            Some(borrow(0).is_some_and(|info| {
                info.access == raw::BorrowAccess::Exclusive
                    && Some(info.ty) == value(0)
                    && operation.borrow_type == Some(info.ty)
                    && type_at(program, info.ty).is_some_and(|ty| {
                        (operation.opcode == raw::Opcode::BorrowWrite) == (ty.drop_kind == 0)
                    })
            }))
        }
        raw::Opcode::EndBorrow => Some(borrow(0).map(|info| info.ty) == operation.borrow_type),
        _ => None,
    }
}

fn source_type(
    function: &raw::Function,
    operation: &raw::Operation,
    borrows: &[Option<BorrowInfo>],
) -> Option<u32> {
    operation
        .places
        .first()
        .and_then(|id| function.places.get(*id as usize))
        .map(|place| place.ty)
        .or_else(|| {
            operation
                .borrows
                .first()
                .and_then(|id| borrows.get(*id as usize))
                .copied()
                .flatten()
                .map(|info| info.ty)
        })
}

fn call_types_are_exact(
    program: &raw::Program,
    function: &raw::Function,
    operation: &raw::Operation,
    borrows: &[Option<BorrowInfo>],
) -> bool {
    let Some(callee) = operation.callee.and_then(|identity| {
        program
            .functions
            .iter()
            .find(|function| (function.module, function.declaration) == identity)
    }) else {
        return false;
    };
    if operation.call_arguments.len() != callee.parameters.len() + callee.borrow_parameters.len()
        || operation.result.map(|result| result.ty) != Some(callee.result_type)
    {
        return false;
    }
    operation
        .call_arguments
        .iter()
        .take(callee.parameters.len())
        .zip(&callee.parameters)
        .all(|(argument, parameter)| {
            matches!(argument, raw::CallArgument::Value(id) if value_type(function, *id) == Some(parameter.ty))
        })
        && operation
            .call_arguments
            .iter()
            .skip(callee.parameters.len())
            .zip(&callee.borrow_parameters)
            .all(|(argument, parameter)| {
                matches!(argument, raw::CallArgument::Borrow(id)
                    if borrows.get(*id as usize).copied().flatten().is_some_and(|info|
                        info.ty == parameter.referent && info.access == parameter.access))
            })
}

fn construct_types_are_exact(
    program: &raw::Program,
    function: &raw::Function,
    operation: &raw::Operation,
) -> bool {
    let Some(result) = operation.result.and_then(|value| type_at(program, value.ty)) else {
        return false;
    };
    match result.category {
        raw::TypeCategory::Struct => {
            matches!(operation.immediate, raw::Immediate::None)
                && operation.values.len() == result.fields.len()
                && operation
                    .values
                    .iter()
                    .zip(&result.fields)
                    .all(|(value, field)| value_type(function, *value) == Some(field.ty))
        }
        raw::TypeCategory::Enum => {
            let raw::Immediate::Variant(ordinal) = operation.immediate else { return false };
            result.variants.iter().find(|variant| variant.ordinal == ordinal).is_some_and(
                |variant| match (operation.values.as_slice(), variant.payload) {
                    ([], None) => true,
                    ([value], Some(ty)) => value_type(function, *value) == Some(ty),
                    _ => false,
                },
            )
        }
        raw::TypeCategory::FixedArray => {
            matches!(operation.immediate, raw::Immediate::None)
                && result.array_length == u64::try_from(operation.values.len()).ok()
                && result.referenced_type.is_some_and(|element| {
                    operation
                        .values
                        .iter()
                        .all(|value| value_type(function, *value) == Some(element))
                })
        }
        _ => false,
    }
}

fn indexed_types_are_exact(
    program: &raw::Program,
    place: Option<u32>,
    index: Option<u32>,
    result: Option<u32>,
) -> bool {
    type_at(program, index.unwrap_or(u32::MAX))
        .is_some_and(|ty| ty.category == raw::TypeCategory::I32)
        && place.and_then(|ty| type_at(program, ty)).is_some_and(|container| {
            matches!(container.category, raw::TypeCategory::FixedArray | raw::TypeCategory::Vec)
                && container.referenced_type == result
                && result
                    .and_then(|ty| type_at(program, ty))
                    .is_some_and(|element| element.drop_kind == 0)
        })
}

fn container_values_are_exact(
    program: &raw::Program,
    function: &raw::Function,
    values: &[u32],
    result: Option<u32>,
    category: raw::TypeCategory,
) -> bool {
    result.and_then(|ty| type_at(program, ty)).is_some_and(|container| {
        container.category == category
            && container.referenced_type.is_some_and(|element| {
                values.iter().all(|value| value_type(function, *value) == Some(element))
            })
    })
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
