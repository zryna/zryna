use zryna_layout::{TypeCategory, raw as raw_layout};
use zryna_syntax::v4::{self as syntax, RawExpressionKind, RawStatementKind, RawTypeSyntaxKind};

use super::SemanticInput;
use super::diagnostics::Errors;
use super::layout_graph::{Decl, semantic_type};
use super::type_model::{OwnedRootBorrowSyntax, Ty};
use crate::data_ownership_v1::diagnostics::span;

fn direct_reference_name(function: &syntax::RawFunctionSyntax, expression: u32) -> Option<&str> {
    let expression =
        usize::try_from(expression).ok().and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Reference { name } = &expression.kind else { return None };
    Some(&name.text)
}

pub(super) fn is_direct_owned_root_borrow_candidate(
    file: &syntax::SourceUnit,
    function: &syntax::RawFunctionSyntax,
) -> bool {
    let Some(root) = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
    else {
        return false;
    };
    let [root_local, nested, _] = root.statements.as_slice() else { return false };
    let Some(root_local) =
        usize::try_from(*root_local).ok().and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    let RawStatementKind::LocalDeclaration { name: root_name, .. } = &root_local.kind else {
        return false;
    };
    let Some(nested) =
        usize::try_from(*nested).ok().and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    let RawStatementKind::Block { block } = nested.kind else { return false };
    let Some(alias) = usize::try_from(block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
        .and_then(|block| block.statements.first())
        .and_then(|statement| usize::try_from(*statement).ok())
        .and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    let RawStatementKind::LocalDeclaration { type_syntax, initializer, .. } = alias.kind else {
        return false;
    };
    let shared = usize::try_from(type_syntax)
        .ok()
        .and_then(|index| file.type_syntax().get(index))
        .is_some_and(|ty| matches!(ty.kind, RawTypeSyntaxKind::Borrow { .. }));
    let Some(initializer) =
        usize::try_from(initializer).ok().and_then(|index| function.body.expressions.get(index))
    else {
        return false;
    };
    let RawExpressionKind::Borrow { value, .. } = initializer.kind else { return false };
    shared && direct_reference_name(function, value) == Some(root_name.text.as_str())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn plan_private_owned_root_borrow_syntax<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &'a syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<OwnedRootBorrowSyntax> {
    let at = span(input.sources(), function.span);
    if function.export_span.is_some() || !function.parameters.is_empty() {
        errors.at(
            "ZRYNA-M3017",
            at,
            "owned-root borrow reads require one private parameter-free function",
            "keep this internal checkpoint private and initialize its owner locally",
        );
        return None;
    }
    if result.is_copy()
        || !matches!(
            result.category,
            TypeCategory::String
                | TypeCategory::Vec
                | TypeCategory::Struct
                | TypeCategory::Enum
                | TypeCategory::FixedArray
        )
    {
        errors.at(
            "ZRYNA-M3017",
            at,
            "owned-root borrow reads require one supported non-Copy result",
            "return the exact String, Vec, Struct, Enum, or fixed-array root after lexical end",
        );
        return None;
    }
    let file = input.syntax().files().get(module)?;
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let [root_local_id, nested_id, return_id] = root.statements.as_slice() else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root.span),
            "owned-root borrow reads require one root local, one lexical block, and one final return",
            "declare the owner, read it through one nested shared-borrow block, then return it",
        );
        return None;
    };
    let root_local = usize::try_from(*root_local_id)
        .ok()
        .and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::LocalDeclaration { name: root_name, type_syntax: root_type, .. } =
        &root_local.kind
    else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root_local.span),
            "owned-root borrow reads require one initialized local owner",
            "declare one exact owned root before the lexical block",
        );
        return None;
    };
    let root_ty = semantic_type(file, *root_type, module, declarations, graph, node_types, errors)?;
    if root_ty != result || root_ty.is_copy() {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), root_local.span),
            "owned-root borrow result must exactly match the initialized non-Copy root",
            "give the local root and function result one exact owned type",
        );
        return None;
    }
    let nested_statement =
        usize::try_from(*nested_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Block { block: nested_block_id } = nested_statement.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), nested_statement.span),
            "owned-root shared authority requires one explicit lexical block",
            "place the alias and its read-only operations inside one nested block",
        );
        return None;
    };
    let nested =
        usize::try_from(nested_block_id).ok().and_then(|index| function.body.blocks.get(index))?;
    let Some((alias_statement_id, read_statement_ids)) = nested.statements.split_first() else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), nested.span),
            "owned-root borrow block requires one shared alias and at least one read",
            "declare Borrow<Root> first, then perform one admitted read-only operation",
        );
        return None;
    };
    if read_statement_ids.is_empty() {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), nested.span),
            "owned-root borrow block requires at least one read-only operation",
            "clone, concatenate, or index through the shared alias before lexical end",
        );
        return None;
    }
    let alias_statement = usize::try_from(*alias_statement_id)
        .ok()
        .and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::LocalDeclaration {
        mutable: false,
        name: alias_name,
        type_syntax: alias_type,
        initializer: alias_initializer,
        ..
    } = &alias_statement.kind
    else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), alias_statement.span),
            "owned-root borrow block must begin with one const shared alias",
            "declare `const alias: Borrow<Root> = borrow(root)`",
        );
        return None;
    };
    let declared =
        usize::try_from(*alias_type).ok().and_then(|index| file.type_syntax().get(index))?;
    let RawTypeSyntaxKind::Borrow { argument, .. } = declared.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), declared.span),
            "owned-root reads require shared Borrow authority",
            "use Borrow rather than BorrowMut for read-only owned access",
        );
        return None;
    };
    let referent = semantic_type(file, argument, module, declarations, graph, node_types, errors)?;
    if referent != root_ty {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), declared.span),
            "owned-root borrow alias has the wrong exact referent type",
            "declare Borrow with the exact whole-root owned type",
        );
        return None;
    }
    let alias_initializer_expression = usize::try_from(*alias_initializer)
        .ok()
        .and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Borrow { value: borrowed, .. } = alias_initializer_expression.kind
    else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), alias_initializer_expression.span),
            "owned-root shared alias must be initialized with borrow(root)",
            "borrow the exact root directly without a projection or reborrow",
        );
        return None;
    };
    if direct_reference_name(function, borrowed) != Some(root_name.text.as_str()) {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), alias_initializer_expression.span),
            "owned-root shared alias must borrow the exact whole root",
            "remove projections, subobjects, and alternate operands from borrow(root)",
        );
        return None;
    }

    let mut flattened = Vec::with_capacity(read_statement_ids.len().saturating_add(2));
    flattened.push(*root_local_id);
    for statement_id in read_statement_ids {
        let statement = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        let RawStatementKind::LocalDeclaration { mutable: false, type_syntax, initializer, .. } =
            statement.kind
        else {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), statement.span),
                "owned-root borrow blocks admit only const read results",
                "remove moves, writes, replacement, drops, calls, and nested control flow",
            );
            return None;
        };
        let local_ty =
            semantic_type(file, type_syntax, module, declarations, graph, node_types, errors)?;
        let expression = usize::try_from(initializer)
            .ok()
            .and_then(|index| function.body.expressions.get(index))?;
        let admitted = match (&expression.kind, root_ty.category) {
            (RawExpressionKind::Clone { value, .. }, TypeCategory::String)
                if local_ty == root_ty
                    && direct_reference_name(function, *value)
                        == Some(alias_name.text.as_str()) =>
            {
                true
            }
            (RawExpressionKind::Call { callee, arguments, .. }, TypeCategory::String)
                if local_ty == root_ty
                    && callee.text == "concat"
                    && arguments.len() == 2
                    && arguments.iter().all(|argument| {
                        direct_reference_name(function, *argument) == Some(alias_name.text.as_str())
                    }) =>
            {
                true
            }
            (RawExpressionKind::Index { base, .. }, TypeCategory::Vec)
                if local_ty.is_copy()
                    && matches!(local_ty.category, TypeCategory::Bool | TypeCategory::I32)
                    && direct_reference_name(function, *base) == Some(alias_name.text.as_str()) =>
            {
                true
            }
            (RawExpressionKind::Clone { value, .. }, category)
                if matches!(
                    category,
                    TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
                ) && local_ty == root_ty
                    && direct_reference_name(function, *value)
                        == Some(alias_name.text.as_str()) =>
            {
                true
            }
            _ => false,
        };
        if !admitted {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), expression.span),
                "operation is outside whole owned-root shared reads",
                "use String clone/concat, exact Vec<bool/i32> indexing, or whole-root aggregate clone through the alias",
            );
            return None;
        }
        flattened.push(*statement_id);
    }

    let return_statement =
        usize::try_from(*return_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Return { value: returned, .. } = return_statement.kind else {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), return_statement.span),
            "owned-root borrow reads require one final root return",
            "return the original owner after lexical end",
        );
        return None;
    };
    if direct_reference_name(function, returned) != Some(root_name.text.as_str()) {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), return_statement.span),
            "owned-root borrow aliases and read results cannot escape",
            "return the original root only after the shared alias ends",
        );
        return None;
    }
    flattened.push(*return_id);

    let mut synthetic = function.clone();
    let synthetic_root = usize::try_from(synthetic.body.root_block)
        .ok()
        .and_then(|index| synthetic.body.blocks.get_mut(index))?;
    synthetic_root.statements = flattened;
    let alias_statement_index = usize::try_from(*alias_statement_id).ok()?;
    synthetic.body.statements[alias_statement_index] = root_local.clone();
    for expression in &mut synthetic.body.expressions {
        if let RawExpressionKind::Reference { name } = &mut expression.kind
            && name.text == alias_name.text
        {
            name.text.clone_from(&root_name.text);
        }
    }
    Some(OwnedRootBorrowSyntax {
        synthetic,
        borrow_at: span(input.sources(), alias_statement.span),
        end_at: span(input.sources(), nested.close_brace_span),
    })
}
