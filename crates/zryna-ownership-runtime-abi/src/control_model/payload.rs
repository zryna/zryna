use super::{Drop, Location, Model, RuntimeAbiViolation, TypeId, budget, violation};
use zryna_layout::TypeCategory;

/// One exact typed node of an initialized payload, not a claim of runtime memory contents.
#[derive(Clone, Debug)]
pub struct PayloadNode {
    /// Layout-branded exact type.
    pub ty: TypeId,
    /// Complete initialized shape; there are no partial-publication nodes.
    pub kind: PayloadKind,
}
/// Flat, child-before-parent initialized payload topology.
#[derive(Clone, Debug)]
pub enum PayloadKind {
    /// A bool or i32 leaf.
    Copy,
    /// An owned String leaf; model cleanup is symbolic, not executed byte destruction.
    String,
    /// Exact declaration-order Struct fields, or ascending FixedArray/Vec elements.
    Children(Vec<u32>),
    /// Exact active variant ordinal and its optional payload node.
    Variant(u32, Option<u32>),
    /// A previously issued explicit Shared/Weak owner of the exact referent.
    Handle(u32),
}

impl Model<'_> {
    pub(super) fn payload(
        &mut self,
        ty: TypeId,
        nodes: &[PayloadNode],
    ) -> Result<Vec<Drop>, RuntimeAbiViolation> {
        self.nodes = super::checked_model_count(
            self.nodes,
            nodes.len() as u64,
            super::MAX_STATUS_TRANSITIONS,
        )?;
        if nodes.is_empty() {
            return Err(budget());
        }
        if nodes.last().is_none_or(|node| node.ty != ty) {
            return Err(violation("payload root has the wrong exact type"));
        }
        let mut uses = vec![0_u8; nodes.len()];
        let mut handles = std::collections::BTreeSet::new();
        for (index, node) in nodes.iter().enumerate() {
            let record = self
                .layouts
                .type_by_id(node.ty)
                .ok_or_else(|| violation("payload node has a foreign layout type"))?;
            let mut edge = |child: u32, expected: TypeId| -> Result<(), RuntimeAbiViolation> {
                let child = child as usize;
                if child >= index || nodes[child].ty != expected || uses[child] != 0 {
                    return Err(violation(
                        "payload children have wrong type, order or unique ownership",
                    ));
                }
                uses[child] = 1;
                Ok(())
            };
            match (&node.kind, record.category()) {
                (PayloadKind::Copy, TypeCategory::Bool | TypeCategory::I32)
                | (PayloadKind::String, TypeCategory::String) => {}
                (PayloadKind::Children(children), TypeCategory::Struct) => {
                    if children.len() != record.fields().len() {
                        return Err(violation("payload Struct fields are incomplete"));
                    }
                    for (&child, field) in children.iter().zip(record.fields()) {
                        edge(child, field.ty())?;
                    }
                }
                (PayloadKind::Children(children), TypeCategory::FixedArray | TypeCategory::Vec) => {
                    if let Some(length) = record.array_length() {
                        if children.len() as u64 != length {
                            return Err(violation("payload array length is not exact"));
                        }
                    } else {
                        let element = self
                            .layouts
                            .type_by_id(
                                record
                                    .referenced_type()
                                    .ok_or_else(|| violation("element type absent"))?,
                            )
                            .ok_or_else(|| violation("element layout absent"))?;
                        if children.len() as u64 > super::super::MAX_VEC_ELEMENTS
                            || element.size() == 0
                            || (children.len() as u64)
                                .checked_mul(element.size())
                                .is_none_or(|bytes| bytes > super::MAX_DYNAMIC_ALLOCATION_BYTES)
                        {
                            return Err(violation(
                                "payload Vec length or element stride is invalid",
                            ));
                        }
                    }
                    let element =
                        record.referenced_type().ok_or_else(|| violation("element type absent"))?;
                    for &child in children {
                        edge(child, element)?;
                    }
                }
                (PayloadKind::Variant(variant, child), TypeCategory::Enum) => {
                    let variant = record
                        .variants()
                        .get(*variant as usize)
                        .ok_or_else(|| violation("payload Enum variant is absent"))?;
                    match (*child, variant.payload()) {
                        (Some(child), Some(ty)) => edge(child, ty)?,
                        (None, None) => {}
                        _ => return Err(violation("payload Enum active shape is not exact")),
                    }
                }
                (PayloadKind::Handle(id), TypeCategory::Shared | TypeCategory::Weak) => {
                    self.payload_handle(
                        *id,
                        record.category(),
                        record.referenced_type(),
                        &mut handles,
                    )?;
                }
                _ => return Err(violation("payload node does not match its sealed category")),
            }
        }
        if uses[..nodes.len() - 1].iter().any(|&count| count != 1) {
            return Err(violation("payload contains orphan initialized nodes"));
        }
        Ok(self.payload_drops(nodes))
    }

    fn payload_handle(
        &self,
        id: u32,
        category: TypeCategory,
        referent: Option<TypeId>,
        handles: &mut std::collections::BTreeSet<u32>,
    ) -> Result<(), RuntimeAbiViolation> {
        let owner = self
            .owners
            .get(id as usize)
            .ok_or_else(|| violation("payload handle was never issued"))?;
        let control = &self.controls[owner.control as usize];
        if owner.location != Location::External
            || !handles.insert(id)
            || owner.weak != (category == TypeCategory::Weak)
            || referent != Some(control.payload)
        {
            return Err(violation(
                "payload handle is foreign, moved, duplicated or has wrong referent",
            ));
        }
        Ok(())
    }

    fn payload_drops(&self, nodes: &[PayloadNode]) -> Vec<Drop> {
        let mut stack = vec![(nodes.len() - 1, false)];
        let mut drops = vec![];
        while let Some((index, storage)) = stack.pop() {
            let node = &nodes[index];
            let record = self.layouts.type_by_id(node.ty).expect("validated payload type");
            match &node.kind {
                PayloadKind::Copy => {}
                PayloadKind::String => {
                    drops.push(Drop::Node(u32::try_from(index).expect("bounded nodes")));
                }
                PayloadKind::Handle(owner) => drops.push(Drop::Handle(*owner)),
                PayloadKind::Children(children) => {
                    if storage {
                        drops.push(Drop::Node(u32::try_from(index).expect("bounded nodes")));
                        continue;
                    }
                    if record.category() == TypeCategory::Vec {
                        stack.push((index, true));
                    }
                    stack.extend(children.iter().map(|&child| (child as usize, false)));
                }
                PayloadKind::Variant(_, child) => {
                    if let Some(child) = child {
                        stack.push((*child as usize, false));
                    }
                }
            }
        }
        drops
    }
}
