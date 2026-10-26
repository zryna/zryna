//! Complete operation vocabulary for the independent execution domain.
use super::{PlaceOrLoan, ReadMode};
/// Exact scalar binary meaning, never an implicit owned operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarBinaryKind {
    /// Wrapping addition.
    Add,
    /// Wrapping subtraction.
    Sub,
    /// Wrapping multiplication.
    Mul,
    /// Exact scalar equality.
    Eq,
    /// Exact scalar inequality.
    Ne,
    /// Signed less-than.
    LtS,
    /// Signed less-than-or-equal.
    LeS,
    /// Signed greater-than.
    GtS,
    /// Signed greater-than-or-equal.
    GeS,
}
/// One private-call argument, in source evaluation order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallArgument {
    /// Prepared ordinary value.
    Value(u32),
    /// Active formal/private loan.
    Loan(u32),
}
/// Complete untrusted operations; source/type/state verification remains mandatory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    /// Unstored unit result.
    Unit,
    /// Exact boolean literal.
    BoolLiteral(bool),
    /// Exact signed literal bits.
    I32Literal(i32),
    /// Wrapping arithmetic or exact scalar comparison.
    ScalarBinary {
        /// Scalar operation.
        kind: ScalarBinaryKind,
        /// Left value.
        left: u32,
        /// Right value.
        right: u32,
    },
    /// Wrapping negation.
    I32Neg {
        /// Prepared operand.
        value: u32,
    },
    /// Copy or move a static place.
    ReadPlace {
        /// Exact place.
        place: u32,
        /// Required observation mode.
        mode: ReadMode,
    },
    /// Commit a prepared value to an uninitialized place.
    InitializePlace {
        /// Exact destination.
        place: u32,
        /// Prepared value.
        value: u32,
    },
    /// Drop old value and commit the prepared replacement.
    ReplacePlace {
        /// Exact destination.
        place: u32,
        /// Prepared value.
        value: u32,
    },
    /// Release exact current initialized state.
    DropPlace {
        /// Exact place.
        place: u32,
    },
    /// Clone the original referent using a retained derived frontier.
    Clone {
        /// Exact readable source.
        source: PlaceOrLoan,
        /// Outer failure cleanup, absent for Copy.
        outer_failure: Option<u32>,
        /// Partial prefix cleanup, absent for Copy.
        prefix_failure: Option<u32>,
    },
    /// Call an exact closed generic instance.
    ClosedGenericCall {
        /// Dense instance identity, excluding roots.
        instance: u32,
        /// Source-ordered arguments.
        arguments: Vec<CallArgument>,
        /// Exact failure cleanup if applicable.
        failure: Option<u32>,
    },
    /// Call an exact nongeneric original source root.
    SourceCall {
        /// Original module.
        module: u32,
        /// Original function index.
        function: u32,
        /// Source-ordered arguments.
        arguments: Vec<CallArgument>,
        /// Exact failure cleanup if applicable.
        failure: Option<u32>,
    },
    /// Declaration-ordered inline struct construction.
    StructConstruct {
        /// Exact stored type.
        ty: u32,
        /// Prepared declaration-ordered values.
        fields: Vec<u32>,
    },
    /// Fixed active enum construction.
    ClosedEnumConstruct {
        /// Exact stored enum type.
        ty: u32,
        /// Exact variant ordinal.
        ordinal: u32,
        /// Active prepared payload, if any.
        payload: Option<u32>,
    },
    /// Ascending inline array construction.
    FixedArrayConstruct {
        /// Exact stored array type.
        ty: u32,
        /// Prepared ascending element values.
        elements: Vec<u32>,
    },
    /// Prepare an exact source UTF-8 literal.
    StringFromUtf8 {
        /// Exact validated source bytes.
        bytes: Vec<u8>,
        /// Exact failure cleanup.
        failure: u32,
    },
    /// Prepare an owned concatenated String.
    StringConcat {
        /// Left readable String.
        left: PlaceOrLoan,
        /// Right readable String.
        right: PlaceOrLoan,
        /// Exact failure cleanup.
        failure: u32,
    },
    /// Allocate then commit already prepared source operands.
    VecConstruct {
        /// Exact stored Vec type.
        ty: u32,
        /// Pending prepared element values.
        elements: Vec<u32>,
        /// Exact allocation failure cleanup.
        failure: u32,
    },
    /// Prepare growth before committing the new element.
    VecPush {
        /// Exclusive Vec source.
        vector: PlaceOrLoan,
        /// Already evaluated new element.
        value: u32,
        /// Exact failure cleanup.
        failure: u32,
    },
    /// Allocate a control block before consuming pending payload.
    SharedConstruct {
        /// Exact stored Shared type.
        ty: u32,
        /// Prepared payload value.
        value: u32,
        /// Exact failure cleanup.
        failure: u32,
    },
    /// Acquire one checked Weak reference.
    WeakDowngrade {
        /// Readable exact Shared source.
        shared: PlaceOrLoan,
        /// Exact failure cleanup.
        failure: u32,
    },
    /// Establish a declared checked loan at the exact source site.
    BeginLoan {
        /// Loan arena identity.
        loan: u32,
        /// Indexed/access failure cleanup when required.
        failure: Option<u32>,
    },
    /// End one exact live loan.
    EndLoan {
        /// Loan arena identity.
        loan: u32,
    },
    /// Observe an active Copy referent.
    ReadLoan {
        /// Loan arena identity.
        loan: u32,
    },
    /// Commit prepared RHS through an exclusive loan.
    ReplaceLoan {
        /// Loan arena identity.
        loan: u32,
        /// Prepared replacement.
        value: u32,
    },
    /// Checked dynamic Copy indexing.
    IndexCopy {
        /// Readable exact container.
        base: PlaceOrLoan,
        /// Prepared i32 index.
        index: u32,
        /// Exact bounds failure cleanup.
        failure: u32,
    },
}
