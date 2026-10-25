//! Genuine-source SSA owner controls; mutations are applied only after pristine admission.
mod bounds;
mod fixture;
mod mutations;
#[path = "../../../tests/recursive_arena/source.rs"]
mod source;

mod clone_bounds;
mod clone_mutations;

mod clone_outcomes;

mod invocation;
