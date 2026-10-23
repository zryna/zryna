//! Private v4 name lookup over the original authenticated folded-name inventory.
use super::{Failure, Originals, Target, budget, folded, reject};
use std::cmp::Ordering;

impl Originals<'_> {
    /// Charge each candidate visit and complete comparison before doing that work.
    /// The existing inventory already owns import binding and folded uniqueness.
    pub(in super::super) fn resolve_v4(
        &self,
        module: u32,
        text: &str,
        charge: &mut impl FnMut(usize) -> Result<(), Failure>,
    ) -> Result<Target, Failure> {
        let names =
            self.names.get(module as usize).ok_or_else(|| reject("unknown source module"))?;
        let mut first = 0;
        let mut end = names.len();
        while first < end {
            let index = first + (end - first) / 2;
            charge(1)?;
            let name = &names[index];
            comparison(name.text, text, charge)?;
            match folded(name.text, text) {
                Ordering::Less => first = index + 1,
                Ordering::Greater => end = index,
                Ordering::Equal => {
                    comparison(name.text, text, charge)?;
                    if name.text != text {
                        return Err(reject(
                            "source name differs from its exact declared/imported spelling",
                        ));
                    }
                    return Ok(name.target);
                }
            }
        }
        Err(reject("source name has no exact original declaration binding"))
    }
}

fn comparison(
    left: &str,
    right: &str,
    charge: &mut impl FnMut(usize) -> Result<(), Failure>,
) -> Result<(), Failure> {
    charge(
        left.len()
            .min(right.len())
            .checked_add(1)
            .ok_or_else(|| budget("original name comparison credit overflow"))?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_v1::source::Name;

    fn table<'a>(names: &'a [&'a str]) -> Originals<'a> {
        let names = names
            .iter()
            .enumerate()
            .map(|(index, text)| Name {
                text,
                target: Target::Data(7, u32::try_from(index).expect("small test inventory")),
            })
            .collect();
        Originals { units: &[], names: vec![names] }
    }

    fn lookup(
        originals: &Originals<'_>,
        name: &str,
        ceiling: usize,
    ) -> (Result<Target, Failure>, usize, Vec<usize>) {
        let mut used = 0usize;
        let mut charges = Vec::new();
        let result = originals.resolve_v4(0, name, &mut |units| {
            let next = used
                .checked_add(units)
                .filter(|next| *next <= ceiling)
                .ok_or_else(|| budget("test original name credit ceiling"))?;
            used = next;
            charges.push(units);
            Ok(())
        });
        (result, used, charges)
    }

    #[test]
    fn exact_names_and_alias_targets_match_the_existing_resolver() {
        let originals = table(&["Alpha", "importedAlias", "Zulu"]);
        for name in ["Alpha", "importedAlias", "Zulu"] {
            assert_eq!(
                lookup(&originals, name, usize::MAX).0.expect("v4 exact lookup"),
                originals.resolve(0, name).expect("legacy exact lookup"),
            );
        }
        for name in ["alpha", "ImportedAlias", "Missing", ""] {
            assert!(lookup(&originals, name, usize::MAX).0.is_err());
            assert!(originals.resolve(0, name).is_err());
        }
    }

    #[test]
    fn common_prefix_and_exact_spelling_comparisons_pay_all_bytes() {
        let text = format!("{}Tail", "LongPrefix".repeat(32));
        let names = [text.as_str()];
        let originals = table(&names);
        let (result, used, charges) = lookup(&originals, &text, usize::MAX);
        assert_eq!(result.expect("complete exact lookup"), Target::Data(7, 0));
        assert_eq!(used, 1 + 2 * (text.len() + 1));
        assert_eq!(charges, [1, text.len() + 1, text.len() + 1]);
        let wrong = text.to_ascii_lowercase();
        let (result, used, charges) = lookup(&originals, &wrong, usize::MAX);
        assert!(result.is_err(), "folded equality cannot grant exact spelling");
        assert_eq!(used, 1 + 2 * (text.len() + 1));
        assert_eq!(charges, [1, text.len() + 1, text.len() + 1]);
    }

    #[test]
    fn exhaustion_stops_before_the_next_candidate_or_comparison() {
        let originals = table(&["Bound"]);
        for (ceiling, expected) in [(0, vec![]), (1, vec![1]), (7, vec![1, 6])] {
            let (result, used, charges) = lookup(&originals, "Bound", ceiling);
            assert!(result.is_err());
            assert_eq!(used, expected.iter().sum::<usize>());
            assert_eq!(charges, expected);
        }
        let (result, used, _) = lookup(&originals, "Bound", 13);
        assert_eq!(result.expect("exact full comparison ceiling"), Target::Data(7, 0));
        assert_eq!(used, 13);
    }

    #[test]
    fn empty_and_unknown_modules_keep_the_existing_binding_errors() {
        let originals = table(&[]);
        let (result, used, charges) = lookup(&originals, "Absent", 0);
        assert!(result.is_err());
        assert_eq!(used, 0);
        assert!(charges.is_empty());
        let mut charges = 0;
        assert!(
            originals
                .resolve_v4(1, "Absent", &mut |_| {
                    charges += 1;
                    Ok(())
                })
                .is_err()
        );
        assert_eq!(charges, 0);
    }
}
