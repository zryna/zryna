//! Allocation-atomic primitive reader; all vector credit is charged before reserve.
use super::{Failure, MAX_CHILDREN, invalid, limits::Totals, reserve};
pub(super) struct Reader<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) cursor: usize,
    pub(super) children: usize,
    pub(super) totals: Totals,
    #[cfg(test)]
    pub(super) nonempty_reservations: usize,
    #[cfg(test)]
    pub(super) elements_visited: usize,
}
impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            cursor: 0,
            children: 0,
            totals: Totals::default(),
            #[cfg(test)]
            nonempty_reservations: 0,
            #[cfg(test)]
            elements_visited: 0,
        }
    }
    pub(super) fn take(&mut self, length: usize) -> Result<&'a [u8], Failure> {
        let end = self.cursor.checked_add(length).ok_or_else(invalid)?;
        let bytes = self.bytes.get(self.cursor..end).ok_or_else(invalid)?;
        self.cursor = end;
        Ok(bytes)
    }
    pub(super) fn byte(&mut self) -> Result<u8, Failure> {
        Ok(self.take(1)?[0])
    }
    pub(super) fn u32(&mut self) -> Result<u32, Failure> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| invalid())?))
    }
    pub(super) fn boolean(&mut self) -> Result<bool, Failure> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid()),
        }
    }
    pub(super) fn option<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, Failure>,
    ) -> Result<Option<T>, Failure> {
        if self.boolean()? { Ok(Some(read(self)?)) } else { Ok(None) }
    }
    pub(super) fn array(&mut self) -> Result<[u8; 32], Failure> {
        self.take(32)?.try_into().map_err(|_| invalid())
    }
    pub(super) fn blob_slice(&mut self, limit: usize) -> Result<&'a [u8], Failure> {
        let length = usize::try_from(self.u32()?).map_err(|_| invalid())?;
        super::limits::bound(length, limit)?;
        self.take(length)
    }
    pub(super) fn vector<T>(
        &mut self,
        limit: usize,
        minimum: usize,
        charge: impl FnOnce(&mut Totals, usize) -> Result<(), Failure>,
        mut read: impl FnMut(&mut Self) -> Result<T, Failure>,
    ) -> Result<Vec<T>, Failure> {
        let count = usize::try_from(self.u32()?).map_err(|_| invalid())?;
        super::limits::bound(count, limit)?;
        let children = self
            .children
            .checked_add(count)
            .filter(|sum| *sum <= MAX_CHILDREN)
            .ok_or_else(super::limits::exceeded)?;
        if count.checked_mul(minimum).is_none_or(|bytes| bytes > self.bytes.len() - self.cursor) {
            return Err(invalid());
        }
        charge(&mut self.totals, count)?;
        self.children = children;
        #[cfg(test)]
        {
            self.nonempty_reservations += usize::from(count > 0);
        }
        let mut values = reserve(count)?;
        for _ in 0..count {
            #[cfg(test)]
            {
                self.elements_visited += 1;
            }
            values.push(read(self)?);
        }
        Ok(values)
    }
    pub(super) fn plain<T>(
        &mut self,
        limit: usize,
        minimum: usize,
        read: impl FnMut(&mut Self) -> Result<T, Failure>,
    ) -> Result<Vec<T>, Failure> {
        self.vector(limit, minimum, |_, _| Ok(()), read)
    }
}
