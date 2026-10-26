//! Free read adapters keep both Reader lifetimes late-bound for literal/truncation checks.
use super::{Failure, Reader};
use crate::generic_v1::execution_v1::raw;
macro_rules! adapter {
    ($method:ident, $ty:ty) => {
        pub(super) fn $method(reader: &mut Reader<'_>) -> Result<$ty, Failure> {
            reader.$method()
        }
    };
}
adapter!(access, raw::Access);
adapter!(call_argument, raw::CallArgument);
adapter!(cleanup_action, raw::CleanupAction);
adapter!(cleanup_plan, raw::CleanupPlan);
adapter!(enum_arm, raw::EnumArm);
adapter!(loan, raw::Loan);
adapter!(loan_origin, raw::LoanOrigin);
adapter!(match_binding, raw::MatchBinding);
adapter!(match_mode, raw::MatchMode);
adapter!(operation, raw::Operation);
adapter!(parameter, raw::Parameter);
adapter!(place, raw::Place);
adapter!(place_kind, raw::PlaceKind);
adapter!(place_or_loan, raw::PlaceOrLoan);
adapter!(region, raw::Region);
adapter!(region_role, raw::RegionRole);
adapter!(static_projection, raw::StaticProjection);
adapter!(terminator, raw::Terminator);
adapter!(trap_identity, raw::TrapIdentity);
adapter!(ty, raw::ValueType);
