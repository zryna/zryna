//! Independent sealed member, field identity and Copy projection typing.
use super::{Admission, Failure, graph, raw, reject};
use zryna_layout::generic_v1::TypeView;

pub(super) fn check(
    operation: &raw::Operation,
    views: &[TypeView<'_>],
    operand: &impl Fn(u32) -> Result<graph::Type, Failure>,
    admission: Admission,
) -> Result<graph::Type, Failure> {
    match operation {
        raw::Operation::StringLiteral(bytes) => {
            std::str::from_utf8(bytes).map_err(|_| reject("invalid String UTF8"))?;
            Ok(graph::Type::Stored(2))
        }
        raw::Operation::Move(id) => {
            let ty = operand(*id)?;
            if !matches!(ty, graph::Type::Stored(_)) {
                return Err(reject("move cannot carry unit or loan"));
            }
            Ok(ty)
        }
        raw::Operation::Borrow { value, exclusive } => {
            let graph::Type::Stored(referent) = operand(*value)? else {
                return Err(reject("loan requires stored root"));
            };
            Ok(graph::Type::Borrow { referent, exclusive: *exclusive })
        }
        raw::Operation::EndLoan(id) => {
            if !matches!(operand(*id)?, graph::Type::Borrow { .. }) {
                return Err(reject("end of nonloan"));
            }
            Ok(graph::Type::Unit)
        }
        raw::Operation::Drop(id) => {
            let graph::Type::Stored(i) = operand(*id)? else {
                return Err(reject("drop requires complete owned value"));
            };
            if layout(views, i)?.drop_kind() == 0 {
                return Err(reject("drop claim for Copy value"));
            }
            Ok(graph::Type::Unit)
        }
        raw::Operation::CloneStruct { value } => {
            if admission != Admission::StructCloneV5 {
                return Err(reject("Struct Clone requires private V5 admission"));
            }
            let graph::Type::Stored(base) = operand(*value)? else {
                return Err(reject("Struct Clone requires an exact stored source"));
            };
            structure(views, base, admission)?;
            Ok(graph::Type::Stored(base))
        }
        raw::Operation::CloneString(id) => string_clone(operand(*id)?, false),
        raw::Operation::CloneBorrowedString(id) => string_clone(operand(*id)?, true),
        raw::Operation::StructConstruct { ty, fields } => {
            let view = structure(views, *ty, admission)?;
            if fields.len() != view.fields().len() {
                return Err(reject(
                    "Struct constructor member count differs from sealed declaration",
                ));
            }
            for (value, (_, field, _)) in fields.iter().zip(view.fields()) {
                if operand(*value)? != graph::Type::Stored(field.index()) {
                    return Err(reject(
                        "Struct initializer differs from exact sealed field identity",
                    ));
                }
            }
            Ok(graph::Type::Stored(*ty))
        }
        raw::Operation::CopyStructField { value, ordinal } => {
            let graph::Type::Stored(base) = operand(*value)? else {
                return Err(reject("Copy field requires stored Struct root"));
            };
            let view = structure(views, base, admission)?;
            let (_, field, _) = view
                .fields()
                .nth(*ordinal as usize)
                .ok_or_else(|| reject("Copy field ordinal is outside sealed Struct"))?;
            if layout(views, field.index())?.drop_kind() != 0 {
                return Err(reject("Copy field cannot duplicate an affine member"));
            }
            Ok(graph::Type::Stored(field.index()))
        }
    }
}
fn layout<'a>(views: &[TypeView<'a>], id: u32) -> Result<TypeView<'a>, Failure> {
    views.get(id as usize).copied().ok_or_else(|| reject("unknown owned layout type ID"))
}
fn structure<'a>(
    views: &[TypeView<'a>],
    id: u32,
    admission: Admission,
) -> Result<TypeView<'a>, Failure> {
    let view = layout(views, id)?;
    if !admission.has_structs()
        || !matches!(view.key().first(), Some(0x10 | 0x12))
        || view.fields().len() == 0
    {
        return Err(reject("Struct effect requires private v4 nonempty sealed Struct"));
    }
    Ok(view)
}
fn string_clone(operand: graph::Type, borrowed: bool) -> Result<graph::Type, Failure> {
    let (expected, message) = if borrowed {
        (
            graph::Type::Borrow { referent: 2, exclusive: false },
            "selected clone requires exact shared String loan",
        )
    } else {
        (graph::Type::Stored(2), "clone requires exact String")
    };
    if operand != expected {
        return Err(reject(message));
    }
    Ok(graph::Type::Stored(2))
}

#[cfg(test)]
#[path = "extension/clone_v5_tests.rs"]
mod clone_v5_tests;
