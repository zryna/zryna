//! Fixed-width Patricia identity index; lookup work never depends on name/key length.
use super::{Budget, Failure};

pub(super) type Key = [u32; 6];
enum Entry {
    Leaf(Key, u32),
    Branch { bit: usize, children: [usize; 2] },
}
#[derive(Default)]
pub(super) struct Memo {
    root: Option<usize>,
    entries: Vec<Entry>,
}
fn bit(key: &Key, position: usize) -> usize {
    ((key[position / 32] >> (31 - position % 32)) & 1) as usize
}
impl Memo {
    pub(super) fn get(&self, key: &Key, credit: &mut Budget) -> Result<Option<u32>, Failure> {
        // One ticket covers at most 192 fixed-width branches and six machine comparisons.
        credit.nominal(1)?;
        let Some(mut cursor) = self.root else { return Ok(None) };
        loop {
            match &self.entries[cursor] {
                Entry::Leaf(found, value) => return Ok((found == key).then_some(*value)),
                Entry::Branch { bit: at, children } => cursor = children[bit(key, *at)],
            }
        }
    }
    pub(super) fn insert(
        &mut self,
        key: Key,
        value: u32,
        credit: &mut Budget,
    ) -> Result<(), Failure> {
        credit.nominal(3)?;
        self.entries.try_reserve(2).map_err(|_| Failure::AllocationFailure)?;
        let leaf = self.entries.len();
        let Some(mut cursor) = self.root else {
            self.entries.push(Entry::Leaf(key, value));
            self.root = Some(leaf);
            return Ok(());
        };
        let found = loop {
            match &self.entries[cursor] {
                Entry::Leaf(found, _) => break *found,
                Entry::Branch { bit: at, children } => cursor = children[bit(&key, *at)],
            }
        };
        let difference = found
            .iter()
            .zip(&key)
            .enumerate()
            .find_map(|(lane, (a, b))| {
                (a != b).then(|| lane * 32 + (a ^ b).leading_zeros() as usize)
            })
            .ok_or(Failure::InternalFailure)?;
        let mut parent = None;
        cursor = self.root.ok_or(Failure::InternalFailure)?;
        while let Entry::Branch { bit: at, children } = &self.entries[cursor] {
            if *at >= difference {
                break;
            }
            let side = bit(&key, *at);
            parent = Some((cursor, side));
            cursor = children[side];
        }
        self.entries.push(Entry::Leaf(key, value));
        let mut children = [cursor; 2];
        children[bit(&key, difference)] = leaf;
        let branch = self.entries.len();
        self.entries.push(Entry::Branch { bit: difference, children });
        if let Some((parent, side)) = parent {
            let Entry::Branch { children, .. } = &mut self.entries[parent] else {
                return Err(Failure::InternalFailure);
            };
            children[side] = branch;
        } else {
            self.root = Some(branch);
        }
        Ok(())
    }
}
