//! Independent actual control8 SSA guards, status joins and funded release predicates.
use super::{
    Block, D, Diagnostic, Function, Inst, IntCC, LogicalOperation, O, Value, args, compare,
    constant, data, flow, invariant, load, origin, state, stored, types,
};
mod acquire;
mod common;
mod outcomes;
mod release;
pub(super) use acquire::check as acquire;
pub(super) use release::check as release;
