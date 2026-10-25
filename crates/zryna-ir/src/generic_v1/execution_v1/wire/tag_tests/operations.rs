//! Frozen hex opcodes, little-endian operands and option/vector shapes.
use crate::generic_v1::execution_v1::raw::*;
#[test]
fn all_operation_tags_have_independent_literal_bytes() {
    check!(operation, Operation::Unit, "00");
    check!(operation, Operation::BoolLiteral(false), "0100");
    check!(operation, Operation::BoolLiteral(true), "0101");
    check!(operation, Operation::I32Literal(-2), "02feffffff");
    check!(operation, Operation::I32Neg { value: 1 }, "0401000000");
}
#[test]
fn place_operations_have_independent_literal_bytes() {
    check!(operation, Operation::ReadPlace { place: 1, mode: ReadMode::Copy }, "100100000000");
    check!(operation, Operation::ReadPlace { place: 1, mode: ReadMode::Move }, "100100000001");
    check!(operation, Operation::InitializePlace { place: 1, value: 2 }, "110100000002000000");
    check!(operation, Operation::ReplacePlace { place: 1, value: 2 }, "120100000002000000");
    check!(operation, Operation::DropPlace { place: 1 }, "1301000000");
    check!(
        operation,
        Operation::Clone {
            source: PlaceOrLoan::Place(1),
            outer_failure: None,
            prefix_failure: None
        },
        "1400010000000000"
    );
    check!(
        operation,
        Operation::Clone {
            source: PlaceOrLoan::Loan(1),
            outer_failure: Some(2),
            prefix_failure: Some(3)
        },
        "14010100000001020000000103000000"
    );
}
#[test]
fn calls_and_constructors_have_independent_literal_bytes() {
    check!(
        operation,
        Operation::ClosedGenericCall {
            instance: 1,
            arguments: vec![CallArgument::Value(2), CallArgument::Loan(3)],
            failure: Some(4)
        },
        "200100000002000000000200000001030000000104000000"
    );
    check!(
        operation,
        Operation::SourceCall { module: 1, function: 2, arguments: vec![], failure: None },
        "2101000000020000000000000000"
    );
    check!(
        operation,
        Operation::StructConstruct { ty: 1, fields: vec![2, 3] },
        "3001000000020000000200000003000000"
    );
    check!(
        operation,
        Operation::ClosedEnumConstruct { ty: 1, ordinal: 2, payload: None },
        "31010000000200000000"
    );
    check!(
        operation,
        Operation::ClosedEnumConstruct { ty: 1, ordinal: 2, payload: Some(3) },
        "3101000000020000000103000000"
    );
    check!(
        operation,
        Operation::FixedArrayConstruct { ty: 1, elements: vec![2, 3] },
        "3201000000020000000200000003000000"
    );
}
#[test]
fn runtime_constructors_have_independent_literal_bytes() {
    check!(
        operation,
        Operation::StringFromUtf8 { bytes: vec![0xc3, 0xa9], failure: 1 },
        "3302000000c3a901000000"
    );
    check!(
        operation,
        Operation::StringConcat {
            left: PlaceOrLoan::Place(1),
            right: PlaceOrLoan::Loan(2),
            failure: 3
        },
        "340001000000010200000003000000"
    );
    check!(
        operation,
        Operation::VecConstruct { ty: 1, elements: vec![2], failure: 3 },
        "3501000000010000000200000003000000"
    );
    check!(
        operation,
        Operation::VecPush { vector: PlaceOrLoan::Loan(1), value: 2, failure: 3 },
        "3601010000000200000003000000"
    );
    check!(
        operation,
        Operation::SharedConstruct { ty: 1, value: 2, failure: 3 },
        "37010000000200000003000000"
    );
    check!(
        operation,
        Operation::WeakDowngrade { shared: PlaceOrLoan::Place(1), failure: 2 },
        "38000100000002000000"
    );
}
#[test]
fn loan_operations_have_independent_literal_bytes() {
    check!(operation, Operation::BeginLoan { loan: 1, failure: None }, "400100000000");
    check!(operation, Operation::BeginLoan { loan: 1, failure: Some(2) }, "40010000000102000000");
    check!(operation, Operation::EndLoan { loan: 1 }, "4101000000");
    check!(operation, Operation::ReadLoan { loan: 1 }, "4201000000");
    check!(operation, Operation::ReplaceLoan { loan: 1, value: 2 }, "430100000002000000");
    check!(
        operation,
        Operation::IndexCopy { base: PlaceOrLoan::Place(1), index: 2, failure: 3 },
        "4400010000000200000003000000"
    );
}
#[test]
fn all_scalar_submodes_pin_operand_endianness() {
    use ScalarBinaryKind::*;
    for (kind, bytes) in [
        (Add, "03000403020108070605"),
        (Sub, "03010403020108070605"),
        (Mul, "03020403020108070605"),
        (Eq, "03030403020108070605"),
        (Ne, "03040403020108070605"),
        (LtS, "03050403020108070605"),
        (LeS, "03060403020108070605"),
        (GtS, "03070403020108070605"),
        (GeS, "03080403020108070605"),
    ] {
        super::verify(
            &Operation::ScalarBinary { kind, left: 0x0102_0304, right: 0x0506_0708 },
            bytes,
            super::read::operation,
            super::Writer::operation,
        );
    }
}
