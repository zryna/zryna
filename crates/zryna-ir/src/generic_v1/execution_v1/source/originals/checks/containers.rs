//! Source capability follows actual opaque owners, never a substituted layout.
use super::{
    Checker, Closed, Failure, UntrustedSpan, Use, charge, copy_key, key, reserve, symbolic_fact,
};
impl Checker<'_, '_, '_> {
    pub(super) fn sharing(
        &mut self,
        index: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let value = self.expression(index, Use::Move, depth + 1)?;
        let Closed::Stored(payload) = value else {
            return Err(self.error(
                span,
                "ZRYNA-M7006",
                "Shared requires an admitted stored payload",
            ));
        };
        self.handle_type(0x22, payload)
    }
    pub(super) fn downgrading(
        &mut self,
        index: u32,
        keyword: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let value = self.expression(index, Use::Observe, depth + 1)?;
        let Closed::Stored(source) = value else {
            return Err(self.error(
                keyword,
                "ZRYNA-M7006",
                "downgrade requires an original Shared handle",
            ));
        };
        let shape = key::shape(&source, self.spent)?;
        if shape.tag != 0x22 {
            return Err(self.error(
                keyword,
                if matches!(shape.tag, 0x30 | 0x31) { "ZRYNA-M7002" } else { "ZRYNA-M7006" },
                "original source does not provide Shared handle capability",
            ));
        }
        let payload = copy_key(shape.children[0], self.spent)?;
        self.handle_type(0x23, payload)
    }
    pub(super) fn handle_type(&mut self, tag: u8, payload: Vec<u8>) -> Result<Closed, Failure> {
        charge(self.spent, 1)?;
        let mut args = reserve(1)?;
        args.push(payload);
        let result = key::encode(tag, &[], &args, self.spent)?;
        // Validity includes opaque argument identity; Shared/Weak count Clone
        // deliberately has no payload Clone dependency.
        symbolic_fact(self.resolver.originals, self.owner, &result, self.spent)?;
        Ok(result)
    }
}
