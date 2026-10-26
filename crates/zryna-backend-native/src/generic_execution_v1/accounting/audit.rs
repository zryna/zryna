//! Independent actual-SSA recipe checks; no emitted recipe/body is accepted as proof.
use super::super::invariant;
use cranelift_codegen::ir::{
    Block, FuncRef, Function, Inst, InstructionData as D, Opcode as O, Value, condcodes::IntCC,
    types,
};
use zryna_diagnostics::Diagnostic;
use zryna_ownership_runtime_abi::{
    LogicalOperation, MAX_ALLOCATION_OPERATIONS as A, MAX_LIVE_ALLOCATIONS as L,
    MAX_STATUS_TRANSITIONS as S,
};
mod acquire;
mod common;
mod control;
mod flow;
mod function;
mod release;
mod state;
mod vector;
pub(super) use acquire::check as acquire;
use common::{args, compare, constant, data, limit, load, origin, stored};
pub(super) use function::check as function;
pub(super) use release::check as release;

#[cfg(test)]
mod tests;
