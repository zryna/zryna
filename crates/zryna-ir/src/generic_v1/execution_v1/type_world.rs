//! Finite source graph: validity, by-value storage and capability edges are distinct.
use super::budget::charge;
use crate::generic_v1::{Failure, budget as exhausted, reject, reserve};
mod builder;
mod proof;
#[cfg(test)]
mod tests;
pub(super) use builder::Builder;
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Frame {
    pub kind: u32,
    pub module: u32,
    pub index: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    Bool,
    I32,
    String,
    Opaque { owner: Frame, slot: u32 },
    Struct { module: u32, declaration: u32, arity: u32 },
    Enum { module: u32, declaration: u32, arity: u32 },
    Option,
    Result,
    FixedArray { length: u32 },
    Vec,
    Shared,
    Weak,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Fact {
    pub copy: bool,
    pub clone: bool,
}
#[derive(Debug)]
struct Node {
    kind: Kind,
    arguments: Vec<u32>,
    members: Vec<Option<u32>>,
    ready: bool,
    key: Vec<u8>,
}
#[derive(Debug)]
pub(super) struct TypeWorld {
    nodes: Vec<Node>,
    facts: Vec<Fact>,
}
impl TypeWorld {
    pub(super) fn len(&self) -> usize {
        self.nodes.len()
    }
    pub(super) fn fact(&self, id: u32) -> Option<Fact> {
        self.facts.get(id as usize).copied()
    }
    pub(super) fn kind(&self, id: u32) -> Option<Kind> {
        self.nodes.get(id as usize).map(|n| n.kind)
    }
    pub(super) fn members(&self, id: u32) -> Option<&[Option<u32>]> {
        self.nodes.get(id as usize).map(|n| n.members.as_slice())
    }
    pub(super) fn arguments(&self, id: u32) -> Option<&[u32]> {
        self.nodes.get(id as usize).map(|n| n.arguments.as_slice())
    }
    pub(super) fn keys(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.nodes.iter().map(|n| n.key.as_slice())
    }
}
impl Kind {
    fn arity(self) -> usize {
        match self {
            Self::Struct { arity, .. } | Self::Enum { arity, .. } => arity as usize,
            Self::Option | Self::FixedArray { .. } | Self::Vec | Self::Shared | Self::Weak => 1,
            Self::Result => 2,
            _ => 0,
        }
    }
    fn identity(self, arguments: &[u32]) -> [u32; 8] {
        let mut key = match self {
            Self::Bool => [0; 8],
            Self::I32 => [1, 0, 0, 0, 0, 0, 0, 0],
            Self::String => [2, 0, 0, 0, 0, 0, 0, 0],
            Self::Opaque { owner, slot } => {
                [0x30, owner.kind, owner.module, owner.index, slot, 0, 0, 0]
            }
            Self::Struct { module, declaration, arity } => {
                [0x10, module, declaration, arity, 0, 0, 0, 0]
            }
            Self::Enum { module, declaration, arity } => {
                [0x11, module, declaration, arity, 0, 0, 0, 0]
            }
            Self::Option => [0x14, 0, 0, 1, 0, 0, 0, 0],
            Self::Result => [0x15, 0, 0, 2, 0, 0, 0, 0],
            Self::FixedArray { length } => [0x20, length, 0, 1, 0, 0, 0, 0],
            Self::Vec => [0x21, 0, 0, 1, 0, 0, 0, 0],
            Self::Shared => [0x22, 0, 0, 1, 0, 0, 0, 0],
            Self::Weak => [0x23, 0, 0, 1, 0, 0, 0, 0],
        };
        key[5..5 + arguments.len()].copy_from_slice(arguments);
        key
    }
}
fn add(a: usize, b: usize) -> Result<usize, Failure> {
    a.checked_add(b).ok_or_else(|| exhausted("execution TypeWorld work overflow"))
}
