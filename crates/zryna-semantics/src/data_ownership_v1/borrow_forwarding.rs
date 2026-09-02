use std::collections::{BTreeMap, BTreeSet};

use zryna_ir::data_ownership_v1::raw;
use zryna_source::SourceMap;
use zryna_syntax::v4::{RawExpressionKind, RawFunctionSyntax};

use super::diagnostics::Errors;
use super::function_catalog::{FunctionParameterOrder, FunctionSignature};
use super::{BorrowBinding, span};

/// Seals the non-evaluating borrow suffix of one source-ordered direct call.
///
/// This deliberately does not estimate values, places, cleanup plans, or call depth. Those
/// resource boundaries remain owned by the existing preflight and mandatory IR verifier.
pub(super) fn plan_forwarded_borrow_arguments(
    sources: &SourceMap,
    function: &RawFunctionSyntax,
    signature: &FunctionSignature,
    arguments: &[u32],
    bindings: &BTreeMap<String, BorrowBinding>,
    errors: &mut Errors<'_>,
) -> Option<Vec<Option<raw::BorrowId>>> {
    let mut borrows = vec![None; signature.borrow_parameters.len()];
    let mut exclusive = BTreeSet::new();
    // Traverse the declared source order even though borrow validation precedes value emission.
    // This keeps the first invalid borrow deterministic for mixed signatures.
    for (argument, order) in arguments.iter().zip(&signature.parameter_order) {
        let FunctionParameterOrder::Borrow(index) = *order else {
            continue;
        };
        let expression = usize::try_from(*argument)
            .ok()
            .and_then(|index| function.body.expressions.get(index))?;
        let argument_span = span(sources, expression.span);
        let RawExpressionKind::Reference { name } = &expression.kind else {
            errors.at(
                "ZRYNA-M3016",
                argument_span,
                "borrow arguments must forward an in-scope borrow parameter",
                "pass an exact Borrow or BorrowMut parameter; lexical call borrows are not enabled",
            );
            return None;
        };
        let Some(actual) = bindings.get(&name.text).copied() else {
            errors.at(
                "ZRYNA-M3016",
                argument_span,
                "borrow arguments must forward an in-scope borrow parameter",
                "pass an exact Borrow or BorrowMut parameter; lexical call borrows are not enabled",
            );
            return None;
        };
        let expected = *signature.borrow_parameters.get(usize::try_from(index).ok()?)?;
        if actual.ty != expected.referent || actual.access != expected.access {
            errors.at(
                "ZRYNA-M3016",
                argument_span,
                "borrow argument does not match the callee referent and access",
                "pass an exact borrow parameter with the declared referent and shared or exclusive access",
            );
            return None;
        }
        if expected.access == raw::BorrowAccess::Exclusive && !exclusive.insert(actual.borrow) {
            errors.at(
                "ZRYNA-M3016",
                argument_span,
                "exclusive borrow arguments cannot reuse the same authority",
                "pass distinct nonoverlapping exclusive borrow parameters",
            );
            return None;
        }
        *borrows.get_mut(usize::try_from(index).ok()?)? = Some(actual.borrow);
    }
    Some(borrows)
}
