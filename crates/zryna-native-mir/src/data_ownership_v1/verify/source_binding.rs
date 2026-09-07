use super::super::{error, raw};
use zryna_ir::data_ownership_v1::{
    VerifiedInstruction, VerifiedInstructionKind as K, VerifiedProgram,
};
use zryna_layout::{TypeCategory, VerifiedLayouts};
use zryna_ownership_runtime_abi::VerifiedOwnershipRuntimeAbi;

pub(in crate::data_ownership_v1) fn needs_storage_plan(
    instruction: VerifiedInstruction<'_>,
    layouts: &VerifiedLayouts,
) -> bool {
    matches!(
        instruction.kind(),
        K::StructConstruct
            | K::EnumConstruct
            | K::FixedArrayConstruct
            | K::CopyFromPlace
            | K::MoveFromPlace
            | K::GenericMoveFromPlace
            | K::BorrowRead
    ) && instruction
        .result_type()
        .and_then(|ty| layouts.type_by_id(ty))
        .is_some_and(|ty| !matches!(ty.category(), TypeCategory::Bool | TypeCategory::I32))
}

fn invalid() -> Vec<zryna_diagnostics::Diagnostic> {
    vec![error(
        "ZRYNA-N3116",
        "native MIR does not preserve sealed source semantics and exact cleanup authority",
    )]
}

fn without_cleanup(mut program: raw::Program) -> raw::Program {
    for function in &mut program.functions {
        function.cleanup_plans.clear();
        for block in &mut function.blocks {
            block.cleanup = None;
            for operation in &mut block.operations {
                operation.cleanup = None;
            }
        }
    }
    program
}

pub(super) fn validate(
    claim: &raw::Program,
    source: &VerifiedProgram,
    runtime: &VerifiedOwnershipRuntimeAbi,
) -> Result<(), Vec<zryna_diagnostics::Diagnostic>> {
    // Canonical comparison authenticates all non-cleanup semantics. The action check below
    // independently reads sealed source views, rather than trusting cleanup-plan construction.
    let canonical = super::super::lower_unverified(source, runtime)?;
    if without_cleanup(claim.clone()) != without_cleanup(canonical) {
        return Err(invalid());
    }
    let functions = source
        .modules()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
        .collect::<Vec<_>>();
    if functions.len() != claim.functions.len() {
        return Err(invalid());
    }
    for (source_function, function) in functions.into_iter().zip(&claim.functions) {
        let blocks = source_function.blocks().collect::<Vec<_>>();
        if blocks.len() != function.blocks.len() {
            return Err(invalid());
        }
        for (source_block, block) in blocks.into_iter().zip(&function.blocks) {
            let instructions = source_block.instructions().collect::<Vec<_>>();
            if instructions.len() != block.operations.len() {
                return Err(invalid());
            }
            for (instruction, operation) in instructions.into_iter().zip(&block.operations) {
                let storage = needs_storage_plan(instruction, source.linux_x86_64_layouts());
                let required = storage || instruction.cleanup().is_some();
                let expected = if storage {
                    instruction.allocation_failure_drop_actions().collect::<Vec<_>>()
                } else if required {
                    instruction.derived_drop_actions().collect()
                } else {
                    Vec::new()
                };
                check(function, operation.cleanup, required, &expected)?;
            }
            let terminator = source_block.terminator();
            check(
                function,
                block.cleanup,
                terminator.cleanup().is_some(),
                &terminator.derived_drop_actions().collect::<Vec<_>>(),
            )?;
        }
    }
    Ok(())
}

fn check(
    function: &raw::Function,
    id: Option<u32>,
    required: bool,
    expected: &[zryna_ir::data_ownership_v1::VerifiedDropAction],
) -> Result<(), Vec<zryna_diagnostics::Diagnostic>> {
    use zryna_ir::data_ownership_v1::VerifiedDropActionKind as K;
    if id.is_some() != required {
        return Err(invalid());
    }
    let Some(id) = id else {
        return Ok(());
    };
    let plan = function
        .cleanup_plans
        .get(usize::try_from(id).map_err(|_| invalid())?)
        .ok_or_else(invalid)?;
    if plan.actions.len() != expected.len() {
        return Err(invalid());
    }
    for (claim, sealed) in plan.actions.iter().zip(expected) {
        let kind = match sealed.kind() {
            K::Place => raw::DropKind::Place,
            K::VecInitializedPrefix => raw::DropKind::VecPrefix,
            K::AggregateInitializedPrefix => raw::DropKind::AggregatePrefix,
            K::GenericCloneInitializedPrefix => raw::DropKind::GenericPrefix,
        };
        if claim.place != sealed.root().index() || claim.kind != kind {
            return Err(invalid());
        }
    }
    Ok(())
}
