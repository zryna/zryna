use std::collections::{BTreeMap, BTreeSet};

use zryna_ownership_runtime_abi::{LogicalOperation, VerifiedOwnershipRuntimeAbi};

use super::{VerifiedMirModule, error, raw};

const MAX_DIAGNOSTICS: usize = 256;

/// Independently verifies untrusted Linux x86-64 `DataOwnershipV1` MIR claims.
///
/// # Errors
/// Returns deterministic bounded diagnostics and no partial verified module.
pub fn verify(
    program: raw::Program,
    layouts: &zryna_layout::VerifiedLayouts,
    runtime: &VerifiedOwnershipRuntimeAbi,
) -> Result<VerifiedMirModule, Vec<zryna_diagnostics::Diagnostic>> {
    let mut errors = Errors::default();
    verify_authority(&program, layouts, runtime, &mut errors);
    verify_types(&program, layouts, &mut errors);
    verify_symbols(&program, runtime, &mut errors);
    verify_functions(&program, layouts, runtime, &mut errors);
    if errors.items.is_empty() { Ok(VerifiedMirModule { program }) } else { Err(errors.items) }
}

fn verify_authority(
    program: &raw::Program,
    layouts: &zryna_layout::VerifiedLayouts,
    runtime: &VerifiedOwnershipRuntimeAbi,
    errors: &mut Errors,
) {
    if program.authority.type_universe != layouts.universe_identity().as_bytes()
        || program.authority.linux_layout != *layouts.fingerprint()
        || program.authority.runtime_identifier != runtime.identifier()
        || runtime.type_universe_identity() != layouts.universe_identity()
        || runtime.linux_x86_64_fingerprint() != *layouts.fingerprint()
    {
        errors.push(
            "ZRYNA-N3101",
            "native MIR authority does not match the sealed Linux layout and runtime ABI",
        );
    }
}

fn verify_types(
    program: &raw::Program,
    layouts: &zryna_layout::VerifiedLayouts,
    errors: &mut Errors,
) {
    if program.types.len() != layouts.types().len() {
        errors.push("ZRYNA-N3102", "native MIR type inventory is incomplete");
        return;
    }
    for (index, (claim, sealed)) in program.types.iter().zip(layouts.types()).enumerate() {
        let category = match sealed.category() {
            zryna_layout::TypeCategory::Bool => raw::TypeCategory::Bool,
            zryna_layout::TypeCategory::I32 => raw::TypeCategory::I32,
            zryna_layout::TypeCategory::Struct => raw::TypeCategory::Struct,
            zryna_layout::TypeCategory::Enum => raw::TypeCategory::Enum,
            zryna_layout::TypeCategory::FixedArray => raw::TypeCategory::FixedArray,
            zryna_layout::TypeCategory::String => raw::TypeCategory::String,
            zryna_layout::TypeCategory::Vec => raw::TypeCategory::Vec,
            zryna_layout::TypeCategory::Shared => raw::TypeCategory::Shared,
            zryna_layout::TypeCategory::Weak => raw::TypeCategory::Weak,
        };
        if claim.id as usize != index
            || claim.category != category
            || claim.size != sealed.size()
            || claim.alignment != sealed.alignment()
            || claim.drop_kind != sealed.drop_kind()
            || claim.runtime_kind != sealed.runtime_kind()
        {
            errors.push(
                "ZRYNA-N3102",
                format!("native MIR type #{index} differs from the sealed layout"),
            );
        }
    }
}

fn verify_symbols(
    program: &raw::Program,
    runtime: &VerifiedOwnershipRuntimeAbi,
    errors: &mut Errors,
) {
    let expected = runtime
        .native_linux_x86_64_functions()
        .map(zryna_ownership_runtime_abi::VerifiedNativeFunction::symbol)
        .collect::<Vec<_>>();
    if program.runtime_symbols.iter().map(String::as_str).collect::<Vec<_>>() != expected {
        errors.push("ZRYNA-N3103", "native MIR runtime symbol inventory is not exact or canonical");
    }
    let mut folded = BTreeSet::new();
    for symbol in &program.runtime_symbols {
        if !valid_symbol(symbol) || !folded.insert(symbol.to_ascii_lowercase()) {
            errors
                .push("ZRYNA-N3103", "native MIR contains an invalid or colliding runtime symbol");
        }
    }
}

