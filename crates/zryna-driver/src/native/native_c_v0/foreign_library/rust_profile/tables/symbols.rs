//! No relocation, dependency or undeclared definition in the measured function-section profile.

use super::super::super::audit::{audit_error, identifier, reserved};
use super::{Section, name, number};
use std::collections::{BTreeMap, BTreeSet};
use zryna_diagnostics::Diagnostic;

pub(super) fn check(
    records: &[u8],
    strings: &[u8],
    sections: &BTreeMap<&str, Section<'_>>,
    expected: &BTreeSet<String>,
) -> Result<(), Diagnostic> {
    if !records.len().is_multiple_of(24)
        || records.len() / 24 != expected.len() + 2
        || records.get(..24).is_none_or(|null| null.iter().any(|byte| *byte != 0))
    {
        return Err(audit_error());
    }
    let mut definitions = BTreeSet::new();
    for (ordinal, record) in records.chunks_exact(24).enumerate().skip(1) {
        let symbol = name(strings, number(record, 0, 4)?)?;
        let kind = record[4];
        let visibility = record[5];
        let section_index = number(record, 6, 2)?;
        let value = number(record, 8, 8)?;
        let size = number(record, 16, 8)?;
        if ordinal == 1 {
            if kind != 4 || visibility != 0 || section_index != 0xfff1 || value != 0 || size != 0 {
                return Err(audit_error());
            }
        } else {
            let section = sections
                .values()
                .find(|section| u64::try_from(section.ordinal).ok() == Some(section_index))
                .ok_or_else(audit_error)?;
            if kind != 0x12
                || visibility != 0
                || value != 0
                || size == 0
                || size != section.size
                || section.name.strip_prefix(".text.") != Some(symbol)
                || !identifier(symbol)
                || reserved(symbol)
                || !expected.contains(symbol)
                || !definitions.insert(symbol.to_owned())
            {
                return Err(audit_error());
            }
        }
    }
    if &definitions != expected {
        return Err(audit_error());
    }
    Ok(())
}
