use zryna_ir::data_ownership_v1::raw;

use super::super::Ty;
use super::PrivateOwnedAggregateLowerer;
use super::constructor_preparation::PreparedValue;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn value(&mut self, id: u32, expected: Ty) -> Option<raw::ValueId> {
        Some(PreparedValue::prepare(self, id, expected)?.consume())
    }
}
