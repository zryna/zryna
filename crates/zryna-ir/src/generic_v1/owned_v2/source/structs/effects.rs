//! Whole-constructor commit and nonconsuming Copy observation retain exact root loans.
use super::super::{Builder, owners::ownership};
use crate::generic_v1::Failure;
use std::collections::BTreeSet;

impl Builder<'_, '_> {
    pub(in super::super) fn consume_struct_fields(
        &mut self,
        fields: &[u32],
    ) -> Result<(), Failure> {
        if !self.type_facts.admission().has_structs() {
            return Err(ownership("Struct ownership requires the distinct private v4 issuer"));
        }
        let mut owners = BTreeSet::new();
        // Validate the entire commit before changing even the first field's availability.
        for id in fields {
            if !self.alive.get(*id as usize).copied().unwrap_or(false) {
                return Err(ownership("Struct constructor uses an unavailable original field"));
            }
            if self.affine[*id as usize]
                && (!owners.insert(*id) || self.loans.values().any(|(root, _)| root == id))
            {
                return Err(ownership("Struct constructor duplicates or moves a loaned field"));
            }
        }
        for id in owners {
            self.consume(id)?;
        }
        Ok(())
    }

    pub(in super::super) fn observe_struct(&self, value: u32) -> Result<(), Failure> {
        if !self.type_facts.admission().has_structs()
            || !self.alive.get(value as usize).copied().unwrap_or(false)
            || self.loans.values().any(|(root, exclusive)| *root == value && *exclusive)
        {
            return Err(ownership(
                "Copy field observation requires a live complete unblocked Struct",
            ));
        }
        Ok(())
    }
}
