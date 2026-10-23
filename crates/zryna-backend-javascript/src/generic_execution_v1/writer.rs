//! Count the complete artifact before bounded reservation or publication.
use super::emission_budget;
use std::fmt::Write;
use zryna_diagnostics::Diagnostic;
pub(super) struct Writer {
    source: Option<String>,
    bytes: usize,
    limit: usize,
}
impl Writer {
    pub(super) const fn counter(limit: usize) -> Self {
        Self { source: None, bytes: 0, limit }
    }
    pub(super) fn output(limit: usize, bytes: usize) -> Result<Self, Diagnostic> {
        let mut source = String::new();
        source.try_reserve_exact(bytes).map_err(|_| emission_budget())?;
        Ok(Self { source: Some(source), bytes: 0, limit })
    }
    pub(super) const fn bytes(&self) -> usize {
        self.bytes
    }
    pub(super) fn finish(self) -> String {
        self.source.expect("output writer")
    }
    pub(super) fn text(&mut self, text: &str) -> Result<(), Diagnostic> {
        self.write_str(text).map_err(|_| emission_budget())
    }
    pub(super) fn format(&mut self, args: std::fmt::Arguments<'_>) -> Result<(), Diagnostic> {
        self.write_fmt(args).map_err(|_| emission_budget())
    }
}
impl Write for Writer {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let next = self
            .bytes
            .checked_add(text.len())
            .filter(|n| *n <= self.limit)
            .ok_or(std::fmt::Error)?;
        if let Some(source) = &mut self.source {
            source.try_reserve(text.len()).map_err(|_| std::fmt::Error)?;
            source.push_str(text);
        }
        self.bytes = next;
        Ok(())
    }
}
