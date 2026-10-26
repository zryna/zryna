//! Original mutation permission and success-only exact Weak/Shared binding.
use super::{Checker, Closed, Failure, Statement, UntrustedSpan, Use, copy_key, key, reject};
impl<'s> Checker<'_, 's, '_> {
    pub(super) fn pushing(
        &mut self,
        vector: u32,
        value: u32,
        keyword: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let place = self.place(vector, depth + 1)?;
        let ty = super::clone_type(&place.ty, self.spent)?;
        let Closed::Stored(bytes) = &ty else {
            return Err(reject("original exclusive-loan append depends on loan binding replay"));
        };
        let shape = key::shape(bytes, self.spent)?;
        if shape.tag != 0x21 {
            return Err(self.error(
                keyword,
                if matches!(shape.tag, 0x30 | 0x31) { "ZRYNA-M7002" } else { "ZRYNA-M7006" },
                "push requires an exact original Vec capability",
            ));
        }
        let element = Closed::Stored(copy_key(shape.children[0], self.spent)?);
        self.access(&place, true, false, keyword)?;
        if place.indexed {
            return self.push_indexed(place, value, &element, keyword, depth);
        }
        let rhs = self.expression(value, Use::Move, depth + 1)?;
        self.expect(&rhs, &element, self.original.body.expressions[value as usize].span)?;
        // Recheck the prepared target without reevaluating its index.
        self.access(&place, true, false, keyword)?;
        Ok(Closed::Unit)
    }
    pub(super) fn upgrade(
        &mut self,
        s: &'s zryna_syntax::v4::RawStatementSyntax,
        depth: usize,
    ) -> Result<bool, Failure> {
        let Statement::WeakUpgrade {
            weak,
            binding,
            success_block,
            failure_block,
            keyword_span,
            ..
        } = &s.kind
        else {
            return Err(Failure::InternalFailure);
        };
        let weak = self.expression(*weak, Use::Observe, depth + 1)?;
        let Closed::Stored(bytes) = weak else {
            return Err(reject("original loan Weak observation depends on loan binding replay"));
        };
        let shape = key::shape(&bytes, self.spent)?;
        if shape.tag != 0x23 {
            return Err(self.error(
                *keyword_span,
                if matches!(shape.tag, 0x30 | 0x31) { "ZRYNA-M7002" } else { "ZRYNA-M7006" },
                "upgrade requires exact original Weak capability",
            ));
        }
        let payload = copy_key(shape.children[0], self.spent)?;
        let shared = self.handle_type(0x22, payload)?;
        let before = self.snapshot()?;
        self.bind(&binding.text, shared)?;
        let yes_returns = self.block(*success_block, depth + 1)?;
        self.locals.truncate(before.len());
        let yes = self.snapshot()?;
        self.restore(&before)?;
        let no_returns = self.block(*failure_block, depth + 1)?;
        let no = self.snapshot()?;
        self.restore(&before)?;
        if !yes_returns && !no_returns {
            self.same_original(&yes, &no, s.span)?;
        }
        self.restore(if yes_returns { &no } else { &yes })?;
        Ok(yes_returns && no_returns)
    }
}