#[allow(clippy::too_many_lines)]
fn verify_functions(
    program: &raw::Program,
    layouts: &zryna_layout::VerifiedLayouts,
    runtime: &VerifiedOwnershipRuntimeAbi,
    errors: &mut Errors,
) {
    let identities = program
        .functions
        .iter()
        .map(|function| (function.module, function.declaration))
        .collect::<BTreeSet<_>>();
    if identities.len() != program.functions.len()
        || program.functions.len() > zryna_ir::data_ownership_v1::MAX_FUNCTIONS_PER_PROGRAM
    {
        errors.push("ZRYNA-N3104", "native MIR function identities are duplicate or over budget");
    }
    let runtime_symbols = runtime
        .operations()
        .zip(runtime.native_linux_x86_64_functions())
        .map(|(operation, function)| (operation.operation(), function.symbol()))
        .collect::<BTreeMap<_, _>>();
    for function in &program.functions {
        if function.symbol != format!("zryna_m3_m{}_f{}", function.module, function.declaration)
            || !valid_symbol(&function.symbol)
        {
            errors.push("ZRYNA-N3104", "native MIR function symbol is not canonical");
        }
        if type_at(program, function.result_type).is_none() {
            errors.push("ZRYNA-N3105", "native MIR result type is unknown");
        }
        let mut next_value = 0_u32;
        for parameter in &function.parameters {
            if parameter.id != next_value || type_at(program, parameter.ty).is_none() {
                errors.push("ZRYNA-N3105", "native MIR parameter is not dense and typed");
            }
            next_value = next_value.saturating_add(1);
        }
        let mut next_borrow = 0_u32;
        for parameter in &function.borrow_parameters {
            if parameter.id != next_borrow || type_at(program, parameter.referent).is_none() {
                errors.push("ZRYNA-N3105", "native MIR borrow parameter is not dense and typed");
            }
            next_borrow = next_borrow.saturating_add(1);
        }
        verify_places(program, function, layouts, errors);
        if function.blocks.len() > zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION {
            errors.push("ZRYNA-N3201", "native MIR block budget exceeded");
        }
        for (block_index, block) in function.blocks.iter().enumerate() {
            if block.id as usize != block_index {
                errors.push("ZRYNA-N3106", "native MIR block identity is not dense");
            }
            for parameter in &block.parameters {
                if parameter.id != next_value || type_at(program, parameter.ty).is_none() {
                    errors.push("ZRYNA-N3105", "native MIR block parameter is not dense and typed");
                }
                next_value = next_value.saturating_add(1);
            }
            for operation in &block.operations {
                verify_operation_shape(operation, errors);
                for value in &operation.values {
                    if *value >= next_value {
                        errors.push("ZRYNA-N3107", "native MIR operation uses an undefined value");
                    }
                }
                let defines_borrow = matches!(
                    operation.opcode,
                    raw::Opcode::BeginBorrow | raw::Opcode::BeginIndexedBorrow
                );
                let defines_projected = matches!(
                    operation.opcode,
                    raw::Opcode::ProjectBorrow | raw::Opcode::BindBorrow
                );
                for (index, borrow) in operation.borrows.iter().enumerate() {
                    let definition = defines_borrow || (defines_projected && index == 1);
                    if (definition && *borrow != next_borrow)
                        || (!definition && *borrow >= next_borrow)
                    {
                        errors.push("ZRYNA-N3107", "native MIR operation uses an invalid borrow");
                    }
                    if definition {
                        next_borrow = next_borrow.saturating_add(1);
                    }
                }
                for place in &operation.places {
                    if (*place as usize) >= function.places.len() {
                        errors.push("ZRYNA-N3107", "native MIR operation uses an undefined place");
                    }
                }
                if let Some(result) = operation.result {
                    if result.id != next_value || type_at(program, result.ty).is_none() {
                        errors.push("ZRYNA-N3105", "native MIR result is not dense and typed");
                    }
                    next_value = next_value.saturating_add(1);
                }
                if operation.callee.is_some_and(|callee| !identities.contains(&callee)) {
                    errors.push("ZRYNA-N3108", "native MIR call target is unknown");
                }
                verify_runtime_operation(operation, &runtime_symbols, errors);
                if operation
                    .cleanup
                    .is_some_and(|plan| plan as usize >= function.cleanup_plans.len())
                {
                    errors.push("ZRYNA-N3112", "native MIR operation cleanup is unknown");
                }
            }
            verify_terminator(block, function, &identities, &runtime_symbols, next_value, errors);
            if block.cleanup.is_some_and(|plan| plan as usize >= function.cleanup_plans.len()) {
                errors.push("ZRYNA-N3112", "native MIR terminator cleanup is unknown");
            }
        }
        verify_cleanup(program, function, errors);
    }
}

