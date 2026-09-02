use std::collections::{BTreeMap, BTreeSet};

use zryna_layout::{self as layout, raw as raw_layout};
use zryna_syntax::v4::{self as syntax, RawExpressionKind};

use super::diagnostics::Errors;
use super::function_catalog::{FunctionCatalog, FunctionParameterOrder, FunctionResolution};
use super::layout_graph::Decl;
use super::root_borrow_value_planning::plan_root_borrow_initializer;
use super::type_model::{RootBorrowAlias, RootBorrowCallArgumentPlan, RootBorrowCallPlan, Ty};
use super::{SemanticInput, span};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn plan_root_borrow_call<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &syntax::RawFunctionSyntax,
    file: &syntax::SourceUnit,
    declarations: &[Decl],
    graph: &raw_layout::Graph,
    node_types: &[Option<Ty>],
    layouts: &layout::VerifiedLayouts,
    catalog: &FunctionCatalog,
    aliases: &BTreeMap<String, RootBorrowAlias>,
    expression_id: u32,
    expected_result: Ty,
    allow_call: bool,
    errors: &mut Errors<'a>,
) -> Option<(RootBorrowCallPlan, Vec<zryna_ir::data_ownership_v1::raw::BorrowId>)> {
    let expression = usize::try_from(expression_id)
        .ok()
        .and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Call { callee, arguments, .. } = &expression.kind else {
        return None;
    };
    let at = span(input.sources(), expression.span);
    if !allow_call {
        errors.at(
            "ZRYNA-M3016",
            at,
            "lexical borrow calls cannot cross a control-flow edge",
            "keep the single borrow call in one straight-line lexical block",
        );
        return None;
    }
    let signature = match catalog.resolve(module, &callee.text) {
        FunctionResolution::Exact(signature) => signature,
        FunctionResolution::WrongCase => {
            errors.at(
                "ZRYNA-M3002",
                span(input.sources(), callee.span),
                format!("call name '{}' has the wrong portable ASCII case", callee.text),
                "use the callee's exact declared spelling",
            );
            return None;
        }
        FunctionResolution::Missing => {
            errors.at(
                "ZRYNA-M3002",
                span(input.sources(), callee.span),
                format!("function '{}' is not declared in this module", callee.text),
                "call one exact private same-module function",
            );
            return None;
        }
    };
    if !signature.private {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), callee.span),
            "lexical borrow calls require one private same-module callee",
            "keep borrow authority behind the private direct-call boundary",
        );
        return None;
    }
    if signature.borrow_parameters.is_empty()
        || !signature.result.is_copy()
        || signature.parameters.iter().any(|parameter| !parameter.is_copy())
    {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), callee.span),
            "lexical borrow calls require a Copy signature with borrow authority",
            "call a private Copy-result function with at least one Borrow or BorrowMut parameter",
        );
        return None;
    }
    if signature.result != expected_result {
        errors.at(
            "ZRYNA-M3016",
            at,
            "lexical borrow call result does not match the declared Copy local",
            "declare the call result with the callee's exact Copy result type",
        );
        return None;
    }
    if arguments.len() != signature.parameter_order.len() {
        errors.at(
            "ZRYNA-M3016",
            at,
            format!(
                "call to '{}' has {} arguments but its signature requires {}",
                signature.name,
                arguments.len(),
                signature.parameter_order.len()
            ),
            "pass one source argument for every exact declared parameter",
        );
        return None;
    }

    let mut planned = Vec::with_capacity(arguments.len());
    let mut used = Vec::with_capacity(signature.borrow_parameters.len());
    let mut seen = BTreeSet::new();
    for (argument_id, order) in arguments.iter().zip(&signature.parameter_order) {
        let argument = usize::try_from(*argument_id)
            .ok()
            .and_then(|index| function.body.expressions.get(index))?;
        let argument_at = span(input.sources(), argument.span);
        match *order {
            FunctionParameterOrder::Value(index) => {
                if let RawExpressionKind::Reference { name } = &argument.kind
                    && aliases.contains_key(&name.text)
                {
                    errors.at(
                        "ZRYNA-M3016",
                        argument_at,
                        "borrow authority cannot satisfy a by-value call parameter",
                        "pass a direct Copy value for this parameter",
                    );
                    return None;
                }
                let expected = *signature.parameters.get(usize::try_from(index).ok()?)?;
                let value = plan_root_borrow_initializer(
                    input,
                    module,
                    function,
                    file,
                    declarations,
                    graph,
                    node_types,
                    layouts,
                    *argument_id,
                    expected,
                    errors,
                )?;
                planned.push(RootBorrowCallArgumentPlan::Value { index, value });
            }
            FunctionParameterOrder::Borrow(index) => {
                let expected = *signature.borrow_parameters.get(usize::try_from(index).ok()?)?;
                let RawExpressionKind::Reference { name } = &argument.kind else {
                    errors.at(
                        "ZRYNA-M3016",
                        argument_at,
                        "borrow call arguments must name one active lexical alias",
                        "pass an exact in-scope Borrow or BorrowMut alias",
                    );
                    return None;
                };
                let Some(actual) = aliases.get(&name.text) else {
                    errors.at(
                        "ZRYNA-M3016",
                        argument_at,
                        format!("borrow alias '{}' is not active at this call", name.text),
                        "declare the lexical alias before the call and keep it in the same block",
                    );
                    return None;
                };
                if !actual.place.projections.is_empty() {
                    errors.at(
                        "ZRYNA-M3016",
                        argument_at,
                        "projected lexical borrows cannot be passed to calls",
                        "pass one whole-root lexical Borrow or BorrowMut alias",
                    );
                    return None;
                }
                if actual.ty != expected.referent || actual.access != expected.access {
                    errors.at(
                        "ZRYNA-M3016",
                        argument_at,
                        "lexical borrow argument does not match the callee referent and access",
                        "pass exact referent and shared or exclusive authority",
                    );
                    return None;
                }
                if !seen.insert(actual.id) {
                    errors.at(
                        "ZRYNA-M3016",
                        argument_at,
                        "one lexical borrow authority cannot be repeated in a call",
                        "pass each active lexical alias at most once",
                    );
                    return None;
                }
                used.push(actual.id);
                planned.push(RootBorrowCallArgumentPlan::Borrow { index, id: actual.id });
            }
        }
    }

    Some((
        RootBorrowCallPlan {
            callee: signature.id,
            arguments: planned,
            value_parameters: signature.parameters.len(),
            borrow_parameters: signature.borrow_parameters.len(),
            result: signature.result,
            at,
        },
        used,
    ))
}
