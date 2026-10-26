//! Normalized terms retain opaque owner identities and ordered substitutions.
use super::{Budget, Engine, Failure, Frame, add, budget, reject};

#[derive(Clone, Copy)]
pub(super) struct Shape {
    pub bytes: usize,
    pub depth: usize,
}
#[derive(Clone, Copy)]
pub(super) struct Fact {
    pub affine: bool,
    pub height: usize,
}
pub(super) struct Term {
    pub key: [u32; 6],
    pub shape: Shape,
    pub state: u8,
    pub fact: Option<Fact>,
    pub members: Vec<Option<u32>>,
}
impl Term {
    pub(super) fn children(&self) -> &[u32] {
        if matches!(self.key[0], 0x10..=0x15) {
            &self.key[4..4 + self.key[3] as usize]
        } else {
            &[]
        }
    }
}
impl Engine<'_, '_> {
    pub(super) fn opaque(
        &mut self,
        owner: Frame,
        slot: u32,
        credit: &mut Budget,
    ) -> Result<u32, Failure> {
        // Opaque owner/slot is identity, never a closed wire encoding.
        self.intern([0x30 + slot, owner.kind, owner.module, owner.index, 0, 0], credit)
    }
    pub(super) fn intern(&mut self, key: [u32; 6], credit: &mut Budget) -> Result<u32, Failure> {
        if let Some(id) = self.memo.get(&key, credit)? {
            return Ok(id);
        }
        let tag = key[0];
        let mut shape = Shape { bytes: 1, depth: 0 };
        if matches!(tag, 0x10..=0x15) {
            if key[3] > 2 {
                return Err(reject("contextual term arity exceeds two"));
            }
            shape.bytes = match tag {
                0x10 | 0x11 => 9,
                0x12 | 0x13 => 13,
                _ => 5,
            };
            for child in &key[4..4 + key[3] as usize] {
                let child = self.terms.get(*child as usize).ok_or(Failure::InternalFailure)?;
                shape.bytes = add(shape.bytes, add(4, child.shape.bytes)?)?;
                shape.depth = shape.depth.max(add(1, child.shape.depth)?);
            }
        } else if !matches!(tag, 0 | 1 | 2 | 0x30 | 0x31) {
            return Err(reject("contextual term is outside the stored source domain"));
        }
        if shape.bytes > 4096 || shape.depth > 64 {
            return Err(budget("contextual complete key exceeds inherited byte/depth ceiling"));
        }
        credit.nominal(1)?;
        let id = u32::try_from(self.terms.len()).map_err(|_| Failure::InternalFailure)?;
        self.terms.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.memo.insert(key, id, credit)?;
        self.terms.push(Term { key, shape, state: 0, fact: None, members: Vec::new() });
        Ok(id)
    }
    pub(super) fn application(
        &mut self,
        tag: u32,
        module: u32,
        index: u32,
        args: &[u32],
        credit: &mut Budget,
    ) -> Result<u32, Failure> {
        if args.len() > 2 {
            return Err(reject("contextual application has inexact arity"));
        }
        let mut key = [
            tag,
            module,
            index,
            u32::try_from(args.len()).map_err(|_| Failure::InternalFailure)?,
            0,
            0,
        ];
        key[4..4 + args.len()].copy_from_slice(args);
        self.intern(key, credit)
    }
}
