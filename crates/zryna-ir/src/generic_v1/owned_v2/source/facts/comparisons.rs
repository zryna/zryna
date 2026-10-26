//! V4 comparisons pay the candidate and its bytes before either is examined.
use super::{Budget, Closed, Facts, Failure};
use crate::generic_v1::{raw, reject, source_types::type_id};
use std::cmp::Ordering;

impl Facts<'_, '_> {
    pub(in super::super) fn equal_type(
        &self,
        left: &Closed,
        right: &Closed,
        credit: &mut Budget,
    ) -> Result<bool, Failure> {
        if matches!(self, Self::V4(_) | Self::V5 { .. }) {
            credit.nominal(1)?;
            if let (Closed::Stored(a), Closed::Stored(b))
            | (Closed::Borrow(a, _), Closed::Borrow(b, _)) = (left, right)
            {
                bytes(a.len(), b.len(), credit)?;
            }
        }
        Ok(left == right)
    }
    pub(in super::super) fn equal_payload(
        &self,
        left: Option<&Closed>,
        right: Option<&Closed>,
        credit: &mut Budget,
    ) -> Result<bool, Failure> {
        if let (Some(left), Some(right)) = (left, right) {
            return self.equal_type(left, right, credit);
        }
        Ok(left == right)
    }
    pub(in super::super) fn stored_key(
        &self,
        ty: &Closed,
        key: &[u8],
        credit: &mut Budget,
    ) -> Result<bool, Failure> {
        let Closed::Stored(expected) = ty else { return Ok(false) };
        if matches!(self, Self::V4(_) | Self::V5 { .. }) {
            bytes(expected.len(), key.len(), credit)?;
        }
        Ok(expected == key)
    }
    pub(in super::super) fn type_id(
        &self,
        program: &raw::Program,
        ty: Closed,
        credit: &mut Budget,
    ) -> Result<raw::Type, Failure> {
        if matches!(self, Self::Legacy(_)) {
            return type_id(program, ty);
        }
        let stored = |key: &[u8], credit: &mut Budget| {
            index(&program.type_keys, key, credit)?.ok_or_else(|| {
                reject("substituted original type is absent from the complete universe")
            })
        };
        match ty {
            Closed::Stored(key) => Ok(raw::Type::Stored(stored(&key, credit)?)),
            Closed::Borrow(key, exclusive) => {
                Ok(raw::Type::Borrow { referent: stored(&key, credit)?, exclusive })
            }
            Closed::Unit => Ok(raw::Type::Unit),
        }
    }
    pub(in super::super) fn variant(
        &self,
        variants: &[(&str, Option<Closed>)],
        name: &str,
        credit: &mut Budget,
    ) -> Result<Option<usize>, Failure> {
        if matches!(self, Self::Legacy(_)) {
            return Ok(variants.iter().position(|(candidate, _)| *candidate == name));
        }
        select(variants, name, false, |row| row.0, credit)
    }
}
pub(in super::super) fn select<T>(
    rows: &[T],
    query: &str,
    unique: bool,
    name: impl Fn(&T) -> &str,
    credit: &mut Budget,
) -> Result<Option<usize>, Failure> {
    let mut found = None;
    for (ordinal, row) in rows.iter().enumerate() {
        credit.nominal(1)?;
        let candidate = name(row);
        bytes(candidate.len(), query.len(), credit)?;
        if candidate == query {
            if found.replace(ordinal).is_some() {
                return Err(reject("duplicate/foreign enum match arm"));
            }
            if !unique {
                break;
            }
        }
    }
    Ok(found)
}
fn index(keys: &[Vec<u8>], query: &[u8], credit: &mut Budget) -> Result<Option<u32>, Failure> {
    let (mut first, mut end) = (0, keys.len());
    while first < end {
        let midpoint = first + (end - first) / 2;
        credit.nominal(1)?;
        let candidate = &keys[midpoint];
        bytes(candidate.len(), query.len(), credit)?;
        match candidate.as_slice().cmp(query) {
            Ordering::Less => first = midpoint + 1,
            Ordering::Greater => end = midpoint,
            Ordering::Equal => {
                return Ok(Some(u32::try_from(midpoint).map_err(|_| Failure::InternalFailure)?));
            }
        }
    }
    Ok(None)
}
fn bytes(left: usize, right: usize, credit: &mut Budget) -> Result<(), Failure> {
    credit.nominal(left.min(right).checked_add(1).ok_or(Failure::InternalFailure)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    const LIMIT: usize = super::super::super::super::plan::MAX_OWNER_UNITS;

    #[test]
    fn long_variant_names_pay_each_visit_and_comparison_before_first_extra() {
        let names = ["LongCommonPrefixX", "LongCommonPrefixY"];
        let query = "LongCommonPrefixY";
        let per_row = 1 + query.len() + 1;
        let cost = 2 * per_row;
        let mut exact = Budget::original(LIMIT - cost);
        assert_eq!(select(&names, query, false, |s| *s, &mut exact).expect("exact"), Some(1));
        assert_eq!(exact.nominal_used(), LIMIT);
        let mut short = Budget::original(LIMIT - cost + 1);
        assert!(select(&names, query, false, |s| *s, &mut short).is_err());
        assert_eq!(short.nominal_used(), LIMIT - cost + 1 + per_row + 1);
        let mut retry = Budget::default();
        assert_eq!(
            select(&names, query, true, |s| *s, &mut retry).expect("pristine unique match"),
            Some(1)
        );
        assert_eq!(retry.nominal_used(), cost);
    }

    #[test]
    fn duplicate_arm_scan_pays_the_second_exact_name_before_rejection() {
        let names = ["RepeatedLongVariant", "RepeatedLongVariant"];
        let mut credit = Budget::default();
        assert!(select(&names, names[0], true, |s| *s, &mut credit).is_err());
        assert_eq!(credit.nominal_used(), 2 * (names[0].len() + 2));
    }

    #[test]
    fn type_key_probe_prepays_row_and_all_key_bytes() {
        let keys = vec![vec![1; 128]];
        let cost = 130;
        let mut exact = Budget::original(LIMIT - cost);
        assert_eq!(index(&keys, &keys[0], &mut exact).expect("exact probe"), Some(0));
        assert_eq!(exact.nominal_used(), LIMIT);
        let mut short = Budget::original(LIMIT - cost + 1);
        assert!(index(&keys, &keys[0], &mut short).is_err());
        assert_eq!(short.nominal_used(), LIMIT - cost + 2, "only row visit completed");
        let mut exhausted = Budget::original(LIMIT);
        assert!(index(&keys, &keys[0], &mut exhausted).is_err());
        assert_eq!(exhausted.nominal_used(), LIMIT, "candidate was never inspected");
    }

    #[test]
    fn legacy_variant_and_type_index_delegates_add_no_credit() {
        let facts = Facts::Legacy(super::super::type_facts::SourceTypeFacts::default());
        let mut credit = Budget::original(LIMIT);
        let variants = [("LongLegacyVariant", None)];
        assert_eq!(
            facts.variant(&variants, "LongLegacyVariant", &mut credit).expect("old scan"),
            Some(0)
        );
        // Raw comparison control only: this table does not issue executable authority.
        let program = raw::Program {
            modules: vec![],
            declarations: vec![],
            type_keys: vec![vec![1]],
            universe: [0; 32],
            linear32: [0; 32],
            linux_x86_64: [0; 32],
            functions: vec![],
        };
        assert_eq!(
            facts.type_id(&program, Closed::Stored(vec![1]), &mut credit).expect("old type lookup"),
            raw::Type::Stored(0)
        );
        assert_eq!(credit.nominal_used(), LIMIT);
    }
}
