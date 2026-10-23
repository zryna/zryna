//! Original mutation permission and success-only exact Weak/Shared binding.
use super::{
    Checker, Closed, Failure, Statement, UntrustedSpan, Use, charge, copy_key, key, reject,
};
impl<'s> Checker<'_, 's, '_> {
    pub(super) fn pushing(
        &mut self,
        vector: u32,
        value: u32,
        keyword: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let ty = self.place(vector, depth + 1)?.ty;
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
        self.mutable_root(vector, keyword)?;
        let rhs = self.expression(value, Use::Move, depth + 1)?;
        self.expect(&rhs, &element, self.original.body.expressions[value as usize].span)?;
        // Target expressions are static place paths, so this observes availability,
        // without repeating an effectful RHS or producing a second owner.
        self.mutable_root(vector, keyword)?;
        Ok(Closed::Unit)
    }
    fn mutable_root(&mut self, at: u32, span: UntrustedSpan) -> Result<(), Failure> {
        let place = self.place(at, 1)?;
        self.access(&place, true, false, span)
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
        charge(self.spent, before.len())?;
        for index in 0..before.len() {
            self.locals[index].available = if yes_returns {
                no[index]
            } else if no_returns {
                yes[index]
            } else {
                yes[index] && no[index]
            };
        }
        Ok(yes_returns && no_returns)
    }
}
