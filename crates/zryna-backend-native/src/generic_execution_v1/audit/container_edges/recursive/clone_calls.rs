//! Source-frontier-derived new Clone/init edges; no encoder references are an authority.
use super::super::{Census, CloneNodeKind, Diagnostic, O, VerifiedCloneNode, invariant};
use crate::generic_execution_v1::recursive::calls::{Kind, name};
pub(super) fn extend(
    census: &mut Census<'_, '_, '_>,
    nodes: &[Option<VerifiedCloneNode<'_, '_>>],
) -> Result<(), Diagnostic> {
    let clone = name(Kind::Clone);
    let init = name(Kind::Init);
    census.add(clone, name(Kind::Arena))?;
    census.add(clone, name(Kind::Find))?;
    census.add(clone, init)?;
    // No certified fallible leaf means no predecessor can enter rollback;
    // Cranelift removes that unreachable block and its prefix-Drop call.
    if nodes.iter().flatten().any(|node| {
        matches!(
            node.kind(),
            CloneNodeKind::String
                | CloneNodeKind::VecEach
                | CloneNodeKind::SharedCount
                | CloneNodeKind::WeakCount
        )
    }) {
        census.add(clone, name(Kind::Drop))?;
    }
    for node in nodes.iter().flatten() {
        match node.kind() {
            CloneNodeKind::Copy => {}
            CloneNodeKind::String => census.runtime(clone, O::StringClone)?,
            CloneNodeKind::SharedCount => census.runtime(clone, O::StrongClone)?,
            CloneNodeKind::WeakCount => census.runtime(clone, O::WeakClone)?,
            CloneNodeKind::Struct => {
                for _ in node.fields() {
                    census.add(init, init)?;
                }
            }
            CloneNodeKind::Enum => {
                for (_, child) in node.variants() {
                    if child.is_some() {
                        census.add(init, init)?;
                    }
                }
            }
            CloneNodeKind::FixedArray { length } => {
                if length != 0 {
                    let _ = node.element().ok_or_else(invariant)?;
                    census.add(init, init)?;
                }
            }
            CloneNodeKind::VecEach => {
                census.runtime(clone, O::VecAllocate)?;
                census.add(clone, init)?;
                census.add(clone, init)?;
                census.add(clone, name(Kind::Drop))?;
                census.add(clone, name(Kind::Drop))?;
            }
        }
    }
    Ok(())
}
