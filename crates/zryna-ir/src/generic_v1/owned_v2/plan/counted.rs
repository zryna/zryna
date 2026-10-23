//! One shared replay: old callers retain exact plans; private v4 also retains spent credit.
use super::{Failure, VerifiedLayouts, graph, raw};

pub(in super::super) fn derive(
    program: &graph::Program,
    extensions: &[Vec<raw::Extension>],
    layouts: &VerifiedLayouts,
) -> Result<Vec<raw::Plan>, Failure> {
    super::replay_counted(program, extensions, layouts).map(|(plans, _)| plans)
}

pub(in super::super) fn derive_counted(
    program: &graph::Program,
    extensions: &[Vec<raw::Extension>],
    layouts: &VerifiedLayouts,
) -> Result<(Vec<raw::Plan>, usize), Failure> {
    super::replay_counted(program, extensions, layouts)
}
