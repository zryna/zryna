//! Every nested raw sum type is pinned independently, including nonstored loans.
use crate::generic_v1::execution_v1::raw::*;
use zryna_source::UntrustedSpan;
#[test]
fn signature_place_and_projection_tags_are_literal() {
    check_copy!(ty, ValueType::Stored(1), "0001000000");
    check_copy!(ty, ValueType::Unit, "01");
    check_copy!(access, Access::Shared, "00");
    check_copy!(access, Access::Exclusive, "01");
    check!(
        parameter,
        Parameter::Value(ValueDefinition { id: 1, ty: ValueType::Stored(2) }),
        "00010000000002000000"
    );
    check!(
        parameter,
        Parameter::Loan(LoanDefinition { id: 1, referent: 2, access: Access::Exclusive }),
        "01010000000200000001"
    );
    check!(place_kind, PlaceKind::Parameter(1), "0001000000");
    check!(place_kind, PlaceKind::Local(1), "0101000000");
    check!(place_kind, PlaceKind::Temporary(1), "0201000000");
    check!(place_kind, PlaceKind::StructField { base: 1, ordinal: 2 }, "030100000002000000");
    check!(place_kind, PlaceKind::EnumPayload { base: 1, variant: 2 }, "040100000002000000");
    check!(place_kind, PlaceKind::FixedArrayConstant { base: 1, index: 2 }, "050100000002000000");
    check_copy!(static_projection, StaticProjection::StructField(1), "0001000000");
    check_copy!(static_projection, StaticProjection::EnumPayload(1), "0101000000");
    check_copy!(static_projection, StaticProjection::FixedArrayConstant(1), "0201000000");
    check_copy!(place_or_loan, PlaceOrLoan::Place(1), "0001000000");
    check_copy!(place_or_loan, PlaceOrLoan::Loan(1), "0101000000");
    check_copy!(call_argument, CallArgument::Value(1), "0001000000");
    check_copy!(call_argument, CallArgument::Loan(1), "0101000000");
}
#[test]
fn all_origin_and_region_roles_are_literal() {
    check!(loan_origin, LoanOrigin::Formal(1), "0001000000");
    check!(loan_origin, LoanOrigin::Place(1), "0101000000");
    check!(loan_origin, LoanOrigin::InternalReborrow(1), "0201000000");
    check!(
        loan_origin,
        LoanOrigin::StaticProjection { parent: 1, projection: StaticProjection::StructField(2) },
        "03010000000002000000"
    );
    check!(
        loan_origin,
        LoanOrigin::Indexed { base: PlaceOrLoan::Loan(1), index: 2 },
        "04010100000002000000"
    );
    check_copy!(region_role, RegionRole::Lexical, "00");
    check_copy!(region_role, RegionRole::SelectedMatchArm, "01");
    check_copy!(region_role, RegionRole::TransientIndexedAccess, "02");
    check!(
        region,
        Region {
            span: UntrustedSpan { file: 1, start: 2, end: 3 },
            role: RegionRole::SelectedMatchArm
        },
        "01000000020000000300000001"
    );
    check!(
        place,
        Place {
            id: 1,
            ty: 2,
            span: UntrustedSpan { file: 3, start: 4, end: 5 },
            kind: PlaceKind::Local(6)
        },
        "01000000020000000300000004000000050000000106000000"
    );
    check!(
        loan,
        Loan {
            id: 1,
            referent: 2,
            access: Access::Shared,
            region: Region {
                span: UntrustedSpan { file: 3, start: 4, end: 5 },
                role: RegionRole::Lexical
            },
            origin: LoanOrigin::Formal(6)
        },
        "010000000200000000030000000400000005000000000006000000"
    );
}
