use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_syntax::v4::{self as syntax, RawStatementKind};

use super::super::diagnostics::Errors;
use super::super::function_catalog::FunctionCatalog;
use super::super::layout_graph::{Decl, semantic_type};
use super::super::owned_cfg_state::OwnedCfgState;
use super::super::owned_control_flow_resources::preflight_owned_place_capacity;
use super::super::owned_control_flow_shape::{root_is_terminal_if, terminal_owned_if};
use super::super::owner_state::{OwnedVecBranchState, OwnerState};
use super::super::type_model::{Binding, Ty};
use super::super::{SemanticInput, span};
use super::PrivateVecLowerer;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(in crate::data_ownership_v1) fn lower_private_vec_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    catalog: &'a FunctionCatalog,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let file = &input.syntax().files()[module];
    let mut vec_ty = (result.category == TypeCategory::Vec).then_some(result);
    let mut parameter_types = Vec::with_capacity(function.parameters.len());
    for parameter in &function.parameters {
        let ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            declarations,
            graph,
            node_types,
            errors,
        )?;
        if ty.category == TypeCategory::Vec {
            if vec_ty.is_some_and(|found| found != ty) {
                errors.at(
                    "ZRYNA-M3013",
                    span(input.sources(), parameter.span),
                    "function uses more than one exact Vec type",
                    "use one exact Vec<bool>, Vec<i32>, or Vec<String> type",
                );
                return None;
            }
            vec_ty = Some(ty);
        }
        parameter_types.push(ty);
    }
    for statement in &function.body.statements {
        if let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind {
            let ty =
                semantic_type(file, type_syntax, module, declarations, graph, node_types, errors)?;
            if ty.category == TypeCategory::Vec {
                if vec_ty.is_some_and(|found| found != ty) {
                    errors.at(
                        "ZRYNA-M3013",
                        span(input.sources(), statement.span),
                        "function uses more than one exact Vec type",
                        "use one exact Vec<bool>, Vec<i32>, or Vec<String> type",
                    );
                    return None;
                }
                vec_ty = Some(ty);
            }
        }
    }
    let Some(vec_ty) = vec_ty else {
        errors.at(
            "ZRYNA-M3013",
            span(input.sources(), function.span),
            "private Vec operation has no exact declared Vec type",
            "declare and initialize one exact Vec<bool>, Vec<i32>, or Vec<String> local",
        );
        return None;
    };
    if parameter_types.len() > 1
        || parameter_types
            .iter()
            .any(|parameter| *parameter != vec_ty && parameter.category != TypeCategory::Bool)
    {
        errors.at(
            "ZRYNA-M3013",
            span(input.sources(), function.span),
            "private Vec functions admit at most one exact Vec or bool parameter",
            "use a zero-argument producer, one-Vec identity, or one-bool branch function",
        );
        return None;
    }
    let element_layout = layouts.type_by_id(vec_ty.layout)?.referenced_type()?;
    let element = node_types.iter().flatten().find(|ty| ty.layout == element_layout).copied()?;
    if !matches!(element.category, TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String) {
        errors.at(
            "ZRYNA-M3013",
            span(input.sources(), function.span),
            "this Vec element type is outside the bounded owned-data slice",
            "use Vec<bool>, Vec<i32>, or Vec<String>",
        );
        return None;
    }
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let cfg = OwnedCfgState::single_block(span(input.sources(), function.body.span), errors)?;
    let mut lowerer = PrivateVecLowerer {
        input,
        file,
        function,
        module,
        declarations,
        graph,
        node_types,
        catalog,
        layouts,
        vec_ty,
        element,
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
        known_string_bytes: BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    let mut parameters = Vec::with_capacity(function.parameters.len());
    for (index, (parameter, ty)) in function.parameters.iter().zip(parameter_types).enumerate() {
        let parameter_span = span(input.sources(), parameter.span);
        if !preflight_owned_place_capacity(lowerer.places.len(), 1, parameter_span, lowerer.errors)
        {
            return None;
        }
        if lowerer.bindings.keys().any(|name| name.eq_ignore_ascii_case(&parameter.name.text)) {
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
        let value = raw::ValueId(lowerer.next_value);
        let parameter_definition =
            raw::ValueDefinition { id: value, ty: ty.ir, span: parameter_span };
        lowerer.cfg.seed_function_parameter(&parameter_definition, lowerer.errors)?;
        lowerer.next_value = lowerer.next_value.checked_add(1)?;
        parameters.push(parameter_definition);
        let place = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: parameter_span,
            kind: raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        });
        if !ty.is_copy() {
            let _ = lowerer.owners.register_parameter(place);
        }
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
    }
    if root_is_terminal_if(function) {
        if result != vec_ty
            || lowerer.bindings.values().any(|binding| binding.ty.category != TypeCategory::Bool)
        {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(input.sources(), function.span),
                "terminal owned Vec if admits an exact Vec result and one optional bool parameter",
                "return one exact Vec value from both branches",
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
        let incoming = OwnedVecBranchState {
            bindings: lowerer.bindings.clone(),
            owners: lowerer.owners.clone(),
            known_string_bytes: lowerer.known_string_bytes.clone(),
        };
        let arms_lowered = (|| {
            for (block, expression, arm_span) in [
                (then_id, terminal.then_value, terminal.then_span),
                (else_id, terminal.else_value, terminal.else_span),
            ] {
                lowerer.cfg.begin_block(block, Vec::new(), arm_span, lowerer.errors)?;
                let value = lowerer.value(expression, result)?;
                let Some(carried) = lowerer.owners.owner(value) else {
                    lowerer.errors.at(
                        "ZRYNA-M3014",
                        arm_span,
                        "terminal Vec arm result has no available owner",
                        "return one newly produced exact Vec value",
                    );
                    return None;
                };
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
                lowerer.known_string_bytes = incoming.known_string_bytes.clone();
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
                lowerer.lower_root_local(statement, saw_if || saw_loop)?;
            }
            RawStatementKind::Return { value, .. } => {
                returned =
                    Some((lowerer.value(*value, result)?, span(input.sources(), statement.span)));
            }
            RawStatementKind::Assignment { target, value, .. } => {
                lowerer.lower_root_assignment(statement, *target, *value, saw_if || saw_loop)?;
            }
            RawStatementKind::ExpressionStatement { expression, .. } => {
                lowerer.lower_root_push_effect(statement, *expression, saw_if || saw_loop)?;
            }
            RawStatementKind::If { .. } => {
                lowerer.lower_root_if(statement, &mut saw_if, saw_loop)?;
            }
            RawStatementKind::While { .. } => {
                lowerer.lower_root_while(*statement_id, statement, saw_if, &mut saw_loop)?;
            }
            _ => {
                lowerer.errors.at(
                    "ZRYNA-M3013",
                    span(input.sources(), statement.span),
                    "statement is outside private straight-line Vec lowering",
                    "use typed locals and one final Vec return",
                );
                return None;
            }
        }
    }
    let (returned, return_span) = returned?;
    let return_owner = lowerer.owners.owner(returned);
    let cleanup = lowerer.push_cleanup(return_span, return_owner)?;
    if !lowerer.cfg.terminate(
        raw::SpannedTerminator {
            span: return_span,
            kind: raw::Terminator::Return { value: returned, cleanup },
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
