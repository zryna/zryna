//! Existing domain container preparation: failure snapshots precede operand commitment.
use super::{
    Binding, Body, Expr, Failure, Kind, UntrustedSpan, Value, charge, lookup, raw, reject, reserve,
};
impl<'s> Body<'_, 's> {
    pub(super) fn vector(
        &mut self,
        syntax: u32,
        elements: &[u32],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let ty = self.resolve(syntax)?;
        let raw::ValueType::Stored(id) = ty else {
            return Err(reject("Vec construction requires stored type"));
        };
        charge(self.spent, 2)?;
        if self.world.kind(id) != Some(Kind::Vec) {
            return Err(reject("Vec constructor has another exact type"));
        }
        let element = self
            .world
            .arguments(id)
            .and_then(|args| args.first())
            .copied()
            .ok_or(Failure::InternalFailure)?;
        charge(self.spent, elements.len())?;
        let mut prepared = reserve(elements.len())?;
        for at in elements {
            let value = self.expression(*at, depth + 1)?;
            if value.ty != raw::ValueType::Stored(element) {
                return Err(reject("Vec element differs from exact original substitution"));
            }
            prepared.push(value);
        }
        // An allocation failure still owns every completed operand. Buffer
        // acquisition happens before any element ownership is committed.
        let failure = self.failure(span)?;
        charge(self.spent, prepared.len())?;
        let mut values = reserve(prepared.len())?;
        for value in prepared {
            self.consume(value, span)?;
            values.push(value.id);
        }
        self.emit(ty, raw::Operation::VecConstruct { ty: id, elements: values, failure }, span)
    }
    pub(super) fn shared(
        &mut self,
        index: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let value = self.expression(index, depth + 1)?;
        let raw::ValueType::Stored(payload) = value.ty else {
            return Err(reject("Shared payload is not a stored source value"));
        };
        let ty = self.container_type(0x22, payload)?;
        let failure = self.failure(span)?;
        self.consume(value, span)?;
        self.emit(
            raw::ValueType::Stored(ty),
            raw::Operation::SharedConstruct { ty, value: value.id, failure },
            span,
        )
    }
    pub(super) fn downgrade(
        &mut self,
        index: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let mark = self.output.loans.len();
        let (source, temporary) = self.observed(index, span, depth)?;
        charge(self.spent, 2)?;
        if self.world.kind(source.ty) != Some(Kind::Shared) {
            return Err(reject("downgrade requires exact Shared source"));
        }
        let payload = self
            .world
            .arguments(source.ty)
            .and_then(|args| args.first())
            .copied()
            .ok_or(Failure::InternalFailure)?;
        let ty = self.container_type(0x23, payload)?;
        let failure = self.failure(span)?;
        let result = self.emit(
            raw::ValueType::Stored(ty),
            raw::Operation::WeakDowngrade { shared: source.source, failure },
            span,
        )?;
        if temporary {
            self.drop_observed(&source, span)?;
        }
        self.end_created_loans(mark, span)?;
        Ok(result)
    }
    /// A prepared temporary remains an owner during a checked handle observation,
    /// then ends immediately after success just like M3 `temporary_handle_read`.
    pub(super) fn observed(
        &mut self,
        index: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<(Binding<'s>, bool), Failure> {
        charge(self.spent, 1)?;
        if matches!(
            self.original.body.expressions[index as usize].kind,
            Expr::Reference { .. } | Expr::FieldAccess { .. } | Expr::Index { .. }
        ) {
            let binding = self.source_place_in(index, depth + 1, Some(span))?;
            self.available_target(&binding, span)?;
            return Ok((binding, false));
        }
        let value = self.expression(index, depth + 1)?;
        let raw::ValueType::Stored(ty) = value.ty else {
            return Err(reject("observation requires a stored source value"));
        };
        charge(self.spent, 1)?;
        let place = if let Some(place) = self.owners[value.id as usize] {
            place
        } else {
            let place = self.place(ty, raw::PlaceKind::Temporary(value.id), span)?;
            self.instruction(
                None,
                raw::Operation::InitializePlace { place, value: value.id },
                span,
            )?;
            self.initialize(place)?;
            place
        };
        Ok((Binding { name: "", source: raw::PlaceOrLoan::Place(place), ty, mutable: false }, true))
    }
    pub(super) fn container_type(&mut self, tag: u8, payload: u32) -> Result<u32, Failure> {
        charge(self.spent, 1)?;
        let child = &self.program.type_keys[payload as usize];
        let bytes = child.len().checked_add(5).ok_or(Failure::InternalFailure)?;
        charge(self.spent, bytes)?;
        let mut key = reserve(bytes)?;
        key.push(tag);
        key.extend_from_slice(
            &u32::try_from(child.len()).map_err(|_| Failure::InternalFailure)?.to_le_bytes(),
        );
        key.extend_from_slice(child);
        let ty = lookup(&self.program.type_keys, &key, self.spent)?;
        self.used[ty as usize] = true;
        Ok(ty)
    }
}
