//! Unchanged finite Clone call census; stage 1 changes only Drop destinations.
use super::{Census, CloneNodeKind, Diagnostic, O, VerifiedCloneNode, invariant};
pub(super) fn check<'a>(
    c: &mut Census<'a, '_, '_>,
    from: &'a str,
    node: VerifiedCloneNode<'_, '_>,
) -> Result<(), Diagnostic> {
    match node.kind() {
        CloneNodeKind::Copy => {}
        CloneNodeKind::String => c.runtime(from, O::StringClone)?,
        CloneNodeKind::SharedCount => c.runtime(from, O::StrongClone)?,
        CloneNodeKind::WeakCount => c.runtime(from, O::WeakClone)?,
        CloneNodeKind::Struct => {
            for (_, child) in node.fields() {
                c.clone(from, child.ty())?;
                c.drop(from, child.ty())?;
            }
        }
        CloneNodeKind::Enum => {
            for (_, child) in node.variants() {
                if let Some(child) = child {
                    c.clone(from, child.ty())?;
                }
            }
        }
        CloneNodeKind::FixedArray { length } => {
            if length != 0 {
                let child = node.element().ok_or_else(invariant)?;
                c.clone(from, child.ty())?;
                c.drop(from, child.ty())?;
            }
        }
        CloneNodeKind::VecEach => {
            c.runtime(from, O::VecAllocate)?;
            let child = node.element().ok_or_else(invariant)?;
            c.clone(from, child.ty())?;
            c.drop(from, child.ty())?;
            c.runtime(from, O::VecReleaseStorage)?;
        }
    }
    Ok(())
}
