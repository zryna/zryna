//! Affine source Builder values, blocks, diagnostics and charged instruction emission.

use super::{Binding, Builder, Closed, Failure, Value, owners, raw, reject, type_id};

impl<'b> Builder<'_, 'b> {
    pub(super) fn locate(&self, failure: Failure, at: zryna_source::UntrustedSpan) -> Failure {
        let Failure::Diagnostics(mut errors) = failure else {
            return failure;
        };
        for error in &mut errors {
            if error.code == "ZRYNA-M7007"
                && error.primary_span().is_none()
                && let Ok(span) = self.sources.verify_span(at)
            {
                *error = zryna_diagnostics::Diagnostic::error_at(
                    &error.code,
                    span,
                    &error.message,
                    &error.guidance,
                );
            }
        }
        Failure::Diagnostics(errors)
    }
    pub(super) fn resolve(&self, occurrence: u32) -> Result<Closed, Failure> {
        if self.symbolic {
            self.resolver.resolve_symbolic(occurrence)
        } else {
            self.resolver.resolve(occurrence)
        }
    }

    pub(super) fn bind(&mut self, name: &'b str, value: Value) -> Result<(), Failure> {
        if self.locals[self.scope_start..].iter().any(|prior| prior.name.eq_ignore_ascii_case(name))
        {
            return Err(reject("source value bindings collide under portable folding"));
        }
        self.locals.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.locals.push(Binding { name, value, mutable: false });
        Ok(())
    }

    pub(super) fn value(&mut self, ty: Closed) -> Result<Value, Failure> {
        owners::admitted(&ty)?;
        if self.next as usize >= crate::data_ownership_v1::MAX_VALUES_PER_FUNCTION {
            return Err(crate::generic_v1::budget("source replay exceeds inherited value ceiling"));
        }
        let id = self.next;
        self.budget.value()?;
        self.next = self.next.checked_add(1).ok_or(Failure::InternalFailure)?;
        let affine = owners::affine(&ty)?;
        self.affine.push(affine);
        self.alive.push(true);
        if let Closed::Borrow(_, exclusive) = &ty {
            self.loans.insert(id, (id, *exclusive));
        }
        Ok(Value { id, ty })
    }

    pub(super) fn definition(&self, value: &Value) -> Result<raw::Definition, Failure> {
        Ok(raw::Definition {
            id: value.id,
            ty: if self.symbolic {
                raw::Type::Unit
            } else {
                type_id(self.program, value.ty.clone())?
            },
        })
    }

    pub(super) fn new_block(
        &mut self,
        span: zryna_source::UntrustedSpan,
    ) -> Result<usize, Failure> {
        if self.blocks.len() >= crate::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION {
            return Err(crate::generic_v1::budget("source replay exceeds inherited block ceiling"));
        }
        self.budget.block()?;
        self.blocks.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        let index = self.blocks.len();
        self.blocks.push(raw::Block {
            id: u32::try_from(index).map_err(|_| Failure::InternalFailure)?,
            parameters: Vec::new(),
            instructions: Vec::new(),
            span,
            terminator: raw::Terminator::Return(u32::MAX),
        });
        Ok(index)
    }

    pub(super) fn emit(
        &mut self,
        ty: Closed,
        span: zryna_source::UntrustedSpan,
        operation: raw::Operation,
    ) -> Result<Value, Failure> {
        owners::core(self, &operation)?;
        self.budget.operation(&operation)?;
        let value = self.value(ty)?;
        let result = self.definition(&value)?;
        let instructions = &mut self.blocks[self.block].instructions;
        instructions.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        instructions.push(raw::Instruction { result, span, operation });
        Ok(value)
    }
}