fn verify_places(
    program: &raw::Program,
    function: &raw::Function,
    layouts: &zryna_layout::VerifiedLayouts,
    errors: &mut Errors,
) {
    if function.places.len() > zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION {
        errors.push("ZRYNA-N3201", "native MIR place budget exceeded");
    }
    for (index, place) in function.places.iter().enumerate() {
        if place.id as usize != index || type_at(program, place.ty).is_none() {
            errors.push("ZRYNA-N3109", "native MIR place is not dense and typed");
        }
        let valid = match place.kind {
            raw::PlaceKind::Parameter(ordinal) => function
                .parameters
                .get(ordinal as usize)
                .is_some_and(|parameter| parameter.ty == place.ty),
            raw::PlaceKind::Local(ordinal) => (ordinal as usize) < function.places.len(),
            raw::PlaceKind::Temporary(value) => function
                .blocks
                .iter()
                .flat_map(|block| block.operations.iter())
                .filter_map(|operation| operation.result)
                .chain(function.parameters.iter().copied())
                .chain(function.blocks.iter().flat_map(|block| block.parameters.iter().copied()))
                .any(|definition| definition.id == value && definition.ty == place.ty),
            raw::PlaceKind::Field { base, offset } => function
                .places
                .get(base as usize)
                .and_then(|base| layouts.types().nth(base.ty as usize))
                .is_some_and(|ty| {
                    ty.fields()
                        .iter()
                        .any(|field| field.offset() == offset && field.ty().index() == place.ty)
                }),
            raw::PlaceKind::EnumPayload { base, offset, variant } => function
                .places
                .get(base as usize)
                .and_then(|base| layouts.types().nth(base.ty as usize))
                .is_some_and(|ty| {
                    ty.enum_payload_layout().is_some_and(|layout| layout.0 == offset)
                        && ty.variants().iter().any(|item| {
                            item.ordinal() == variant
                                && item.payload().is_some_and(|payload| payload.index() == place.ty)
                        })
                }),
            raw::PlaceKind::ArrayElement { base, offset } => function
                .places
                .get(base as usize)
                .and_then(|base| layouts.types().nth(base.ty as usize))
                .is_some_and(|ty| {
                    ty.array_stride().is_some_and(|stride| {
                        stride != 0
                            && offset % stride == 0
                            && offset / stride < ty.array_length().unwrap_or(0)
                            && ty
                                .referenced_type()
                                .is_some_and(|element| element.index() == place.ty)
                    })
                }),
        };
        if !valid {
            errors.push("ZRYNA-N3109", "native MIR place address is not layout-bound");
        }
    }
}

