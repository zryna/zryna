//! Closed control/trap role census, independent of encoder recipes; no unclassified additions.
use super::{Diagnostic, Model, budget, invariant};
use wasmparser::{FunctionBody, Operator as O};
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, raw};
use zryna_layout::TypeCategory;
#[derive(Default, PartialEq, Eq)]
struct Counts {
    ifs: u32,
    blocks: u32,
    loops: u32,
    jumps: u32,
    conditional: u32,
    returns: u32,
    traps: u32,
    ends: u32,
    elses: u32,
}
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let mut expected = Counts::default();
    roles(&mut expected, index, m)?;
    expected.ends = expected
        .ifs
        .checked_add(expected.blocks)
        .and_then(|n| n.checked_add(expected.loops))
        .and_then(|n| n.checked_add(1))
        .ok_or_else(budget)?;
    let mut actual = Counts::default();
    let mut reader = body.get_operators_reader().map_err(|_| invariant())?;
    while !reader.eof() {
        m.take(1)?;
        let slot = match reader.read().map_err(|_| invariant())? {
            O::If { .. } => Some(&mut actual.ifs),
            O::Block { .. } => Some(&mut actual.blocks),
            O::Loop { .. } => Some(&mut actual.loops),
            O::Br { .. } => Some(&mut actual.jumps),
            O::BrIf { .. } => Some(&mut actual.conditional),
            O::Return => Some(&mut actual.returns),
            O::Unreachable => Some(&mut actual.traps),
            O::End => Some(&mut actual.ends),
            O::Else => Some(&mut actual.elses),
            _ => None,
        };
        if let Some(n) = slot {
            *n = n.checked_add(1).ok_or_else(budget)?;
        }
    }
    if actual != expected {
        return Err(invariant());
    }
    Ok(())
}
fn roles(c: &mut Counts, index: usize, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    if index >= m.container_base as usize {
        return container_role(
            c,
            crate::generic_execution_v1::checked_u32(index)? - m.container_base,
            m,
        );
    }
    if index < 5 {
        match index {
            0 => {
                c.blocks = 1;
                c.loops = 1;
                c.jumps = 1;
                c.conditional = 1;
            }
            2 => {
                c.ifs = 5 + u32::from(m.observed());
                c.returns = c.ifs;
            }
            3 => {
                c.ifs = 5;
                c.returns = 5;
            }
            4 => {
                c.ifs = 3 + u32::from(m.observed());
                c.returns = c.ifs;
            }
            _ => {}
        }
        return Ok(());
    }
    let n = m.types.len();
    if index < 5 + n {
        return clone_role(c, index - 5, m);
    }
    if index < m.source_base as usize {
        let t = m.types[index - 5 - n];
        match t.category() {
            TypeCategory::String | TypeCategory::Weak => {
                c.ifs = 1;
                c.traps = 1;
            }
            TypeCategory::Enum => {
                c.ifs = crate::generic_execution_v1::checked_u32(t.variants().len())?;
                c.returns = c.ifs;
                c.traps = 1;
            }
            TypeCategory::Vec => {
                c.ifs = 1;
                c.traps = 1;
                c.blocks = 1;
                c.loops = 1;
                c.jumps = 1;
                c.conditional = 1;
            }
            TypeCategory::Shared => {
                c.ifs = 3;
                c.traps = 2;
            }
            TypeCategory::FixedArray => {
                c.blocks = 1;
                c.loops = 1;
                c.jumps = 1;
                c.conditional = 1;
            }
            _ => {}
        }
        return Ok(());
    }
    if index < m.wrapper_base as usize {
        return source_role(c, index - m.source_base as usize, m);
    }
    if index < m.observer as usize {
        return wrapper_role(c, index, m);
    }
    if m.observed() && index == m.observer as usize {
        c.ifs = 7;
        c.returns = 3;
        c.traps = 4;
        return Ok(());
    }
    if m.observed() && index == m.recorder as usize {
        c.ifs = 2;
        c.returns = 2;
        c.blocks = 1;
        c.loops = 1;
        c.jumps = 1;
        c.conditional = 1;
        return Ok(());
    }
    Err(invariant())
}
fn clone_role(c: &mut Counts, ty: usize, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    let Some(node) = m.clones[ty] else {
        c.traps = 1;
        return Ok(());
    };
    match node.kind() {
        CloneNodeKind::Copy => {}
        CloneNodeKind::String => c.returns = 1,
        CloneNodeKind::Enum => {
            m.take(node.variants().len())?;
            let children = crate::generic_execution_v1::checked_u32(
                node.variants().filter(|(_, c)| c.is_some()).count(),
            )?;
            c.ifs = crate::generic_execution_v1::checked_u32(node.variants().len())?
                .checked_add(children)
                .ok_or_else(budget)?;
            c.returns = c.ifs;
            c.traps = 1;
        }
        CloneNodeKind::Struct => {
            c.ifs = crate::generic_execution_v1::checked_u32(node.fields().len())?;
            c.returns = c.ifs;
        }
        CloneNodeKind::FixedArray { .. } => {
            c.ifs = 1;
            c.blocks = 2;
            c.loops = 2;
            c.jumps = 2;
            c.conditional = 2;
            c.returns = 1;
        }
        CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => {
            c.ifs = 1;
            c.returns = 1;
        }
        CloneNodeKind::VecEach => {
            c.ifs = 3;
            c.returns = 2;
            c.traps = 1;
            c.blocks = 2;
            c.loops = 2;
            c.jumps = 2;
            c.conditional = 2;
        }
    }
    Ok(())
}
fn source_role(c: &mut Counts, f: usize, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    c.loops = 1;
    c.traps = 2;
    m.take(m.functions[f].blocks().len())?;
    for block in m.functions[f].blocks() {
        c.ifs += 1;
        match block.terminator() {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => c.returns += 1,
            raw::Terminator::Jump(_) => c.jumps += 1,
            raw::Terminator::Branch { .. } => {
                c.ifs += 1;
                c.jumps += 2;
            }
            raw::Terminator::ClosedEnumMatch { arms, .. } => {
                let n = crate::generic_execution_v1::checked_u32(arms.len())?;
                c.ifs += n;
                c.jumps += n;
                c.traps += 1;
            }
            raw::Terminator::WeakUpgradeBranch { .. } => return Err(invariant()),
        }
        m.take(block.operations().len())?;
        for op in block.operations() {
            match op.kind() {
                raw::Operation::Clone { .. }
                | raw::Operation::StringFromUtf8 { .. }
                | raw::Operation::SourceCall { .. }
                | raw::Operation::ClosedGenericCall { .. }
                | raw::Operation::VecConstruct { .. }
                | raw::Operation::SharedConstruct { .. }
                | raw::Operation::WeakDowngrade { .. } => {
                    c.ifs += 1;
                    c.returns += 1;
                    c.traps += u32::from(op.failure().is_none());
                }
                raw::Operation::IndexCopy { .. } => {
                    c.ifs += 1;
                    c.returns += 1;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn container_role(c: &mut Counts, role: u32, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    use crate::generic_execution_v1::model::containers as C;
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    let n = crate::generic_execution_v1::checked_u32(
        rows.iter().filter(|v| v.target() == zryna_layout::StorageTarget::Linear32V1).count(),
    )?;
    match role {
        C::ALLOCATE_RAW => {
            c.ifs = 4 + u32::from(m.observed());
            c.returns = c.ifs;
        }
        C::RELEASE_RAW => {
            c.ifs = 6;
            c.returns = 6;
        }
        C::VEC_ALLOCATE => {
            c.ifs = 1 + 3 * n;
            c.returns = 2 + 3 * n;
        }
        C::VEC_RELEASE => {
            c.ifs = 1 + 2 * n;
            c.returns = 2 + 2 * n;
        }
        C::STRONG_CLONE | C::DOWNGRADE | C::WEAK_CLONE => {
            c.ifs = 5;
            c.returns = 5;
        }
        C::STRONG_BEGIN | C::STRONG_FINISH | C::WEAK_RELEASE => {
            c.ifs = 5;
            c.returns = 4;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}

fn wrapper_role(c: &mut Counts, index: usize, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    let f = m.functions[m.program.export_functions()[index - m.wrapper_base as usize]];
    c.ifs = 3;
    m.take(f.parameters().len())?;
    for param in f.parameters() {
        if let zryna_ir::generic_v1::execution_v1::VerifiedParameter::Value(v) = param
            && let raw::ValueType::Stored(ty) = v.ty()
        {
            c.ifs += u32::from(m.types[ty as usize].size() == 1);
        }
    }
    c.traps = c.ifs;
    Ok(())
}
