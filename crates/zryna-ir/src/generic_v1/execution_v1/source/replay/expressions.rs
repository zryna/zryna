use super::{
    Binding, Body, Expr, Failure, Kind, UntrustedSpan, Value, charge, copy_key, exhausted, lookup,
    raw, reject, reserve,
};
mod scalars;
impl<'s> Body<'_, 's> {
    pub(super) fn expression(&mut self, index: u32, depth: usize) -> Result<Value, Failure> {
        // The syntax owner counts root block=1 and each child AST edge once.
        if depth
            > usize::try_from(zryna_syntax::v4::MAX_NESTING_DEPTH)
                .map_err(|_| Failure::InternalFailure)?
        {
            return Err(exhausted("source expression depth exceeds 128"));
        }
        charge(self.spent, 1)?;
        let source = &self.original.body.expressions[index as usize];
        let span = source.span;
        match &source.kind {
            Expr::Reference { .. } | Expr::FieldAccess { .. } => {
                let mark = self.output.loans.len();
                let b = self.source_place(index, depth)?;
                let value = self.read_source(&b, span)?;
                self.end_created_loans(mark, span)?;
                Ok(value)
            }
            Expr::BoolLiteral { value } => {
                let ty = self.primitive(0)?;
                self.emit(ty, raw::Operation::BoolLiteral(*value), span)
            }
            Expr::I32Literal { spelling } => {
                charge(self.spent, spelling.len())?;
                let n = spelling.parse().map_err(|_| reject("inexact source i32 literal"))?;
                let ty = self.primitive(1)?;
                self.emit(ty, raw::Operation::I32Literal(n), span)
            }
            Expr::StringLiteral { spelling } => self.string_literal(spelling, span),
            Expr::Negation { operand, .. } => self.scalar_negation(*operand, span, depth),
            Expr::Addition { lhs, rhs, .. }
            | Expr::Subtraction { lhs, rhs, .. }
            | Expr::Multiplication { lhs, rhs, .. }
            | Expr::Equal { lhs, rhs, .. }
            | Expr::NotEqual { lhs, rhs, .. }
            | Expr::LessThan { lhs, rhs, .. }
            | Expr::LessEqual { lhs, rhs, .. }
            | Expr::GreaterThan { lhs, rhs, .. }
            | Expr::GreaterEqual { lhs, rhs, .. } => {
                let kind = scalars::binary_kind(&source.kind)
                    .ok_or_else(|| reject("nonscalar source passed to scalar replay"))?;
                self.scalar_binary(*lhs, *rhs, kind, span, depth)
            }
            Expr::Clone { value, keyword_span, .. } => {
                self.clone_expression(*value, span, *keyword_span, depth)
            }
            Expr::Call { callee, type_arguments, arguments, .. } => {
                self.call(callee, type_arguments.as_ref(), arguments, span, depth)
            }
            Expr::EnumConstruction { type_name, variant, type_arguments, payload, .. } => self
                .construct_enum(type_name, variant, type_arguments.as_ref(), *payload, span, depth),
            Expr::StructConstruction { type_name, type_arguments, fields, .. } => {
                self.construct_struct(type_name, type_arguments.as_ref(), fields, span, depth)
            }
            Expr::FixedArrayConstruction { type_syntax, elements, .. } => {
                self.array(*type_syntax, elements, span, depth)
            }
            Expr::VecConstruction { type_syntax, elements, .. } => {
                self.vector(*type_syntax, elements, span, depth)
            }
            Expr::Shared { value, .. } => self.shared(*value, span, depth),
            Expr::Downgrade { value, .. } => self.downgrade(*value, span, depth),
            Expr::VecPush { vector, value, keyword_span, .. } => {
                self.push_vector(*vector, *value, span, *keyword_span, depth)
            }
            Expr::Index { base, index, .. } => self.index_copy(*base, *index, span, depth),
            Expr::Match { scrutinee, arms, .. } => {
                self.match_expression(*scrutinee, arms, span, depth)
            }
            _ => {
                Err(reject("source operation remains an open implementation gate in execution_v1"))
            }
        }
    }
    fn string_literal(&mut self, spelling: &str, span: UntrustedSpan) -> Result<Value, Failure> {
        charge(self.spent, spelling.len())?;
        let text =
            spelling.get(1..spelling.len() - 1).ok_or_else(|| reject("invalid source string"))?;
        if text.contains('\\') {
            return Err(reject("escaped string source replay is an open full-profile gate"));
        }
        if text.len() > 65_536 {
            return Err(exhausted("String literal exceeds inherited limit"));
        }
        let bytes = copy_key(text.as_bytes(), self.spent)?;
        let failure = self.failure(span)?;
        let ty = self.primitive(2)?;
        self.emit(ty, raw::Operation::StringFromUtf8 { bytes, failure }, span)
    }
    fn index_copy(
        &mut self,
        base: u32,
        index: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let mark = self.output.loans.len();
        let b = self.source_place_in(base, depth + 1, Some(span))?;
        if let Some(child) = self.static_index_binding(&b, index, span)? {
            let value = self.read_source(&child, span)?;
            self.end_created_loans(mark, span)?;
            return Ok(value);
        }
        let i = self.expression(index, depth + 1)?;
        if i.ty != self.primitive(1)? {
            return Err(reject("array index requires exact i32"));
        }
        charge(self.spent, 2)?;
        if !matches!(self.world.kind(b.ty), Some(Kind::FixedArray { .. } | Kind::Vec)) {
            return Err(reject("indexed container replay remains open"));
        }
        let element =
            *self.world.arguments(b.ty).and_then(|a| a.first()).ok_or(Failure::InternalFailure)?;
        if !self.copy(element)? {
            return Err(reject("indexed Copy observation requires original Copy"));
        }
        let failure = self.failure(span)?;
        let value = self.emit(
            raw::ValueType::Stored(element),
            raw::Operation::IndexCopy { base: b.source, index: i.id, failure },
            span,
        )?;
        self.end_created_loans(mark, span)?;
        Ok(value)
    }
    pub(super) fn primitive(&mut self, tag: u8) -> Result<raw::ValueType, Failure> {
        let id = lookup(&self.program.type_keys, &[tag], self.spent)?;
        self.used[id as usize] = true;
        Ok(raw::ValueType::Stored(id))
    }
    pub(super) fn reference(&mut self, index: u32) -> Result<Binding<'s>, Failure> {
        charge(self.spent, 1)?;
        let e = &self.original.body.expressions[index as usize];
        let Expr::Reference { name } = &e.kind else {
            return Err(reject("operation requires an exact lexical source place"));
        };
        self.named(&name.text, e.span)
    }
    fn clone_expression(
        &mut self,
        index: u32,
        span: UntrustedSpan,
        keyword: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let mark = self.output.loans.len();
        let (b, temporary) = self.observed(index, span, depth)?;
        charge(self.spent, 1)?;
        let fact = self.world.fact(b.ty).ok_or(Failure::InternalFailure)?;
        if !fact.clone {
            return Err(self.error(
                keyword,
                "ZRYNA-M7002",
                "original source type does not have Clone",
            ));
        }
        let d = self.definition(raw::ValueType::Stored(b.ty), true, span)?;
        let value = Value { id: d.id, ty: d.ty };
        let (outer_failure, prefix_failure) = if fact.copy {
            (None, None)
        } else {
            let actions = self.actions()?;
            charge(self.spent, actions.len() + 1)?;
            let mut prefix = reserve(actions.len() + 1)?;
            let destination = self.owners[d.id as usize].ok_or(Failure::InternalFailure)?;
            prefix.push(raw::CleanupAction::DropInitializedPrefix {
                destination,
                preparation_site: raw::PreparationSite {
                    function: u32::try_from(self.function).map_err(|_| Failure::InternalFailure)?,
                    block: u32::try_from(self.block).map_err(|_| Failure::InternalFailure)?,
                    instruction: u32::try_from(self.output.blocks[self.block].instructions.len())
                        .map_err(|_| Failure::InternalFailure)?,
                },
            });
            prefix.extend(actions.iter().cloned());
            (Some(self.plan(actions, span)?), Some(self.plan(prefix, span)?))
        };
        self.instruction(
            Some(d),
            raw::Operation::Clone { source: b.source, outer_failure, prefix_failure },
            span,
        )?;
        self.activate(value.id)?;
        if temporary {
            self.drop_observed(&b, span)?;
        }
        self.end_created_loans(mark, span)?;
        Ok(value)
    }
}