fn verify_runtime_operation(
    operation: &raw::Operation,
    symbols: &BTreeMap<LogicalOperation, &str>,
    errors: &mut Errors,
) {
    let expected = match operation.opcode {
        raw::Opcode::String => Some(LogicalOperation::StringFromUtf8Copy),
        raw::Opcode::StringConcat => Some(LogicalOperation::StringConcat),
        raw::Opcode::VecConstruct => Some(LogicalOperation::VecAllocate),
        raw::Opcode::VecPush => Some(LogicalOperation::VecReserve),
        raw::Opcode::SharedClone => Some(LogicalOperation::StrongClone),
        raw::Opcode::WeakDowngrade => Some(LogicalOperation::WeakDowngrade),
        raw::Opcode::WeakClone => Some(LogicalOperation::WeakClone),
        _ => None,
    };
    if expected.and_then(|kind| symbols.get(&kind).copied()) != operation.runtime_symbol.as_deref()
    {
        errors.push("ZRYNA-N3110", "native MIR operation has a missing or unapproved runtime call");
    }
}

fn verify_operation_shape(operation: &raw::Operation, errors: &mut Errors) {
    let (values, places, borrows, result) = match operation.opcode {
        raw::Opcode::BoolLiteral | raw::Opcode::I32Literal | raw::Opcode::String => {
            (Some(0), Some(0), Some(0), true)
        }
        raw::Opcode::I32Add
        | raw::Opcode::I32Sub
        | raw::Opcode::I32Mul
        | raw::Opcode::Eq
        | raw::Opcode::Ne
        | raw::Opcode::I32LtS
        | raw::Opcode::I32LeS
        | raw::Opcode::I32GtS
        | raw::Opcode::I32GeS => (Some(2), Some(0), Some(0), true),
        raw::Opcode::I32Neg | raw::Opcode::SharedConstruct => (Some(1), Some(0), Some(0), true),
        raw::Opcode::Call | raw::Opcode::Construct | raw::Opcode::VecConstruct => {
            (None, Some(0), None, true)
        }
        raw::Opcode::Copy | raw::Opcode::Move | raw::Opcode::Clone => (Some(0), None, None, true),
        raw::Opcode::Initialize | raw::Opcode::Replace | raw::Opcode::VecPush => {
            (Some(1), Some(1), Some(0), false)
        }
        raw::Opcode::Drop => (Some(0), Some(1), Some(0), false),
        raw::Opcode::Discriminant => (Some(0), Some(1), Some(0), true),
        raw::Opcode::Index => (Some(1), Some(1), Some(0), true),
        raw::Opcode::StringConcat => (Some(0), Some(2), Some(0), true),
        raw::Opcode::SharedClone | raw::Opcode::WeakDowngrade | raw::Opcode::WeakClone => {
            (Some(0), Some(1), Some(0), true)
        }
        raw::Opcode::BeginBorrow => (Some(0), Some(1), Some(1), false),
        raw::Opcode::BeginIndexedBorrow => (Some(1), Some(1), Some(1), false),
        raw::Opcode::ProjectBorrow => (Some(1), Some(0), Some(2), false),
        raw::Opcode::BindBorrow => (Some(0), Some(0), Some(2), false),
        raw::Opcode::BorrowReplace | raw::Opcode::BorrowWrite => (Some(1), Some(0), Some(1), false),
        raw::Opcode::BorrowRead => (Some(0), Some(0), Some(1), true),
        raw::Opcode::EndBorrow => (Some(0), Some(0), Some(1), false),
    };
    let one_source = operation.opcode != raw::Opcode::Copy
        && operation.opcode != raw::Opcode::Move
        && operation.opcode != raw::Opcode::Clone
        || operation.places.len() + operation.borrows.len() == 1;
    if values.is_some_and(|count| operation.values.len() != count)
        || places.is_some_and(|count| operation.places.len() != count)
        || borrows.is_some_and(|count| operation.borrows.len() != count)
        || operation.result.is_some() != result
        || operation.callee.is_some() != (operation.opcode == raw::Opcode::Call)
        || !one_source
    {
        errors.push("ZRYNA-N3107", "native MIR operation operand shape is not canonical");
    }
}

