//! Private whole-component guard evidence; custom-padded cores are not public artifacts.

use super::*;
use std::borrow::Cow;
use wasm_encoder::{CustomSection, Encode};

const LIMIT: usize = 1_048_576;

#[test]
fn private_component_audit_accepts1048576_and_rejects1048577_but_language_rejects_padding() {
    let candidate = Candidate::new("pure-entry");
    let original = candidate.artifact.bytes();
    assert!(original.len() < LIMIT);
    candidate.audit(original).expect("original source-bound artifact admitted");
    super::super::super::language_audit::audit(
        candidate.artifact.language(),
        candidate.artifact.program(),
    )
    .expect("original source-bound language independently admitted");
    let mut padding = LIMIT - original.len();
    let mut selected = None;
    // Only section-size LEB widths vary around this target. Correct the observed
    // encoded byte length, rather than assuming a fixed section-header cost.
    for _ in 0..4 {
        let mut language = candidate.artifact.language().to_vec();
        language.push(0); // Core custom-section id; Encode writes size/name/data.
        CustomSection {
            name: Cow::Borrowed("private-boundary-padding"),
            data: Cow::Owned(vec![0; padding]),
        }
        .encode(&mut language);
        let mut replaced = false;
        let bytes = rewrite(original, |chunk, component| {
            if chunk.id == 1 && chunk.data == candidate.artifact.language() {
                assert!(!replaced, "exactly one retained language core");
                component.section(&RawSection { id: 1, data: &language });
                replaced = true;
                true
            } else {
                false
            }
        });
        assert!(replaced);
        if bytes.len() == LIMIT {
            selected = Some((bytes, language));
            break;
        }
        padding = if bytes.len() > LIMIT {
            padding.checked_sub(bytes.len() - LIMIT).expect("bounded header correction")
        } else {
            padding.checked_add(LIMIT - bytes.len()).expect("bounded header correction")
        };
    }
    let (bytes, language) = selected.expect("actual encoded component reaches exact byte boundary");
    valid(&bytes);
    assert_eq!(bytes.len(), LIMIT);
    // This independent prerequisite explicitly excludes source/artifact admission.
    // The factory audits language before composing/auditing its component.
    assert!(language.len() < LIMIT, "language length guard is not the padding rejection");
    let error = super::super::super::language_audit::audit(&language, candidate.artifact.program())
        .expect_err("private custom padding remains outside the public artifact factory");
    assert_eq!(error.code(), "ZRYNA-W4103");
    assert_eq!(
        super::super::super::shell::encode(
            &candidate.world,
            candidate.artifact.storage(),
            &language
        )
        .expect("unchanged shell exact byte policy"),
        bytes
    );
    super::super::audit(&bytes, candidate.artifact.storage(), &language, &candidate.world)
        .expect("unchanged complete component guard admits exact size and exact retained cores");
    let mut extra = bytes;
    extra.push(0);
    assert_eq!(extra.len(), LIMIT + 1);
    // The explicit byte guard precedes parsing: first-extra rejection does not
    // claim a valid canonical or source-admitted component of this size.
    let error =
        super::super::audit(&extra, candidate.artifact.storage(), &language, &candidate.world)
            .expect_err("unchanged owning byte guard rejects first extra");
    assert_eq!(error.code(), "ZRYNA-W4104");
    let mut extra_language = language;
    extra_language.push(0);
    let reconstructed_extra = rewrite(original, |chunk, component| {
        if chunk.id == 1 && chunk.data == candidate.artifact.language() {
            component.section(&RawSection { id: 1, data: &extra_language });
            true
        } else {
            false
        }
    });
    assert_eq!(reconstructed_extra.len(), LIMIT + 1);
    assert_eq!(
        super::super::super::shell::encode(
            &candidate.world,
            candidate.artifact.storage(),
            &extra_language,
        )
        .expect_err("unchanged shell byte guard rejects first extra")
        .code(),
        "ZRYNA-W4102"
    );
    candidate.audit(original).expect("retained source-bound component recovers");
}
