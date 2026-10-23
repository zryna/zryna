//! Finite checked clone recipes, including indirection cycles without static tree expansion.
use super::{
    Failure, budget, reject, reserve, resource_failure,
    type_world::{Kind, TypeWorld},
};

/// Closed clone action in this execution domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloneNodeKind {
    /// Resource-free copy of initialized stored content.
    Copy,
    /// One fresh String acquisition.
    String,
    /// Declaration-ordered fields.
    Struct,
    /// Runtime-selected payload only.
    Enum,
    /// Exact ascending fixed element sequence.
    FixedArray {
        /// Exact verified length.
        length: u64,
    },
    /// Actual runtime elements with local child/frame progress.
    VecEach,
    /// Acquire a strong count; never clone its payload.
    SharedCount,
    /// Acquire a weak count; never clone its payload.
    WeakCount,
}
#[derive(Debug)]
pub(super) struct Node {
    pub(super) kind: CloneNodeKind,
    pub(super) children: Vec<Option<u32>>,
    pub(super) may_fail: bool,
}
#[derive(Debug)]
pub(super) struct Table {
    pub(super) nodes: Vec<Option<Node>>,
}

impl Table {
    pub(super) fn build(world: &TypeWorld, spent: &mut usize) -> Result<Self, Failure> {
        let mut work = *spent;
        budget::charge(&mut work, world.len())?;
        let mut edges = 0usize;
        for id in 0..world.len() {
            let ty = u32::try_from(id).map_err(|_| Failure::InternalFailure)?;
            let members =
                world.members(ty).ok_or_else(|| reject("source world has a missing member row"))?;
            edges = edges
                .checked_add(members.len())
                .ok_or_else(|| resource_failure("execution frontier edges overflow"))?;
        }
        let complete = world
            .len()
            .checked_mul(8)
            .and_then(|n| n.checked_add(edges.checked_mul(8)?))
            .ok_or_else(|| resource_failure("execution frontier work overflow"))?;
        budget::charge(&mut work, complete)?;
        let mut nodes = reserve(world.len())?;
        let mut parents: Vec<Vec<usize>> = reserve(world.len())?;
        parents.resize_with(world.len(), Vec::new);
        let mut pending = reserve(world.len())?;
        for id in 0..world.len() {
            let ty = u32::try_from(id).map_err(|_| Failure::InternalFailure)?;
            let fact = world
                .fact(ty)
                .ok_or_else(|| reject("source world has a missing capability row"))?;
            if !fact.clone {
                nodes.push(None);
                continue;
            }
            let kind = action(
                world.kind(ty).ok_or_else(|| reject("source world type is absent"))?,
                fact.copy,
            )?;
            let members = world.members(ty).ok_or_else(|| reject("source world members absent"))?;
            let retains = matches!(
                kind,
                CloneNodeKind::Struct
                    | CloneNodeKind::Enum
                    | CloneNodeKind::FixedArray { .. }
                    | CloneNodeKind::VecEach
            );
            let mut children = reserve(if retains { members.len() } else { 0 })?;
            if retains {
                for child in members {
                    if let Some(child) = child {
                        if *child as usize >= world.len()
                            || !world
                                .fact(*child)
                                .ok_or_else(|| reject("frontier child absent"))?
                                .clone
                        {
                            return Err(reject(
                                "frontier child lacks independent source Clone fact",
                            ));
                        }
                        let follows = !matches!(kind, CloneNodeKind::FixedArray { length: 0 });
                        if follows {
                            parents[*child as usize]
                                .try_reserve(1)
                                .map_err(|_| Failure::AllocationFailure)?;
                            parents[*child as usize].push(id);
                        }
                    }
                    children.push(*child);
                }
            }
            let may_fail = matches!(
                kind,
                CloneNodeKind::String
                    | CloneNodeKind::VecEach
                    | CloneNodeKind::SharedCount
                    | CloneNodeKind::WeakCount
            );
            if may_fail {
                pending.push(id);
            }
            nodes.push(Some(Node { kind, children, may_fail }));
        }
        while let Some(child) = pending.pop() {
            for &parent in &parents[child] {
                let node = nodes[parent]
                    .as_mut()
                    .ok_or_else(|| reject("frontier parent lacks source Clone fact"))?;
                if !node.may_fail {
                    node.may_fail = true;
                    pending.push(parent);
                }
            }
        }
        *spent = work;
        Ok(Self { nodes })
    }
    pub(super) fn node(&self, ty: u32) -> Result<&Node, Failure> {
        self.nodes
            .get(ty as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| reject("operation type has no source-authorized clone frontier"))
    }
}
fn action(kind: Kind, copy: bool) -> Result<CloneNodeKind, Failure> {
    if copy {
        return Ok(CloneNodeKind::Copy);
    }
    Ok(match kind {
        Kind::Bool | Kind::I32 => CloneNodeKind::Copy,
        Kind::String => CloneNodeKind::String,
        Kind::Struct { .. } => CloneNodeKind::Struct,
        Kind::Enum { .. } | Kind::Option | Kind::Result => CloneNodeKind::Enum,
        Kind::FixedArray { length } => CloneNodeKind::FixedArray { length: u64::from(length) },
        Kind::Vec => CloneNodeKind::VecEach,
        Kind::Shared => CloneNodeKind::SharedCount,
        Kind::Weak => CloneNodeKind::WeakCount,
        Kind::Opaque { .. } => {
            return Err(reject("opaque source atom cannot issue a clone frontier"));
        }
    })
}
