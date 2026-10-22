//! Encoded data audit controls do not claim C compiler provenance or host enforcement.
use super::{approval, support};
use sha2::{Digest as _, Sha256};
use zryna_package::SourceTrustMode;

#[test]
fn independent_output_size_and_hash_expectations_reject_altered_data_and_recover() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    let bytes = support::encoded();
    let approved = approval::admit(f.plan(), SourceTrustMode::TrustedNative, f.approval())
        .expect("input expectations only");
    let captured = approval::capture_output(&approved, &bytes)
        .expect("independently encoded closed data object");
    assert_eq!(captured.bytes(), bytes);
    captured.check_binding(&f.requirements).expect("original issuer");
    let mut changed = bytes.clone();
    changed[0] ^= 1;
    assert_eq!(
        approval::capture_output(&approved, &changed)
            .expect_err("changed output against independent digest")
            .reason(),
        "ZRYNA-C4102"
    );
    for mutation in 0..2 {
        let mut operator = f.approval();
        if mutation == 0 {
            operator.output_bytes += 1;
        } else {
            operator.output_sha256 = [0; 32];
        }
        let approved = approval::admit(f.plan(), SourceTrustMode::TrustedNative, operator)
            .expect("approval data does not bypass object audit");
        assert_eq!(
            approval::capture_output(&approved, &bytes)
                .expect_err("size/hash substitution")
                .reason(),
            "ZRYNA-C4102"
        );
    }
    let other = support::Fixture::new();
    assert!(captured.check_binding(&other.requirements).is_err());
    approval::capture_output(&approved, &bytes).expect("pristine audit recovers");
    assert_eq!(f.snapshot(), before);
}

#[test]
fn complete_foreign_audit_rejects_reserved_and_casefolded_dependency_names_with_matching_bytes() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    for name in ["sqlite3_malloc", "ZrYnA_private_escape", "SQLITE_FIXTURE_COPY_BYTES"] {
        let bytes = support::encoded_with_dependencies(&[name]);
        let mut operator = f.approval();
        operator.output_bytes = bytes.len();
        operator.output_sha256 = Sha256::digest(&bytes).into();
        operator.dependency_symbols = vec![name.into()];
        let approved = approval::admit(f.plan(), SourceTrustMode::TrustedNative, operator)
            .expect("encoded expectation data alone is not object or execution approval");
        let result = approval::capture_output(&approved, &bytes);
        if name == "sqlite3_malloc" {
            assert_eq!(
                result
                    .expect("valid ordinary dependency control")
                    .dependencies()
                    .collect::<Vec<_>>(),
                [name]
            );
        } else {
            assert_eq!(
                result.expect_err("exact matching bytes still violate namespace policy").reason(),
                "ZRYNA-C4104"
            );
        }
    }
    assert_eq!(f.snapshot(), before);
}
