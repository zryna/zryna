use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{TypeCategory, raw as raw_layout};
use zryna_syntax::v4::{self as syntax, RawStatementKind};

use super::super::diagnostics::Errors;
use super::super::function_catalog::FunctionCatalog;
use super::super::layout_graph::Decl;
use super::super::owned_cfg_state::OwnedCfgState;
use super::super::owned_control_flow_resources::preflight_owned_place_capacity;
use super::super::owned_control_flow_shape::{root_is_terminal_if, terminal_owned_if};
use super::super::owner_state::{OwnedStringBranchState, OwnerState};
use super::super::type_model::{Binding, Ty};
use super::super::{SemanticInput, semantic_type, span};
use super::{PrivateStringLowerer, StringBranchTypes};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(in crate::data_ownership_v1) fn lower_private_string_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    result: Ty,
    catalog: &'a FunctionCatalog,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    if function.parameters.len() > 1 {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), function.span),
            "private String calls admit at most one by-value parameter",
            "use a zero-argument producer or one-String identity function",
        );
        return None;
    }
    let file = &input.syntax().files()[module];
    let cfg = OwnedCfgState::single_block(span(input.sources(), function.body.span), errors)?;
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module,
        ty: result,
        catalog,
        errors,
        bindings: BTreeMap::new(),
        places: Vec::new(),
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners: OwnerState::default(),
        known_bytes: BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    let mut parameters = Vec::with_capacity(function.parameters.len());
    for (index, parameter) in function.parameters.iter().enumerate() {
        let parameter_ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            declarations,
            graph,
            node_types,
            lowerer.errors,
        )?;
        if parameter_ty != result && parameter_ty.category != TypeCategory::Bool {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(input.sources(), parameter.span),
                "private String lowering admits only one String or bool parameter",
                "use one owned String argument or one Copy bool branch condition",
            );
            return None;
        }
        if lowerer
            .bindings
            .keys()
            .any(|existing| existing.eq_ignore_ascii_case(&parameter.name.text))
        {
            lowerer.errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!(
                    "parameter '{}' collides under portable ASCII case folding",
                    parameter.name.text
                ),
                "give every parameter one portable case-insensitive unique name",
            );
            return None;
        }
        let parameter_span = span(input.sources(), parameter.span);
        if !preflight_owned_place_capacity(lowerer.places.len(), 1, parameter_span, lowerer.errors)
        {
            return None;
        }
        let value = raw::ValueId(lowerer.next_value);
        let parameter_definition =
            raw::ValueDefinition { id: value, ty: parameter_ty.ir, span: parameter_span };
        lowerer.cfg.seed_function_parameter(&parameter_definition, lowerer.errors)?;
        lowerer.next_value = lowerer.next_value.checked_add(1)?;
        parameters.push(parameter_definition);
        let place = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: place,
            ty: parameter_ty.ir,
            span: parameter_span,
            kind: raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        });
        if parameter_ty == result {
            let _ = lowerer.owners.register_parameter(place);
            lowerer.known_bytes.insert(place, None);
        }
        lowerer.bindings.insert(
            parameter.name.text.clone(),
            Binding { ty: parameter_ty, place, mutable: false },
        );
    }
    if root_is_terminal_if(function) {
        if lowerer.bindings.values().any(|binding| binding.ty.category != TypeCategory::Bool) {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(input.sources(), function.span),
                "terminal owned String if admits only one optional bool parameter",
                "move owned inputs into branch-local producer expressions",
            );
            return None;
        }
        let terminal = terminal_owned_if(function, input.sources(), lowerer.errors)?;
        let bool_ty =
            node_types.iter().flatten().find(|ty| ty.category == TypeCategory::Bool).copied()?;
        if !lowerer.cfg.preflight_skeleton(3, 4, terminal.span, lowerer.errors) {
            return None;
        }
        lowerer.cfg.reserve_values(1, terminal.span, lowerer.errors)?;
        if !lowerer.reserve_local_place(terminal.span) {
            lowerer.cfg.release_values(1);
            return None;
        }
        if !lowerer.reserve_cleanup_capacity(0, terminal.span) {
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        let Some(condition) = lowerer.condition(terminal.condition, bool_ty) else {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        };
        let then_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        let else_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        let join_id = lowerer
            .cfg
            .reserve_block(terminal.span, lowerer.errors)
            .expect("terminal skeleton block capacity was preflighted");
        if !lowerer.cfg.terminate(
            raw::SpannedTerminator {
                span: terminal.span,
                kind: raw::Terminator::Branch {
                    condition,
                    when_true: raw::Edge { target: then_id, arguments: Vec::new() },
                    when_false: raw::Edge { target: else_id, arguments: Vec::new() },
                },
            },
            lowerer.errors,
        ) {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        let incoming = OwnedStringBranchState {
            bindings: lowerer.bindings.clone(),
            owners: lowerer.owners.clone(),
            known_bytes: lowerer.known_bytes.clone(),
        };
        let arms_lowered = (|| {
            for (block, expression, arm_span) in [
                (then_id, terminal.then_value, terminal.then_span),
                (else_id, terminal.else_value, terminal.else_span),
            ] {
                lowerer.cfg.begin_block(block, Vec::new(), arm_span, lowerer.errors)?;
                let (value, carried) = lowerer.value(expression)?;
                lowerer.drop_non_carried(carried, arm_span)?;
                if !lowerer.cfg.terminate(
                    raw::SpannedTerminator {
                        span: arm_span,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: join_id,
                            arguments: vec![value],
                        }),
                    },
                    lowerer.errors,
                ) {
                    return None;
                }
                lowerer.bindings = incoming.bindings.clone();
                lowerer.owners = incoming.owners.clone();
                lowerer.known_bytes = incoming.known_bytes.clone();
            }
            Some(())
        })();
        if arms_lowered.is_none() {
            lowerer.release_cleanup_capacity(0);
            lowerer.release_local_place();
            lowerer.cfg.release_values(1);
            return None;
        }
        lowerer.release_cleanup_capacity(0);
        lowerer.release_local_place();
        lowerer.cfg.release_values(1);
        let joined = raw::ValueId(lowerer.next_value);
        let joined_definition =
            raw::ValueDefinition { id: joined, ty: result.ir, span: terminal.span };
        lowerer.next_value = lowerer.next_value.checked_add(1)?;
        lowerer.cfg.begin_block(join_id, vec![joined_definition], terminal.span, lowerer.errors)?;
        let joined_owner = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: joined_owner,
            ty: result.ir,
            span: terminal.span,
            kind: raw::PlaceKind::Temporary(joined),
        });
        let _ = lowerer.owners.register(joined, joined_owner);
        lowerer.known_bytes.insert(joined_owner, None);
        let cleanup = lowerer.push_cleanup(terminal.span, Some(joined_owner))?;
        if !lowerer.cfg.terminate(
            raw::SpannedTerminator {
                span: terminal.span,
                kind: raw::Terminator::Return { value: joined, cleanup },
            },
            lowerer.errors,
        ) {
            return None;
        }
        let blocks = lowerer.cfg.finish(terminal.span, lowerer.errors)?;
        return Some(raw::Function {
            id: raw::FunctionId {
                module: raw::ModuleId(u32::try_from(module).ok()?),
                declaration: u32::try_from(declaration).ok()?,
            },
            entry_export: None,
            span: span(input.sources(), function.span),
            parameters,
            borrow_parameters: Vec::new(),
            result: result.ir,
            places: lowerer.places,
            blocks,
            cleanup_plans: lowerer.cleanup_plans,
        });
    }
    let mut returned = None;
    let mut saw_if = false;
    let mut saw_loop = false;
    for statement_id in &root.statements {
        let statement = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        match &statement.kind {
            RawStatementKind::LocalDeclaration { .. } => {
                let types = StringBranchTypes { file, declarations, graph, node_types };
                lowerer.lower_root_local(statement, saw_if || saw_loop, types)?;
            }
            RawStatementKind::Assignment { .. } => {
                lowerer.lower_root_assignment(statement, saw_if || saw_loop)?;
            }
            RawStatementKind::If { .. } => {
                let types = StringBranchTypes { file, declarations, graph, node_types };
                lowerer.lower_root_branch(statement, &mut saw_if, saw_loop, types)?;
            }
            RawStatementKind::While { .. } => {
                let types = StringBranchTypes { file, declarations, graph, node_types };
                lowerer.lower_root_loop(*statement_id, statement, saw_if, &mut saw_loop, types)?;
            }
            RawStatementKind::Return { value, .. } => {
                let value = lowerer.value(*value)?;
                returned = Some((value, span(input.sources(), statement.span)));
            }
            _ => {
                lowerer.errors.at(
                    "ZRYNA-M3012",
                    span(input.sources(), statement.span),
                    "this statement is outside straight-line private String lowering",
                    "use typed local initialization and one final return",
                );
                return None;
            }
        }
    }
    let ((return_value, returned_place), return_span) = returned?;
    let return_owner = lowerer.owners.owner(return_value)?;
    debug_assert_eq!(return_owner, returned_place);
    let return_cleanup = lowerer.push_cleanup(return_span, Some(return_owner))?;
    if !lowerer.cfg.terminate(
        raw::SpannedTerminator {
            span: return_span,
            kind: raw::Terminator::Return { value: return_value, cleanup: return_cleanup },
        },
        lowerer.errors,
    ) {
        return None;
    }
    let blocks = lowerer.cfg.finish(return_span, lowerer.errors)?;
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: span(input.sources(), function.span),
        parameters,
        borrow_parameters: Vec::new(),
        result: result.ir,
        places: lowerer.places,
        blocks,
        cleanup_plans: lowerer.cleanup_plans,
    })
}
