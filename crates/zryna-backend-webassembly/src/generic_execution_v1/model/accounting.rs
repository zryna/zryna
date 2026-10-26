//! Private source-bound leaf effects across all closed layouts; inline traversal adds no debit.
use super::{
    Diagnostic, Model, TypeCategory, TypeView, VerifiedExecution, VerifiedFunction, budget,
    invariant, work,
};
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, VerifiedParameter, raw};
pub(super) struct Plan<'p, 'a> {
    owner: &'p VerifiedExecution<'a>,
    vector: bool,
    control: bool,
}
impl<'p, 'a> Plan<'p, 'a> {
    pub(super) fn derive(
        program: &'p VerifiedExecution<'a>,
        types: &[TypeView<'a>],
        functions: &[VerifiedFunction<'p, 'a>],
        observed: bool,
        spent: &mut usize,
    ) -> Result<Option<Self>, Diagnostic> {
        if observed {
            return Ok(None);
        }
        work::take(spent, types.len())?;
        let mut vector = false;
        let mut control = false;
        for ty in types {
            match ty.category() {
                TypeCategory::Vec => vector = true,
                TypeCategory::Shared | TypeCategory::Weak => control = true,
                TypeCategory::Bool
                | TypeCategory::I32
                | TypeCategory::String
                | TypeCategory::Struct
                | TypeCategory::Enum
                | TypeCategory::FixedArray => {}
            }
        }
        work::take(spent, functions.len())?;
        let mut resource = false;
        for function in functions {
            work::take(spent, function.blocks().len())?;
            for block in function.blocks() {
                work::take(spent, block.operations().len())?;
                for operation in block.operations() {
                    if let Some(clone) = operation.clone_frontier() {
                        match clone.frontier().kind() {
                            CloneNodeKind::String
                            | CloneNodeKind::VecEach
                            | CloneNodeKind::SharedCount
                            | CloneNodeKind::WeakCount
                            | CloneNodeKind::Enum
                            | CloneNodeKind::Struct
                            | CloneNodeKind::FixedArray { .. } => resource = true,
                            CloneNodeKind::Copy => {}
                        }
                    }
                    if matches!(
                        operation.kind(),
                        raw::Operation::StringFromUtf8 { .. }
                            | raw::Operation::StringConcat { .. }
                            | raw::Operation::VecConstruct { .. }
                            | raw::Operation::VecPush { .. }
                            | raw::Operation::SharedConstruct { .. }
                            | raw::Operation::WeakDowngrade { .. }
                    ) || block.weak_upgrade().is_some()
                    {
                        resource = true;
                    }
                    if !matches!(
                        operation.kind(),
                        raw::Operation::Unit
                            | raw::Operation::BoolLiteral(_)
                            | raw::Operation::I32Literal(_)
                            | raw::Operation::ScalarBinary { .. }
                            | raw::Operation::I32Neg { .. }
                            | raw::Operation::ReadPlace { .. }
                            | raw::Operation::ReadLoan { .. }
                            | raw::Operation::InitializePlace { .. }
                            | raw::Operation::ReplacePlace { .. }
                            | raw::Operation::ReplaceLoan { .. }
                            | raw::Operation::DropPlace { .. }
                            | raw::Operation::Clone { .. }
                            | raw::Operation::BeginLoan { .. }
                            | raw::Operation::EndLoan { .. }
                            | raw::Operation::SourceCall { .. }
                            | raw::Operation::ClosedGenericCall { .. }
                            | raw::Operation::StringFromUtf8 { .. }
                            | raw::Operation::StringConcat { .. }
                            | raw::Operation::VecConstruct { .. }
                            | raw::Operation::VecPush { .. }
                            | raw::Operation::IndexCopy { .. }
                            | raw::Operation::StructConstruct { .. }
                            | raw::Operation::ClosedEnumConstruct { .. }
                            | raw::Operation::FixedArrayConstruct { .. }
                            | raw::Operation::SharedConstruct { .. }
                            | raw::Operation::WeakDowngrade { .. }
                    ) {
                        return Ok(None);
                    }
                }
            }
        }
        if !resource {
            return Ok(None);
        }
        work::take(spent, program.export_functions().len())?;
        for &entry in program.export_functions() {
            let function = functions.get(entry).ok_or_else(invariant)?;
            work::take(spent, function.parameters().len().checked_add(1).ok_or_else(budget)?)?;
            for parameter in function.parameters() {
                let VerifiedParameter::Value(value) = parameter else { return Err(invariant()) };
                scalar(types, value.ty())?;
            }
            scalar(types, function.result_type())?;
        }
        Ok(Some(Self { owner: program, vector, control }))
    }
}
fn scalar(types: &[TypeView<'_>], ty: raw::ValueType) -> Result<(), Diagnostic> {
    let raw::ValueType::Stored(ty) = ty else { return Err(invariant()) };
    if matches!(
        types.get(ty as usize).ok_or_else(invariant)?.category(),
        TypeCategory::Bool | TypeCategory::I32
    ) {
        Ok(())
    } else {
        Err(invariant())
    }
}
impl Plan<'_, '_> {
    pub(super) fn count(&self, concat: bool, continuation: bool) -> u32 {
        3 + u32::from(concat)
            + (if self.vector { 2 + u32::from(continuation) } else { 0 })
            + (if self.control { 7 + u32::from(continuation) } else { 0 })
    }
}
impl Model<'_, '_, '_> {
    pub(in crate::generic_execution_v1) fn accounted(&self) -> bool {
        self.accounting.as_ref().is_some_and(|plan| std::ptr::eq(plan.owner, self.program))
    }
    pub(in crate::generic_execution_v1) fn accounting_base(&self) -> u32 {
        self.container_base + self.container_count + if self.concat { 2 } else { 0 }
    }
    pub(in crate::generic_execution_v1) fn has_accounted_vec(&self) -> bool {
        self.accounted() && self.accounting.as_ref().is_some_and(|plan| plan.vector)
    }
    pub(in crate::generic_execution_v1) fn has_accounted_control(&self) -> bool {
        self.accounted() && self.accounting.as_ref().is_some_and(|plan| plan.control)
    }
    pub(in crate::generic_execution_v1) fn control_base(&self) -> u32 {
        self.vector_base()
            + if self.has_accounted_vec() { 2 + u32::from(self.continuation) } else { 0 }
    }
    pub(in crate::generic_execution_v1) fn control_birth(&self) -> u32 {
        if self.has_accounted_control() { self.control_base() } else { self.container_base }
    }
    pub(in crate::generic_execution_v1) fn accounting_count(&self) -> u32 {
        if self.accounted() {
            self.accounting.as_ref().map_or(0, |plan| plan.count(self.concat, self.continuation))
        } else {
            0
        }
    }
    pub(in crate::generic_execution_v1) fn vector_base(&self) -> u32 {
        self.accounting_base() + 3 + u32::from(self.concat)
    }
    pub(in crate::generic_execution_v1) fn global_count(&self) -> u32 {
        (if self.accounted() { 13 } else { 10 }) + u32::from(self.recursive_count() != 0)
    }
    pub(in crate::generic_execution_v1) fn string_call(&self, raw: u32) -> u32 {
        if !self.accounted() {
            return raw;
        }
        self.accounting_base()
            + if raw == super::STRING_FROM {
                0
            } else if raw == super::STRING_CLONE {
                1
            } else if raw == super::STRING_RELEASE {
                2
            } else {
                debug_assert!(self.concat && raw == self.concat_index());
                3
            }
    }
}

#[cfg(test)]
mod tests;
