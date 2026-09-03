use std::collections::BTreeMap;

use super::copy_lowering::{BorrowBinding, FunctionLowerer};
use super::function_catalog::FunctionParameterOrder;
use super::{
    Binding, Decl, Errors, FunctionCatalog, RawStatementKind, SemanticInput, Ty, TypeCategory,
    layout, raw, raw_layout, require_current_type_only_boundary, semantic_type, span, syntax,
};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn lower_copy_function<'a>(
    input: SemanticInput<'a>,
    file: &'a syntax::SourceUnit,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let mut lowerer = FunctionLowerer {
        input,
        file,
        function,
        module,
        declarations,
        graph,
        node_types,
        layouts,
        catalog,
        errors,
        bindings: BTreeMap::new(),
        borrow_bindings: BTreeMap::new(),
        projections: BTreeMap::new(),
        places: Vec::new(),
        instructions: Vec::new(),
        cleanup_plans: Vec::new(),
        values: 0,
    };
    let signature = catalog
        .modules
        .get(module)
        .and_then(|signatures| signatures.get(declaration))
        .and_then(Option::as_ref)?;
    debug_assert_eq!(signature.result, result);
    debug_assert_eq!(signature.parameter_order.len(), function.parameters.len());
    let mut parameters = Vec::with_capacity(signature.parameters.len());
    let mut borrow_parameters = Vec::with_capacity(signature.borrow_parameters.len());
    for (parameter, order) in function.parameters.iter().zip(&signature.parameter_order) {
        let parameter_span = span(input.sources(), parameter.span);
        if lowerer.binding_name_exists(&parameter.name.text) {
            lowerer.errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!("parameter '{}' is declared more than once", parameter.name.text),
                "give each parameter one exact name",
            );
            continue;
        }
        match *order {
            FunctionParameterOrder::Value(index) => {
                let ty = *signature.parameters.get(usize::try_from(index).ok()?)?;
                require_current_type_only_boundary(
                    ty,
                    parameter_span,
                    function.export_span.is_some(),
                    lowerer.errors,
                )?;
                debug_assert_eq!(usize::try_from(index).ok(), Some(parameters.len()));
                let value = raw::ValueId(lowerer.values);
                lowerer.values += 1;
                parameters.push(raw::ValueDefinition {
                    id: value,
                    ty: ty.ir,
                    span: parameter_span,
                });
                let place =
                    lowerer.push_place(ty, parameter_span, raw::PlaceKind::Parameter(index));
                lowerer
                    .bindings
                    .insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
            }
            FunctionParameterOrder::Borrow(index) => {
                let descriptor = *signature.borrow_parameters.get(usize::try_from(index).ok()?)?;
                debug_assert_eq!(usize::try_from(index).ok(), Some(borrow_parameters.len()));
                let borrow = raw::BorrowId(index);
                borrow_parameters.push(raw::BorrowParameter {
                    id: borrow,
                    referent: descriptor.referent.ir,
                    access: descriptor.access,
                    span: descriptor.span,
                });
                lowerer.borrow_bindings.insert(
                    parameter.name.text.clone(),
                    BorrowBinding { ty: descriptor.referent, borrow, access: descriptor.access },
                );
            }
        }
    }
    let root =
        usize::try_from(function.body.root_block).ok().and_then(|i| function.body.blocks.get(i));
    let root = root?;
    let mut returned = None;
    for statement_id in &root.statements {
        let Some(statement) =
            usize::try_from(*statement_id).ok().and_then(|i| function.body.statements.get(i))
        else {
            continue;
        };
        match &statement.kind {
            RawStatementKind::LocalDeclaration {
                mutable, name, type_syntax, initializer, ..
            } => {
                let ty = semantic_type(
                    file,
                    *type_syntax,
                    module,
                    declarations,
                    graph,
                    node_types,
                    lowerer.errors,
                )?;
                require_current_type_only_boundary(
                    ty,
                    span(input.sources(), statement.span),
                    false,
                    lowerer.errors,
                )?;
                let value = lowerer.value(*initializer)?;
                lowerer.require_type(
                    ty,
                    value.0,
                    span(input.sources(), statement.span),
                    "local initializer",
                )?;
                let place = lowerer.push_place(
                    ty,
                    span(input.sources(), statement.span),
                    raw::PlaceKind::Local(u32::try_from(lowerer.bindings.len()).ok()?),
                );
                lowerer.emit(
                    None,
                    span(input.sources(), statement.span),
                    raw::InstructionKind::InitializePlace { place, value: value.1 },
                );
                if lowerer.binding_name_exists(&name.text) {
                    lowerer.errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!(
                            "binding '{}' collides under portable ASCII case folding",
                            name.text
                        ),
                        "give every binding one portable case-insensitive unique name",
                    );
                } else {
                    lowerer
                        .bindings
                        .insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
                }
            }
            RawStatementKind::Assignment { target, value, .. } => {
                if let Some(binding) = lowerer.borrow_reference(*target) {
                    if binding.access != raw::BorrowAccess::Exclusive {
                        lowerer.errors.at(
                            "ZRYNA-M3016",
                            span(input.sources(), function.body.expressions[*target as usize].span),
                            "shared borrow parameters are read-only",
                            "write only through an exact BorrowMut parameter",
                        );
                        return None;
                    }
                    let value = lowerer.value(*value)?;
                    lowerer.require_type(
                        binding.ty,
                        value.0,
                        span(input.sources(), statement.span),
                        "borrow write",
                    )?;
                    lowerer.emit(
                        None,
                        span(input.sources(), statement.span),
                        raw::InstructionKind::BorrowWrite {
                            borrow: binding.borrow,
                            value: value.1,
                        },
                    );
                    continue;
                }
                let (target_ty, place, mutable) = lowerer.place(*target)?;
                if !mutable {
                    lowerer.errors.at(
                        "ZRYNA-M3007",
                        span(input.sources(), statement.span),
                        "assignment target is not rooted in a mutable local",
                        "declare the root with let mut before assigning",
                    );
                    return None;
                }
                let value = lowerer.value(*value)?;
                lowerer.require_type(
                    target_ty,
                    value.0,
                    span(input.sources(), statement.span),
                    "assignment",
                )?;
                lowerer.emit(
                    None,
                    span(input.sources(), statement.span),
                    raw::InstructionKind::ReplacePlace { place, value: value.1 },
                );
            }
            RawStatementKind::Return { value, .. } => {
                let value = lowerer.value(*value)?;
                lowerer.require_type(
                    result,
                    value.0,
                    span(input.sources(), statement.span),
                    "return",
                )?;
                returned = Some((value.1, span(input.sources(), statement.span)));
            }
            _ => {
                lowerer.errors.at(
                    "ZRYNA-M3008",
                    span(input.sources(), statement.span),
                    "this statement form is outside deterministic aggregate M3",
                    "use local initialization, aggregate assignment, and one value return",
                );
                return None;
            }
        }
    }
    let Some((return_value, return_span)) = returned else {
        lowerer.errors.at(
            "ZRYNA-M3010",
            span(input.sources(), function.body.span),
            "function has no value return",
            "return one value of the exact declared type",
        );
        return None;
    };
    let cleanup = raw::CleanupPlanId(u32::try_from(lowerer.cleanup_plans.len()).ok()?);
    lowerer.cleanup_plans.push(raw::CleanupPlan {
        id: cleanup,
        span: span(input.sources(), function.body.span),
        actions: Vec::new(),
    });
    let block = raw::Block {
        id: raw::BlockId(0),
        parameters: Vec::new(),
        instructions: lowerer.instructions,
        terminators: vec![raw::SpannedTerminator {
            span: return_span,
            kind: raw::Terminator::Return { value: return_value, cleanup },
        }],
    };
    let entry_export = if input.entry() == file.id() && function.export_span.is_some() {
        let aggregate_parameter = parameters.iter().any(|parameter| {
            layouts
                .types()
                .nth(usize::try_from(parameter.ty.0).unwrap_or(usize::MAX))
                .is_none_or(|ty| !matches!(ty.category(), TypeCategory::Bool | TypeCategory::I32))
        });
        if !matches!(result.category, TypeCategory::Bool | TypeCategory::I32) || aggregate_parameter
        {
            lowerer.errors.at(
                "ZRYNA-M3010",
                span(input.sources(), function.span),
                "public aggregate signatures are outside scalar ABI v1",
                "keep aggregate functions internal and export only bool/i32 signatures",
            );
        }
        Some(function.name.text.clone())
    } else {
        None
    };
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export,
        span: span(input.sources(), function.span),
        parameters,
        borrow_parameters,
        result: result.ir,
        places: lowerer.places,
        blocks: vec![block],
        cleanup_plans: lowerer.cleanup_plans,
    })
}
