use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind, RawStatementKind, RawTypeSyntaxKind};

use super::SemanticInput;
use super::diagnostics::Errors;
use super::function_catalog::{FunctionCatalog, FunctionResolution};
use super::layout_graph::{Decl, semantic_type};
use super::root_borrow_call_planning::plan_root_borrow_call;
use super::root_borrow_value_planning::{plan_root_borrow_initializer, plan_root_borrow_place};
use super::type_model::{
    RootBorrowAlias, RootBorrowArmPlan, RootBorrowPlacePlan, RootBorrowStep, Ty,
};
use crate::data_ownership_v1::diagnostics::span;

pub(super) fn root_borrow_paths_overlap(
    left: &RootBorrowPlacePlan,
    right: &RootBorrowPlacePlan,
) -> bool {
    let left = left.key();
    let right = right.key();
    let common = left.len().min(right.len());
    left[..common] == right[..common]
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn plan_root_borrow_arm<'a>(
    input: SemanticInput<'a>,
    module: usize,
    function: &syntax::RawFunctionSyntax,
    file: &syntax::SourceUnit,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    root_mutable: bool,
    root_name: &syntax::RawIdentifierSyntax,
    root_ty: Ty,
    nested: &syntax::RawBlockSyntax,
    allow_call: bool,
    errors: &mut Errors<'a>,
) -> Option<RootBorrowArmPlan> {
    let mut aliases = BTreeMap::<String, RootBorrowAlias>::new();
    let mut names = BTreeMap::<String, Span>::new();
    names.insert(root_name.text.to_ascii_lowercase(), span(input.sources(), root_name.span));
    let mut steps = Vec::with_capacity(nested.statements.len());
    let mut reads = 0_usize;
    let mut writes = 0_usize;
    let mut calls = 0_usize;
    let mut call_values = 0_usize;
    for statement_id in &nested.statements {
        let statement = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        let (mutable, name, type_syntax, initializer) = match &statement.kind {
            RawStatementKind::LocalDeclaration {
                mutable, name, type_syntax, initializer, ..
            } => (mutable, name, type_syntax, initializer),
            RawStatementKind::Assignment { target, value, .. } => {
                let target = usize::try_from(*target)
                    .ok()
                    .and_then(|index| function.body.expressions.get(index))?;
                let RawExpressionKind::Reference { name: target_name } = &target.kind else {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), target.span),
                        "exclusive-borrow writes require one direct alias target",
                        "assign a bool or i32 literal directly through the BorrowMut alias",
                    );
                    return None;
                };
                let Some(alias) = aliases.get_mut(&target_name.text) else {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), target_name.span),
                        "borrow blocks cannot replace the root or an ordinary local",
                        "write only through one active BorrowMut alias",
                    );
                    return None;
                };
                if alias.access != raw::BorrowAccess::Exclusive {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), target_name.span),
                        "shared aliases do not grant write authority",
                        "use a BorrowMut alias for an exact Copy write",
                    );
                    return None;
                }
                if !alias.ty.is_copy() {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), statement.span),
                        "exclusive projected writes require an exact Copy referent",
                        "write only through a bool, i32, Copy struct, or Copy fixed-array alias",
                    );
                    return None;
                }
                let assigned = usize::try_from(*value)
                    .ok()
                    .and_then(|index| function.body.expressions.get(index))?;
                if alias.place.projections.is_empty()
                    && matches!(alias.ty.category, TypeCategory::Bool | TypeCategory::I32)
                    && !matches!(
                        (&assigned.kind, alias.ty.category),
                        (RawExpressionKind::BoolLiteral { .. }, TypeCategory::Bool)
                            | (RawExpressionKind::I32Literal { .. }, TypeCategory::I32)
                    )
                {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), assigned.span),
                        "exclusive-borrow writes require an exact referent-typed literal",
                        "assign one literal with the exact bool or i32 referent type",
                    );
                    return None;
                }
                let value = plan_root_borrow_initializer(
                    input,
                    module,
                    function,
                    file,
                    declarations,
                    graph,
                    node_types,
                    layouts,
                    *value,
                    alias.ty,
                    errors,
                )?;
                alias.used = true;
                steps.push(RootBorrowStep::Write {
                    id: alias.id,
                    value,
                    at: span(input.sources(), statement.span),
                });
                writes = writes.checked_add(1)?;
                continue;
            }
            _ => {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), statement.span),
                    "root-borrow blocks admit only const aliases, const Copy reads, and BorrowMut writes",
                    "remove calls, control flow, effects, nested blocks, and ordinary assignment",
                );
                return None;
            }
        };
        if *mutable {
            errors.at(
                "ZRYNA-M3017",
                span(input.sources(), statement.span),
                "borrow block bindings must be const",
                "declare the alias or Copy read with const",
            );
            return None;
        }
        let portable_name = name.text.to_ascii_lowercase();
        if names.insert(portable_name, span(input.sources(), name.span)).is_some() {
            errors.at(
                "ZRYNA-M3002",
                span(input.sources(), name.span),
                format!("binding '{}' collides under portable ASCII case folding", name.text),
                "give every binding one portable case-insensitive unique name",
            );
            return None;
        }
        let declared =
            usize::try_from(*type_syntax).ok().and_then(|index| file.type_syntax().get(index))?;
        if matches!(
            declared.kind,
            RawTypeSyntaxKind::Borrow { .. } | RawTypeSyntaxKind::BorrowMut { .. }
        ) {
            let (argument, access) = match declared.kind {
                RawTypeSyntaxKind::Borrow { argument, .. } => (argument, raw::BorrowAccess::Shared),
                RawTypeSyntaxKind::BorrowMut { argument, .. } => {
                    (argument, raw::BorrowAccess::Exclusive)
                }
                _ => unreachable!(),
            };
            let referent =
                semantic_type(file, argument, module, declarations, graph, node_types, errors)?;
            if matches!(root_ty.category, TypeCategory::Bool | TypeCategory::I32)
                && referent != root_ty
            {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), declared.span),
                    "borrow alias referent does not match the root's exact scalar type",
                    "declare Borrow or BorrowMut matching the bool or i32 root",
                );
                return None;
            }
            let initializer = usize::try_from(*initializer)
                .ok()
                .and_then(|index| function.body.expressions.get(index))?;
            let (value, initializer_access) = match initializer.kind {
                RawExpressionKind::Borrow { value, .. } => (value, raw::BorrowAccess::Shared),
                RawExpressionKind::BorrowMut { value, .. } => (value, raw::BorrowAccess::Exclusive),
                _ => {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), initializer.span),
                        "borrow alias type and initializer must use the same access mode",
                        "initialize Borrow with borrow and BorrowMut with borrowMut",
                    );
                    return None;
                }
            };
            if initializer_access != access {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), initializer.span),
                    "borrow alias type and initializer access modes do not match",
                    "initialize Borrow with borrow and BorrowMut with borrowMut",
                );
                return None;
            }
            let borrowed = usize::try_from(value)
                .ok()
                .and_then(|index| function.body.expressions.get(index))?;
            let place = if let RawExpressionKind::Reference { name: borrowed_name } = &borrowed.kind
                && let Some(parent) = aliases.get(&borrowed_name.text).cloned()
            {
                if access != raw::BorrowAccess::Shared || parent.access != raw::BorrowAccess::Shared
                {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), borrowed_name.span),
                        "only shared-from-shared reborrowing is admitted",
                        "reborrow an active Borrow alias with borrow, or borrow a static place directly",
                    );
                    return None;
                }
                aliases.get_mut(&borrowed_name.text).expect("resolved shared parent").used = true;
                parent.place
            } else {
                plan_root_borrow_place(
                    input,
                    module,
                    function,
                    file,
                    declarations,
                    graph,
                    node_types,
                    layouts,
                    value,
                    &root_name.text,
                    root_ty,
                    errors,
                )?
            };
            if place.ty != referent || !referent.is_copy() {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), declared.span),
                    "borrow alias referent does not match the projected place's exact Copy type",
                    "declare Borrow or BorrowMut with the exact static projection type",
                );
                return None;
            }
            if access == raw::BorrowAccess::Exclusive && !root_mutable {
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), borrowed.span),
                    "exclusive borrowing requires a mutable root local",
                    "declare the Copy root with let before borrowMut",
                );
                return None;
            }
            let conflicting = aliases.values().find(|alias| {
                root_borrow_paths_overlap(&place, &alias.place)
                    && (access == raw::BorrowAccess::Exclusive
                        || alias.access == raw::BorrowAccess::Exclusive)
            });
            if let Some(conflicting) = conflicting {
                let root_only =
                    place.projections.is_empty() && conflicting.place.projections.is_empty();
                errors.at(
                    "ZRYNA-M3017",
                    span(input.sources(), statement.span),
                    if root_only {
                        "borrow access conflicts with an active alias of the same root"
                    } else {
                        "borrow access conflicts with an active alias of an overlapping place"
                    },
                    if root_only {
                        "end the active alias before requesting incompatible authority"
                    } else {
                        "keep exclusive authority on disjoint static siblings or use shared access"
                    },
                );
                return None;
            }
            let id = raw::BorrowId(u32::try_from(aliases.len()).ok()?);
            aliases.insert(
                name.text.clone(),
                RootBorrowAlias { id, ty: referent, place: place.clone(), access, used: false },
            );
            steps.push(RootBorrowStep::Begin {
                id,
                place,
                access,
                at: span(input.sources(), statement.span),
            });
        } else {
            let read_ty =
                semantic_type(file, *type_syntax, module, declarations, graph, node_types, errors)?;
            let initializer_id = *initializer;
            let initializer = usize::try_from(initializer_id)
                .ok()
                .and_then(|index| function.body.expressions.get(index))?;
            let lexical_call = if let RawExpressionKind::Call { callee, .. } = &initializer.kind {
                matches!(
                    catalog.resolve(module, &callee.text),
                    FunctionResolution::Exact(signature)
                        if signature.private && signature.has_borrow_parameters()
                )
            } else {
                false
            };
            if lexical_call {
                if calls != 0 {
                    errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), initializer.span),
                        "a lexical borrow block admits only one direct call",
                        "keep one bounded borrow call in the straight-line lexical block",
                    );
                    return None;
                }
                let (call, used) = plan_root_borrow_call(
                    input,
                    module,
                    function,
                    file,
                    declarations,
                    graph,
                    node_types,
                    layouts,
                    catalog,
                    &aliases,
                    initializer_id,
                    read_ty,
                    allow_call,
                    errors,
                )?;
                let Some(estimated_values) = call.checked_argument_value_count() else {
                    errors.at(
                        "ZRYNA-M3201",
                        span(input.sources(), initializer.span),
                        "lexical borrow-call preparation overflows its checked resource estimate",
                        "reduce nested Copy aggregate call arguments",
                    );
                    return None;
                };
                call_values = estimated_values;
                for alias in aliases.values_mut().filter(|alias| used.contains(&alias.id)) {
                    alias.used = true;
                }
                calls = 1;
                steps.push(RootBorrowStep::Call(call));
                continue;
            }
            if let RawExpressionKind::Reference { name: alias_name } = &initializer.kind
                && let Some(alias) = aliases.get_mut(&alias_name.text)
            {
                if alias.ty != read_ty {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), declared.span),
                        "borrow alias read type does not match its exact Copy referent",
                        "declare the Copy local with the alias referent type",
                    );
                    return None;
                }
                alias.used = true;
                steps.push(RootBorrowStep::Read {
                    id: alias.id,
                    ty: read_ty,
                    at: span(input.sources(), statement.span),
                });
            } else {
                if matches!(root_ty.category, TypeCategory::Bool | TypeCategory::I32)
                    && !matches!(initializer.kind, RawExpressionKind::Reference { .. })
                {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), initializer.span),
                        "shared-borrow Copy locals must read one active alias",
                        "initialize the Copy local from one preceding Borrow or BorrowMut alias",
                    );
                    return None;
                }
                if let RawExpressionKind::Reference { name } = &initializer.kind
                    && name.text != root_name.text
                {
                    errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!("borrow alias '{}' is not active in this block", name.text),
                        "read one exact preceding alias or one static root projection",
                    );
                    return None;
                }
                let place = plan_root_borrow_place(
                    input,
                    module,
                    function,
                    file,
                    declarations,
                    graph,
                    node_types,
                    layouts,
                    initializer_id,
                    &root_name.text,
                    root_ty,
                    errors,
                )?;
                if read_ty != place.ty || !read_ty.is_copy() {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), declared.span),
                        "owner Copy read type does not match the static projected place",
                        "declare the local with the exact Copy projection type",
                    );
                    return None;
                }
                if aliases.is_empty() {
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), initializer.span),
                        "owner reads in the lexical block require one active borrow alias",
                        "declare and use a borrow alias before reading owner storage",
                    );
                    return None;
                }
                let hiding = aliases.values().find(|alias| {
                    alias.access == raw::BorrowAccess::Exclusive
                        && root_borrow_paths_overlap(&place, &alias.place)
                });
                if let Some(hiding) = hiding {
                    let root_only =
                        place.projections.is_empty() && hiding.place.projections.is_empty();
                    errors.at(
                        "ZRYNA-M3017",
                        span(input.sources(), initializer.span),
                        if root_only {
                            "owner reads are hidden while an exclusive alias is active"
                        } else {
                            "owner reads are hidden by an overlapping exclusive alias"
                        },
                        if root_only {
                            "read through BorrowMut or wait for lexical end"
                        } else {
                            "read a disjoint sibling, read through BorrowMut, or wait for lexical end"
                        },
                    );
                    return None;
                }
                steps.push(RootBorrowStep::OwnerRead {
                    place,
                    at: span(input.sources(), statement.span),
                });
            }
            reads = reads.checked_add(1)?;
        }
    }
    if aliases.is_empty() || aliases.values().any(|alias| !alias.used) {
        errors.at(
            "ZRYNA-M3017",
            span(input.sources(), nested.span),
            "each lexical borrow alias must be used by an exact Copy read or write",
            "read every Borrow alias and read or write every BorrowMut alias before block exit",
        );
        return None;
    }
    Some(RootBorrowArmPlan {
        steps,
        aliases: aliases.len(),
        reads,
        writes,
        calls,
        call_values,
        block_exit: span(input.sources(), nested.close_brace_span),
    })
}
