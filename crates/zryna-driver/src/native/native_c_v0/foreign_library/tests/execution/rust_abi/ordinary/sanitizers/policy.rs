//! Independent closed hook/layout/material expectations; no product pins or origin imports.
use super::*;
use object::{Object as _, ObjectSymbol as _};
mod owned;
pub(super) use owned::{RuntimeCase, RuntimeProfile};
struct Provider {
    path: &'static str,
    bytes: usize,
    sha256: [u8; 32],
}
pub(super) struct Bindings {
    case: Case,
    pub(super) hooks: &'static [&'static str],
    mnemonics: &'static [&'static str],
    undefined_symbols: &'static [&'static str],
    providers: &'static [Provider],
}
// INITIAL correction: actual independent hooks/layout/providers are still UNOBSERVED.
// No runtime authority can be reconstructed from serialized products or this unavailable value.
pub(super) fn reviewed(_case: Case) -> Option<&'static Bindings> {
    None
}
impl Bindings {
    pub(super) fn materials(&self, case: Case) -> Result<(), Diagnostic> {
        if self.case != case || self.providers.is_empty() || self.providers.len() > 64 {
            return Err(rejected());
        }
        let mut total = 0_usize;
        let mut paths = std::collections::BTreeSet::new();
        for provider in self.providers {
            total = total.checked_add(provider.bytes).ok_or_else(rejected)?;
            if provider.bytes == 0
                || total > 32 * 1024 * 1024
                || !paths.insert(provider.path)
                || !Path::new(provider.path).is_absolute()
            {
                return Err(rejected());
            }
            let bytes = material::read(Path::new(provider.path), provider.bytes)?;
            if bytes.len() != provider.bytes || sha(&bytes) != provider.sha256 {
                return Err(rejected());
            }
        }
        Ok(())
    }
    pub(super) fn layout(&self, bytes: &[u8], mnemonics: &[&str]) -> Result<(), Diagnostic> {
        if self.hooks.is_empty() || self.mnemonics.is_empty() || mnemonics != self.mnemonics {
            return Err(rejected());
        }
        let mut names = std::collections::BTreeSet::new();
        for name in self.hooks {
            if name.is_empty()
                || ["add", "sum_bytes", "__wrap_sum_bytes"].contains(name)
                || !names.insert(name)
            {
                return Err(rejected());
            }
        }
        let file = machine::parse(bytes)?;
        let mut undefined = file
            .dynamic_symbols()
            .filter(object::ObjectSymbol::is_undefined)
            .map(|s| s.name().map(str::to_owned).map_err(|_| rejected()))
            .collect::<Result<Vec<_>, _>>()?;
        undefined.retain(|s| !s.is_empty());
        undefined.sort();
        let expected = self.undefined_symbols.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        if undefined != expected || expected.is_empty() {
            return Err(rejected());
        }
        Ok(())
    }
}
