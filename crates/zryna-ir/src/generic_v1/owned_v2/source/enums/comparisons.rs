//! Match selection keeps the complete legacy description debit before duplicate rejection.
use super::super::facts::{Facts, comparisons::select};
use super::{Builder, Closed, RawMatchArm, describe};
use crate::generic_v1::{Failure, reject};

impl<'b> Builder<'_, 'b> {
    pub(super) fn match_arm(
        &mut self,
        arms: &'b [RawMatchArm],
        name: &str,
        arguments: &[Vec<u8>],
        expected: &Closed,
    ) -> Result<&'b RawMatchArm, Failure> {
        if let Facts::Legacy(facts) = &mut self.type_facts {
            let mut selected = arms.iter().filter(|arm| arm.variant.text == name);
            let arm = selected.next().ok_or_else(|| reject("missing enum ordinal"))?;
            // This was unconditional in the original source replay, even for duplicates.
            facts.description(&self.resolver, &arm.type_name.text, arguments, &mut self.budget)?;
            if selected.next().is_some()
                || describe(&self.resolver, &arm.type_name.text, arguments, self.symbolic)?.ty
                    != *expected
            {
                return Err(reject("duplicate/foreign enum match arm"));
            }
            return Ok(arm);
        }
        let ordinal = select(arms, name, true, |arm| arm.variant.text.as_str(), &mut self.budget)?
            .ok_or_else(|| reject("missing enum ordinal"))?;
        let arm = &arms[ordinal];
        let actual = self.type_facts.enum_description(
            &self.resolver,
            &arm.type_name.text,
            arguments,
            self.symbolic,
            &mut self.budget,
        )?;
        if !self.type_facts.equal_type(&actual.ty, expected, &mut self.budget)? {
            return Err(reject("duplicate/foreign enum match arm"));
        }
        Ok(arm)
    }
}
