//! Canonical keys stay byte-identical; opaque ownership is retained only in normalized terms.
use super::syntax::push;
use super::{Budget, Engine, Failure, Frame, add, budget, reject, reserve};

enum Decode<'a> {
    Key(&'a [u8], usize),
    Finish(u32, u32, u32, usize),
}
impl Engine<'_, '_> {
    pub(super) fn decode(
        &mut self,
        bytes: &[u8],
        owner: Frame,
        symbolic: bool,
        credit: &mut Budget,
    ) -> Result<u32, Failure> {
        if bytes.len() > 4096 {
            return Err(budget("contextual key exceeds 4096 bytes"));
        }
        let mut pending = Vec::new();
        let mut values = Vec::new();
        push(&mut pending, Decode::Key(bytes, 0), credit)?;
        while let Some(task) = pending.pop() {
            match task {
                Decode::Key(key, depth) => {
                    credit.nominal(1)?;
                    if depth > 64 {
                        return Err(budget("contextual key depth exceeds 64"));
                    }
                    let tag =
                        u32::from(*key.first().ok_or_else(|| reject("empty contextual key"))?);
                    if matches!(tag, 0 | 1 | 2 | 0x30 | 0x31) {
                        if key.len() != 1 {
                            return Err(reject("contextual leaf has trailing bytes"));
                        }
                        let id = if tag >= 0x30 {
                            if !symbolic
                                || (tag - 0x30) as usize
                                    >= owner
                                        .parameters(self.originals)?
                                        .map_or(0, |p| p.parameters.len())
                            {
                                return Err(reject(
                                    "opaque key lacks its exact original owner slot",
                                ));
                            }
                            self.opaque(owner, tag - 0x30, credit)?
                        } else {
                            self.intern([tag, 0, 0, 0, 0, 0], credit)?
                        };
                        push(&mut values, id, credit)?;
                        continue;
                    }
                    let mut cursor = 1;
                    let (module, index) = if matches!(tag, 0x10..=0x13) {
                        (lane(key, &mut cursor, credit)?, lane(key, &mut cursor, credit)?)
                    } else {
                        (0, 0)
                    };
                    let count = match tag {
                        0x10 | 0x11 => 0,
                        0x12..=0x15 => lane(key, &mut cursor, credit)? as usize,
                        _ => return Err(reject("contextual key tag is not admitted")),
                    };
                    if count > 2
                        || matches!(tag, 0x12 | 0x13) && count == 0
                        || tag == 0x14 && count != 1
                        || tag == 0x15 && count != 2
                    {
                        return Err(reject("contextual key has inexact arity"));
                    }
                    let mut children = [&[][..]; 2];
                    for child in &mut children[..count] {
                        let length = lane(key, &mut cursor, credit)? as usize;
                        let end = add(cursor, length)?;
                        *child = key
                            .get(cursor..end)
                            .ok_or_else(|| reject("truncated contextual child key"))?;
                        cursor = end;
                    }
                    if cursor != key.len() {
                        return Err(reject("contextual key has trailing bytes"));
                    }
                    push(&mut pending, Decode::Finish(tag, module, index, count), credit)?;
                    for child in children[..count].iter().rev() {
                        push(&mut pending, Decode::Key(child, add(depth, 1)?), credit)?;
                    }
                }
                Decode::Finish(tag, module, index, count) => {
                    let mut children = [0; 2];
                    for child in children[..count].iter_mut().rev() {
                        *child = values.pop().ok_or(Failure::InternalFailure)?;
                    }
                    let id = self.application(tag, module, index, &children[..count], credit)?;
                    push(&mut values, id, credit)?;
                }
            }
        }
        if values.len() != 1 {
            return Err(Failure::InternalFailure);
        }
        values.pop().ok_or(Failure::InternalFailure)
    }
    pub(super) fn encode(&self, root: u32, credit: &mut Budget) -> Result<Vec<u8>, Failure> {
        let length = self.terms[root as usize].shape.bytes;
        credit.nominal(length)?;
        let mut output = reserve(length)?;
        let mut pending = Vec::new();
        push(&mut pending, (root, false), credit)?;
        while let Some((id, child)) = pending.pop() {
            credit.nominal(1)?;
            let term = &self.terms[id as usize];
            if child {
                output.extend_from_slice(
                    &u32::try_from(term.shape.bytes)
                        .map_err(|_| Failure::InternalFailure)?
                        .to_le_bytes(),
                );
            }
            let tag = term.key[0];
            output.push(u8::try_from(tag).map_err(|_| Failure::InternalFailure)?);
            if !matches!(tag, 0x10..=0x15) {
                continue;
            }
            if matches!(tag, 0x10..=0x13) {
                output.extend_from_slice(&term.key[1].to_le_bytes());
                output.extend_from_slice(&term.key[2].to_le_bytes());
            }
            if matches!(tag, 0x12..=0x15) {
                output.extend_from_slice(&term.key[3].to_le_bytes());
            }
            for child in term.children().iter().rev() {
                push(&mut pending, (*child, true), credit)?;
            }
        }
        if output.len() != length {
            return Err(Failure::InternalFailure);
        }
        Ok(output)
    }
}
fn lane(bytes: &[u8], cursor: &mut usize, credit: &mut Budget) -> Result<u32, Failure> {
    credit.nominal(4)?;
    let end = add(*cursor, 4)?;
    let lane = bytes.get(*cursor..end).ok_or_else(|| reject("truncated contextual header"))?;
    *cursor = end;
    Ok(u32::from_le_bytes(lane.try_into().map_err(|_| Failure::InternalFailure)?))
}
