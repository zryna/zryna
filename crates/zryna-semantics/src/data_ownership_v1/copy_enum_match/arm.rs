use super::super::copy_lowering::FunctionLowerer;
use super::super::{RawExpressionKind, Ty, raw};
use super::span;

pub(super) fn value(
    lowerer: &mut FunctionLowerer<'_, '_, '_>,
    id: u32,
) -> Option<(Ty, raw::ValueId)> {
    let expression = lowerer.function.body.expressions.get(usize::try_from(id).ok()?)?;
    let at = span(lowerer.input.sources(), expression.span);
    match &expression.kind {
        RawExpressionKind::I32Literal { spelling } if spelling.parse::<i32>().is_err() => {
            lowerer.errors.at(
                "ZRYNA-M3008",
                at,
                "match-arm integer literal is outside i32",
                "use an i32 literal",
            );
            return None;
        }
        RawExpressionKind::Reference { name } if !lowerer.bindings.contains_key(&name.text) => {
            lowerer.errors.at(
                "ZRYNA-M3002",
                span(lowerer.input.sources(), name.span),
                format!("match-arm name '{}' is not declared", name.text),
                "reference the payload binding or a parameter",
            );
            return None;
        }
        RawExpressionKind::Match { .. } => {
            lowerer.errors.at(
                "ZRYNA-M3009",
                at,
                "nested aggregate operations in match arms are outside the M3 match oracle",
                "return a scalar literal, parameter, or payload binding from each arm",
            );
            return None;
        }
        _ => {}
    }
    lowerer.value(id)
}
