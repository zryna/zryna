use super::*;

pub(in crate::data_ownership_v1) const SOURCE: &str =
    include_str!("../../../../../tests/m3-fixtures/finite-recursive-composition.zry");
const RESPONSE: &str =
    include_str!("../../../../../tests/m3-fixtures/finite-recursive-composition.json");

pub(in crate::data_ownership_v1) fn snapshot() -> RawProjectSyntaxSnapshot {
    response_snapshot(RESPONSE)
}

pub(in crate::data_ownership_v1) const PROJECTION_SOURCE: &str =
    include_str!("../../../../../tests/m3-fixtures/finite-recursive-projections.zry");
const PROJECTION_RESPONSE: &str =
    include_str!("../../../../../tests/m3-fixtures/finite-recursive-projections.json");

pub(in crate::data_ownership_v1) fn projection_snapshot() -> RawProjectSyntaxSnapshot {
    response_snapshot(PROJECTION_RESPONSE)
}
