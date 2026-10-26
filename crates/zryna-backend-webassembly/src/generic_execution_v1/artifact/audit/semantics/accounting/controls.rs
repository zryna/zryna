//! Ordered independently decoded control obligations, including typed packed-result widths.
use super::{Diagnostic, Proof, T};
use crate::generic_execution_v1::model::containers as C;
mod birth;
mod common;
mod counts;
mod release;
pub(super) fn check(p: &mut Proof<'_, '_, '_, '_, '_>, role: usize) -> Result<(), Diagnostic> {
    match role {
        0 => birth::check(p),
        1..=3 | 7 => counts::check(p, role),
        4..=6 => release::check(p, role),
        _ => Err(super::invariant()),
    }
}
