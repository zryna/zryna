//! Headers may precede stored edges; all mutations prepay and publish atomically.
use super::{Fact, Failure, Kind, Node, TypeWorld, add, charge, exhausted, reject, reserve};
#[derive(Debug, Default)]
pub(in crate::generic_v1::execution_v1) struct Builder {
    nodes: Vec<Node>,
    index: Vec<([u32; 8], u32)>,
    expected: Option<usize>,
    members: usize,
}
impl Builder {
    pub(in crate::generic_v1::execution_v1) const fn new() -> Self {
        Self { nodes: Vec::new(), index: Vec::new(), expected: None, members: 0 }
    }
    pub(in crate::generic_v1::execution_v1) fn closed(count: usize) -> Result<Self, Failure> {
        if count > zryna_layout::MAX_TYPE_NODES {
            return Err(exhausted("TypeWorld type ceiling 65536"));
        }
        Ok(Self { expected: Some(count), ..Self::new() })
    }
    #[cfg(test)]
    pub(in crate::generic_v1::execution_v1) fn len(&self) -> usize {
        self.nodes.len()
    }
    pub(in crate::generic_v1::execution_v1) fn declare(
        &mut self,
        kind: Kind,
        arguments: &[u32],
        spent: &mut usize,
    ) -> Result<u32, Failure> {
        if kind.arity() > 2 || arguments.len() != kind.arity() {
            return Err(reject("TypeWorld constructor arity differs"));
        }
        if let Kind::Opaque { owner, slot } = kind
            && (owner.kind > 1 || slot > 1)
        {
            return Err(reject("invalid opaque owner/slot"));
        }
        if let Kind::FixedArray { length } = kind
            && u64::from(length) > zryna_layout::MAX_ARRAY_LENGTH
        {
            return Err(reject("fixed array length exceeds inherited bound"));
        }
        charge(spent, add(arguments.len(), arguments.len())?)?;
        let bound = self.expected.unwrap_or(self.nodes.len());
        if arguments.iter().any(|a| *a as usize >= bound) {
            return Err(reject("TypeWorld argument is absent"));
        }
        let key = kind.identity(arguments);
        let mut low = 0;
        let mut high = self.index.len();
        while low < high {
            let mid = low + (high - low) / 2;
            charge(spent, 9)?;
            match self.index[mid].0.cmp(&key) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
                std::cmp::Ordering::Equal => return Ok(self.index[mid].1),
            }
        }
        if self.nodes.len() >= zryna_layout::MAX_TYPE_NODES {
            return Err(exhausted("TypeWorld type ceiling 65536"));
        }
        // Retain the referenced-type description independently of graph relations.
        // proof selects validity, storage and capability edges from Kind explicitly.
        let members = match kind {
            Kind::Option | Kind::Result => 2,
            Kind::FixedArray { .. } | Kind::Vec | Kind::Shared | Kind::Weak => 1,
            _ => 0,
        };
        let total = add(self.members, members)?;
        if total > zryna_layout::MAX_MEMBERS {
            return Err(exhausted("TypeWorld global member ceiling"));
        }
        let mut checked = *spent;
        charge(
            &mut checked,
            add(add(arguments.len(), members)?, add(2, self.index.len() - low)?)?,
        )?;
        let mut args = reserve(arguments.len())?;
        args.extend_from_slice(arguments);
        let mut fields = reserve(members)?;
        match kind {
            Kind::Option => fields.extend([None, Some(arguments[0])]),
            Kind::Result => fields.extend([Some(arguments[0]), Some(arguments[1])]),
            Kind::FixedArray { .. } | Kind::Vec | Kind::Shared | Kind::Weak => {
                fields.push(Some(arguments[0]));
            }
            _ => {}
        }
        self.nodes.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.index.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        let id = u32::try_from(self.nodes.len()).map_err(|_| Failure::InternalFailure)?;
        self.nodes.push(Node {
            kind,
            arguments: args,
            members: fields,
            ready: !matches!(kind, Kind::Struct { .. } | Kind::Enum { .. }),
            key: Vec::new(),
        });
        self.index.insert(low, (key, id));
        self.members = total;
        *spent = checked;
        Ok(id)
    }
    pub(in crate::generic_v1::execution_v1) fn set_members(
        &mut self,
        id: u32,
        members: &[Option<u32>],
        spent: &mut usize,
    ) -> Result<(), Failure> {
        charge(spent, 1)?;
        let node = self.nodes.get(id as usize).ok_or_else(|| reject("absent member owner"))?;
        if node.ready
            || members.is_empty()
            || members.len() > zryna_layout::MAX_MEMBERS_PER_DECLARATION
        {
            return Err(reject("inexact or repeated member inventory"));
        }
        let mut checked = *spent;
        charge(&mut checked, add(members.len(), members.len())?)?;
        if members.iter().any(|child| {
            child.is_none() && matches!(node.kind, Kind::Struct { .. })
                || child.is_some_and(|c| c as usize >= self.nodes.len())
        }) {
            return Err(reject("invalid stored member identity"));
        }
        let total = add(self.members, members.len())?;
        if total > zryna_layout::MAX_MEMBERS {
            return Err(exhausted("TypeWorld global member ceiling"));
        }
        let mut retained = reserve(members.len())?;
        retained.extend_from_slice(members);
        let node = &mut self.nodes[id as usize];
        node.members = retained;
        node.ready = true;
        self.members = total;
        *spent = checked;
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1) fn set_key(
        &mut self,
        id: u32,
        key: &[u8],
        spent: &mut usize,
    ) -> Result<(), Failure> {
        if key.len() > 4096 {
            return Err(exhausted("TypeWorld key ceiling 4096"));
        }
        let mut checked = *spent;
        charge(&mut checked, add(1, key.len())?)?;
        let node = self.nodes.get(id as usize).ok_or_else(|| reject("absent keyed type"))?;
        if !node.key.is_empty() {
            return Err(reject("repeated TypeWorld key"));
        }
        let mut retained = reserve(key.len())?;
        retained.extend_from_slice(key);
        self.nodes[id as usize].key = retained;
        *spent = checked;
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1) fn seal(
        self,
        spent: &mut usize,
    ) -> Result<TypeWorld, Failure> {
        let mut checked = *spent;
        if self.expected.is_some_and(|n| n != self.nodes.len()) {
            return Err(reject("incomplete closed TypeWorld"));
        }
        let facts: Vec<Fact> = super::proof::derive(&self.nodes, &mut checked)?;
        *spent = checked;
        Ok(TypeWorld { nodes: self.nodes, facts })
    }
}