fn verify_terminator(
    block: &raw::Block,
    function: &raw::Function,
    _: &BTreeSet<(u32, u32)>,
    symbols: &BTreeMap<LogicalOperation, &str>,
    values: u32,
    errors: &mut Errors,
) {
    let edge = |edge: &raw::Edge, synthesized: usize| {
        function.blocks.get(edge.target as usize).is_some_and(|target| {
            edge.arguments.len() + synthesized == target.parameters.len()
                && edge.arguments.iter().all(|value| *value < values)
        })
    };
    let valid = match &block.terminator {
        raw::Terminator::Return(value) => *value < values,
        raw::Terminator::Jump(target) => edge(target, 0),
        raw::Terminator::Branch { condition, when_true, when_false } => {
            *condition < values && edge(when_true, 0) && edge(when_false, 0)
        }
        raw::Terminator::EnumMatch { place, arms } => {
            (*place as usize) < function.places.len()
                && !arms.is_empty()
                && arms.iter().all(|(_, target)| edge(target, 0))
        }
        raw::Terminator::WeakUpgrade { weak, success, expired, runtime_symbol } => {
            (*weak as usize) < function.places.len()
                && edge(success, 1)
                && edge(expired, 0)
                && symbols.get(&LogicalOperation::WeakUpgrade).copied()
                    == Some(runtime_symbol.as_str())
        }
        raw::Terminator::Trap(identity) => *identity < 5,
    };
    if !valid {
        errors.push(
            "ZRYNA-N3111",
            "native MIR terminator has an invalid operand, edge, or runtime call",
        );
    }
}

fn verify_cleanup(program: &raw::Program, function: &raw::Function, errors: &mut Errors) {
    if function.cleanup_plans.len() > zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION {
        errors.push("ZRYNA-N3201", "native MIR cleanup plan budget exceeded");
    }
    for (index, plan) in function.cleanup_plans.iter().enumerate() {
        if plan.id as usize != index {
            errors.push("ZRYNA-N3112", "native MIR cleanup identity is not dense");
        }
        for action in &plan.actions {
            let Some(place) = function.places.get(action.place as usize) else {
                errors.push("ZRYNA-N3112", "native MIR cleanup uses an unknown place");
                continue;
            };
            let category = type_at(program, place.ty).map(|ty| ty.category);
            let valid = match action.kind {
                raw::DropKind::Place => {
                    type_at(program, place.ty).is_some_and(|ty| ty.drop_kind != 0)
                }
                raw::DropKind::VecPrefix => category == Some(raw::TypeCategory::Vec),
                raw::DropKind::AggregatePrefix | raw::DropKind::GenericPrefix => matches!(
                    category,
                    Some(
                        raw::TypeCategory::Struct
                            | raw::TypeCategory::Enum
                            | raw::TypeCategory::FixedArray
                            | raw::TypeCategory::Vec
                    )
                ),
            };
            if !valid {
                errors
                    .push("ZRYNA-N3112", "native MIR cleanup kind disagrees with its place layout");
            }
        }
    }
}

fn type_at(program: &raw::Program, id: u32) -> Option<&raw::Type> {
    program.types.get(id as usize).filter(|ty| ty.id == id)
}
fn valid_symbol(symbol: &str) -> bool {
    let mut bytes = symbol.bytes();
    bytes.next().is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

#[derive(Default)]
struct Errors {
    items: Vec<zryna_diagnostics::Diagnostic>,
}
impl Errors {
    fn push(&mut self, code: &'static str, message: impl Into<String>) {
        if self.items.len() < MAX_DIAGNOSTICS {
            self.items.push(error(code, message));
        }
    }
}
