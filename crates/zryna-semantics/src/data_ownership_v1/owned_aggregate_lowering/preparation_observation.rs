use super::super::indexed_vec_preparation::IndexedObservation;
use super::{PreparationContext, Ty};

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn prepared_observation(
        &mut self,
        id: u32,
        expected: Option<Ty>,
    ) -> Option<IndexedObservation> {
        if self.state.summary {
            if let IndexedObservation::Value(value) = self.lexical_alias_read(id, expected)? {
                return Some(IndexedObservation::Value(value));
            }
            if let IndexedObservation::Value(value) = self.indexed_read(id, expected)? {
                return Some(IndexedObservation::Value(value));
            }
        }
        Some(IndexedObservation::Unselected)
    }
}
