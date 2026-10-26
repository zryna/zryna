//! Seal-derived obligations, independent of the encoder's body table and signatures.
use super::{Diagnostic, Model, budget, invariant, sections};
use wasmparser::FunctionBody;
mod accounting;
mod expected;
mod expressions;
mod grammar;
mod kernels;
mod predicates;
mod scan;
use expressions::Expr;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Region {
    None,
    Failure,
    Child,
    Tag,
    Block,
    Branch,
    Bounds,
    Release,
    LastStrong,
    Element,
    Capacity,
    Empty,
    AllocationStatus,
    Length,
    StrongZero,
    WeakZero,
    NoOwners,
    RuntimeGuard,
    AppendFull,
    UpgradeSuccess,
    UpgradeExpired,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    RuntimeGuard(predicates::Guard),
    AppendFull(i64),
    LoanAppendFull(u32, i64, i64),
    LoanVecBounds(u32, i64, i64),
    LoanTag(u32, i64, i64, i64),
    UpgradeSuccess,
    UpgradeExpired,
    RuntimeReturn(Expr),
    ControlGuard(Region),
    LiveOwners(Expr),
    Read { address: Expr, width: u8 },
    Trap,
    WideReturn(Expr),
    LastStrong,
    Element(i64),
    Capacity(i64),
    Empty,
    AllocationStatus,
    Length,
    VecBounds(u32, i64),
    ReleaseFailure,
    End(Region),
    Block(i64),
    Branch(i64),
    Edge(i64),
    Jump(u32),
    Index(Expr),
    Status(i64),
    Store { address: Expr, value: Expr, width: u8 },
    ChildFailure,
    ChildReturn,
    Call { target: u32, args: Vec<Expr> },
    Tag { base: u32, offset: i64, ordinal: i64 },
    Failure,
    FailureReturn,
    SuccessReturn,
    TrapReturn(i64),
    Bounds(i64),
}
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if m.recursive_function(index).is_some() {
        return crate::generic_execution_v1::recursive::check_body(body, index, m);
    }
    if m.recursive_count() != 0 && index >= 5 && index < m.source_base as usize {
        return crate::generic_execution_v1::recursive::check_wrapper(body, index, m);
    }
    if m.accounted()
        && (index >= m.accounting_base() as usize
            || (index >= m.wrapper_base as usize && index < m.observer as usize))
    {
        return accounting::check(body, index, m);
    }
    if (m.concat && index >= m.concat_allocator() as usize)
        || (m.continuation && index >= m.container_base as usize + 10)
    {
        return kernels::continuation::check(body, index, m);
    }
    grammar::check(body, index, m)?;
    let actual = scan::events(body, index, m)?;
    let expected = expected::events(index, m)?;
    m.take(actual.len().checked_add(expected.len()).ok_or_else(budget)?)?;
    if actual != expected {
        return Err(invariant());
    }
    kernels::check(body, index, m)?;
    Ok(())
}
fn push(events: &mut Vec<Event>, event: Event, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    m.take(size_of::<Event>())?;
    events.try_reserve(1).map_err(|_| budget())?;
    events.push(event);
    Ok(())
}
fn call(
    events: &mut Vec<Event>,
    target: u32,
    args: &[Expr],
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    m.take(args.len().checked_mul(size_of::<Expr>()).ok_or_else(budget)?)?;
    let mut owned = Vec::new();
    owned.try_reserve_exact(args.len()).map_err(|_| budget())?;
    owned.extend_from_slice(args);
    push(events, Event::Call { target, args: owned }, m)
}
