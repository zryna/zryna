//! Charge each exact original field-name comparison before examining its bytes.
use super::super::{Budget, Closed};
use crate::generic_v1::Failure;

pub(super) fn ordinal(
    fields: &[(&str, Closed)],
    name: &str,
    credit: &mut Budget,
) -> Result<Option<usize>, Failure> {
    for (ordinal, (candidate, _)) in fields.iter().enumerate() {
        let work =
            candidate.len().min(name.len()).checked_add(1).ok_or(Failure::InternalFailure)?;
        credit.nominal(work)?;
        if *candidate == name {
            return Ok(Some(ordinal));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_prefix_name_scans_exhaust_existing_credit_before_next_comparison() {
        let limit = super::super::super::super::plan::MAX_OWNER_UNITS;
        let fields = [("aaaX", Closed::Unit), ("aaaY", Closed::Unit)];
        // Each absent-name scan compares two four-byte names: exactly ten work units.
        let mut credit = Budget::original(limit % 10);
        for _ in 0..limit / 10 {
            assert_eq!(ordinal(&fields, "aaaZ", &mut credit).expect("bounded scan"), None);
        }
        assert_eq!(credit.nominal_used(), limit);
        assert!(ordinal(&fields, "aaaZ", &mut credit).is_err());
        assert_eq!(credit.nominal_used(), limit, "failure charges no unperformed comparison");
        let mut fresh = Budget::default();
        assert_eq!(ordinal(&fields, "aaaY", &mut fresh).expect("exact name"), Some(1));
        assert_eq!(fresh.nominal_used(), 10);
    }
}
