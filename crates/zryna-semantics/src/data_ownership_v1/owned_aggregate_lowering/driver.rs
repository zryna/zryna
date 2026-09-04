use std::collections::{BTreeMap, BTreeSet};

use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_syntax::v4::{self as syntax, RawStatementKind};

use super::super::diagnostics::Errors;
use super::super::layout_graph::{Decl, semantic_type};
use super::super::owner_state::OwnerState;
use super::super::type_model::{Binding, Ty};
use super::super::{SemanticInput, span};
use super::{
    PrivateOwnedAggregateLowerer, StatementOutcome, aggregate_graph_is_supported,
    owned_enum_graph_is_supported,
};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_owned_aggregate_function_impl<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    if !if result.category == TypeCategory::Enum {
        owned_enum_graph_is_supported(result, layouts)
    } else {
        aggregate_graph_is_supported(result, layouts, &mut BTreeSet::new())
    } {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), function.span),
            "owned aggregate graph contains an unsupported nested enum, Vec, handle, borrow, or cycle",
            "use an acyclic Struct/Enum/FixedArray graph with only bool, i32, and String leaves",
        );
        return None;
    }
    let file = &input.syntax().files()[module];
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    if result.category == TypeCategory::Enum && !function.parameters.is_empty() {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), function.parameters[0].span),
            "private owned enum functions do not admit parameters",
            "construct the enum from literals and explicitly typed initialized locals",
        );
        return None;
    }
    let mut lowerer = PrivateOwnedAggregateLowerer {
        input,
        file,
        function,
        module,
        declarations,
        graph,
        node_types,
        layouts,
        errors,
        bindings: BTreeMap::new(),
        projections: BTreeMap::new(),
        moved_projections: BTreeSet::new(),
        partial_roots: BTreeSet::new(),
        places: Vec::new(),
        instructions: Vec::new(),
        constructor_types: super::ConstructorValueTypes::default(),
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        aggregate_operands: 0,
        aggregate_subobject_moves: 0,
        projected_aggregate_clones: 0,
        projected_aggregate_assignments: 0,
        reserved_transitions: 0,
        owners: OwnerState::default(),
        next_value: 0,
        next_local: 0,
    };
    let mut parameters = Vec::with_capacity(function.parameters.len());
    for (index, parameter) in function.parameters.iter().enumerate() {
        let ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            declarations,
            graph,
            node_types,
            lowerer.errors,
        )?;
        if !ty.is_copy() || !matches!(ty.category, TypeCategory::Bool | TypeCategory::I32) {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(input.sources(), parameter.span),
                "owned aggregate functions do not admit owned or aggregate parameters",
                "use only optional bool/i32 parameters in this private checkpoint",
            );
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
                "give every binding one portable case-insensitive unique name",
            );
            return None;
        }
        if lowerer.next_value as usize >= ir::MAX_VALUES_PER_FUNCTION
            || lowerer.places.len() >= ir::MAX_PLACES_PER_FUNCTION
        {
            lowerer.errors.at(
                "ZRYNA-M3201",
                span(input.sources(), parameter.span),
                "derived aggregate parameter storage exceeds an M3 resource limit",
                "reduce private aggregate parameters",
            );
            return None;
        }
        let value = raw::ValueId(lowerer.next_value);
        lowerer.next_value += 1;
        parameters.push(raw::ValueDefinition {
            id: value,
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
        });
        lowerer
            .constructor_types
            .record_parameter(parameters.last().expect("new parameter"))
            .expect("aggregate parameters have dense emitted identities");
        let place = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
            kind: raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        });
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
    }
    let mut returned = None;
    let final_statement = root.statements.last().copied();
    let return_count = root
        .statements
        .iter()
        .filter(|statement_id| {
            usize::try_from(**statement_id)
                .ok()
                .and_then(|index| function.body.statements.get(index))
                .is_some_and(|statement| matches!(statement.kind, RawStatementKind::Return { .. }))
        })
        .count();
    for statement_id in &root.statements {
        let statement = usize::try_from(*statement_id)
            .ok()
            .and_then(|index| function.body.statements.get(index))?;
        if let StatementOutcome::Return(value, at) = lowerer.lower_statement(
            *statement_id,
            statement,
            result,
            final_statement,
            return_count,
        )? {
            returned = Some((value, at));
        }
    }
    let (returned, return_span) = returned?;
    let return_owner = lowerer.owners.owner(returned);
    let cleanup = lowerer.push_cleanup(return_span, return_owner)?;
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
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: Vec::new(),
            instructions: lowerer.instructions,
            terminators: vec![raw::SpannedTerminator {
                span: return_span,
                kind: raw::Terminator::Return { value: returned, cleanup },
            }],
        }],
        cleanup_plans: lowerer.cleanup_plans,
    })
}

pub(in crate::data_ownership_v1) fn is_private_owned_aggregate_candidate(
    function: &syntax::RawFunctionSyntax,
    result: Ty,
) -> bool {
    function.export_span.is_none()
        && !result.is_copy()
        && matches!(
            result.category,
            TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
        )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::data_ownership_v1) fn lower_private_owned_aggregate_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let diagnostics_before = errors.len();
    let lowered = lower_owned_aggregate_function_impl(
        input,
        module,
        declaration,
        function,
        declarations,
        graph,
        node_types,
        layouts,
        result,
        errors,
    );
    if lowered.is_none() && errors.len() == diagnostics_before {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), function.span),
            "private owned aggregate lowering rejected a source function without a specific diagnostic",
            "use only the exact straight-line owned Struct/Enum/FixedArray forms",
        );
    }
    lowered
}
