//! Source-layout-derived direct-call inventory; no encoder call table is trusted.
use super::{Census, Diagnostic, O, TypeCategory, invariant};
use crate::generic_execution_v1::recursive::{
    STUB_NAME,
    calls::{Kind, name},
};
mod clone_calls;
pub(super) fn extend(
    c: &mut Census<'_, '_, '_>,
    nodes: &[Option<super::VerifiedCloneNode<'_, '_>>],
    entry: crate::generic_execution_v1::invocation::Entry,
) -> Result<(), Diagnostic> {
    let arena = name(Kind::Arena);
    let find = name(Kind::Find);
    let drop = name(Kind::Drop);
    let release = name(Kind::Release);
    c.add(arena, STUB_NAME)?;
    c.add(drop, arena)?;
    c.add(drop, find)?;
    for (name, _) in entry.wrappers(c.p) {
        c.add(name, STUB_NAME)?;
    }
    for id in 0..c.p.program().type_count() {
        let id = u32::try_from(id).map_err(|_| invariant())?;
        let view = c.p.stored_type(id).ok_or_else(invariant)?;
        match view.category() {
            TypeCategory::String => c.runtime(drop, O::StringRelease)?,
            TypeCategory::Weak => c.runtime(drop, O::WeakRelease)?,
            TypeCategory::Shared => {
                c.runtime(drop, O::StrongReleaseBegin)?;
                c.runtime(drop, O::StrongReleaseFinish)?;
            }
            TypeCategory::Vec => {
                // Empty descriptor release and final resumed element use distinct call sites.
                c.add(drop, release)?;
                c.add(drop, release)?;
                c.runtime(release, O::VecReleaseStorage)?;
            }
            TypeCategory::Bool
            | TypeCategory::I32
            | TypeCategory::Struct
            | TypeCategory::Enum
            | TypeCategory::FixedArray => {}
        }
        if view.size() == 0 {
            continue;
        }
        match view.category() {
            TypeCategory::Struct => {
                for _ in view.fields() {
                    c.add(find, find)?;
                    c.add(find, find)?;
                }
            }
            TypeCategory::Enum => {
                for (_, payload) in view.variants() {
                    if payload.is_some() {
                        c.add(find, find)?;
                    }
                }
            }
            TypeCategory::FixedArray
                if view.array_length().ok_or_else(invariant)? != 0
                    && view.array_stride().ok_or_else(invariant)? != 0 =>
            {
                c.add(find, find)?;
            }
            _ => {}
        }
    }
    clone_calls::extend(c, nodes)?;
    Ok(())
}
