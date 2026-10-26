//! Bounded primitive writer; output and child credit precede every reserve.
use super::{Failure, MAX_BYTES, MAX_CHILDREN, invalid};
pub(super) struct Writer {
    pub(super) bytes: Vec<u8>,
    children: usize,
    length: usize,
    count_only: bool,
}
impl Writer {
    pub(super) const fn new() -> Self {
        Self { bytes: Vec::new(), children: 0, length: 0, count_only: false }
    }
    pub(super) const fn counter() -> Self {
        Self { bytes: Vec::new(), children: 0, length: 0, count_only: true }
    }
    pub(super) const fn length(&self) -> usize {
        self.length
    }
    pub(super) fn bytes(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        let length = self.length.checked_add(bytes.len()).ok_or_else(super::limits::exceeded)?;
        super::limits::bound(length, MAX_BYTES)?;
        if self.count_only {
            self.length = length;
            return Ok(());
        }
        self.bytes.try_reserve(bytes.len()).map_err(|_| Failure::AllocationFailure)?;
        self.bytes.extend_from_slice(bytes);
        self.length = length;
        Ok(())
    }
    pub(super) fn byte(&mut self, byte: u8) -> Result<(), Failure> {
        self.bytes(&[byte])
    }
    pub(super) fn u32(&mut self, value: u32) -> Result<(), Failure> {
        self.bytes(&value.to_le_bytes())
    }
    pub(super) fn boolean(&mut self, value: bool) -> Result<(), Failure> {
        self.byte(u8::from(value))
    }
    pub(super) fn option<T>(
        &mut self,
        value: Option<&T>,
        write: impl FnOnce(&mut Self, &T) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        self.boolean(value.is_some())?;
        if let Some(value) = value {
            write(self, value)?;
        }
        Ok(())
    }
    pub(super) fn optional_id(&mut self, value: Option<u32>) -> Result<(), Failure> {
        self.option(value.as_ref(), |writer, id| writer.u32(*id))
    }
    pub(super) fn blob(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        self.u32(u32::try_from(bytes.len()).map_err(|_| invalid())?)?;
        self.bytes(bytes)
    }
    pub(super) fn count(&mut self, count: usize) -> Result<(), Failure> {
        self.children = self
            .children
            .checked_add(count)
            .filter(|sum| *sum <= MAX_CHILDREN)
            .ok_or_else(super::limits::exceeded)?;
        self.u32(u32::try_from(count).map_err(|_| invalid())?)
    }
    pub(super) fn vector<T>(
        &mut self,
        values: &[T],
        mut write: impl FnMut(&mut Self, &T) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        self.count(values.len())?;
        for value in values {
            write(self, value)?;
        }
        Ok(())
    }
    pub(super) fn ids(&mut self, values: &[u32]) -> Result<(), Failure> {
        self.vector(values, |writer, id| writer.u32(*id))
    }
}
