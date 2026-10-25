//! Count the complete artifact before bounded reservation or publication.
use super::emission_budget;
use std::fmt::Write;
use zryna_diagnostics::Diagnostic;
pub(super) struct Writer {
    source: Option<String>,
    bytes: usize,
    limit: usize,
    invocation: bool,
    template_work: usize,
}
impl Writer {
    pub(super) const fn counter(limit: usize) -> Self {
        Self { source: None, bytes: 0, limit, invocation: false, template_work: 0 }
    }
    pub(super) fn output(limit: usize, bytes: usize) -> Result<Self, Diagnostic> {
        let mut source = String::new();
        source.try_reserve_exact(bytes).map_err(|_| emission_budget())?;
        Ok(Self { source: Some(source), bytes: 0, limit, invocation: false, template_work: 0 })
    }
    pub(super) fn invocation(&mut self) {
        self.invocation = true;
    }
    pub(super) fn throws_source(
        &mut self,
        identity: zryna_ir::generic_v1::execution_v1::raw::TrapIdentity,
    ) -> Result<(), Diagnostic> {
        let invocation = self.invocation;
        super::invocation::traps::emit(self, identity, invocation)
    }
    // Only explicitly enumerated immutable compiler fragments may enter this path.
    // User strings use format/text and are never searched or replaced.
    pub(super) fn trusted(&mut self, text: &'static str) -> Result<(), Diagnostic> {
        if !self.invocation {
            return self.text(text);
        }
        template_work(&mut self.template_work, text.len())?;
        super::invocation::traps::fragment(self, text)
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

// Eight byte-visits per original static byte fund disjoint prefix/delimiter scans,
// five exact identity comparisons and one copy, independently of output byte limit.
pub(super) fn template_work(work: &mut usize, bytes: usize) -> Result<(), Diagnostic> {
    let units = bytes.checked_mul(8).ok_or_else(emission_budget)?;
    let next = work
        .checked_add(units)
        .filter(|n| *n <= crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES)
        .ok_or_else(emission_budget)?;
    *work = next;
    Ok(())
}
