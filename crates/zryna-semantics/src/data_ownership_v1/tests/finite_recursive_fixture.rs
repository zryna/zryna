use super::*;

pub(in crate::data_ownership_v1) const SOURCE: &str =
    include_str!("../../../../../tests/m3-fixtures/finite-recursive-composition.zry");
const RESPONSE: &str =
    include_str!("../../../../../tests/m3-fixtures/finite-recursive-composition.json");

pub(in crate::data_ownership_v1) fn snapshot() -> RawProjectSyntaxSnapshot {
    response_snapshot(RESPONSE)
}
