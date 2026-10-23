//! Immutable Copy source statement order, bindings and exact returns.

use super::{Builder, Closed, Failure, RawStatementKind, raw, reject};

impl Builder<'_, '_> {
    pub(super) fn source_block(&mut self, index: u32, depth: usize) -> Result<(), Failure> {
        if depth > 128 {
            return Err(super::super::budget("source replay nesting exceeds 128"));
        }
        let source = &self.original.body.blocks[index as usize];
        let mut returned = false;
        for id in &source.statements {
            if returned {
                return Err(reject("Copy executable lane does not admit statements after return"));
            }
            let statement = &self.original.body.statements[*id as usize];
            match &statement.kind {
                RawStatementKind::LocalDeclaration {
                    name,
                    type_syntax,
                    initializer,
                    mutable,
                    ..
                } => {
                    if *mutable {
                        return Err(reject(
                            "mutable source state requires successor ownership/CFG replay",
                        ));
                    }
                    let value = self.expression(*initializer, depth + 1)?;
                    if value.ty != self.resolve(*type_syntax)? {
                        return Err(reject(
                            "source initializer differs from its declared symbolic type",
                        ));
                    }
                    self.bind(&name.text, value)?;
                }
                RawStatementKind::Return { value, .. } => {
                    let value = self.expression(*value, depth + 1)?;
                    if value.ty != self.result {
                        return Err(reject("source return differs from original symbolic result"));
                    }
                    self.blocks[self.block].span = statement.span;
                    self.blocks[self.block].terminator = raw::Terminator::Return(value.id);
                    returned = true;
                }
                RawStatementKind::ExpressionStatement { expression, .. } => {
                    self.expression(*expression, depth + 1)?;
                }
                _ => {
                    return Err(reject(
                        "source statement requires a successor lane beyond immutable Copy replay",
                    ));
                }
            }
        }
        if !returned {
            if self.result != Closed::Unit {
                return Err(reject("source function lacks an exact return"));
            }
            let value = self.emit(Closed::Unit, source.span, raw::Operation::Unit)?;
            self.blocks[self.block].span = source.span;
            self.blocks[self.block].terminator = raw::Terminator::Return(value.id);
        }
        Ok(())
    }
}
