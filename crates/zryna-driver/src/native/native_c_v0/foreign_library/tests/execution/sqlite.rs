//! Required Linux generated `SQLite` observations, outside production recipe admission.
use super::*;
mod harness;
mod provider;
mod source_input;
mod stage;
use harness::{Variant, observe};

const SANITIZERS: [&str; 3] =
    ["-fsanitize=address,undefined", "-fno-sanitize-recover=all", "-fno-omit-frame-pointer"];
fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().fold(String::with_capacity(64), |mut text, byte| {
        write!(text, "{byte:02x}").expect("hex digest");
        text
    })
}
fn succeeds(variant: Variant, scenario: u8, sanitized: bool) {
    assert!(observe(variant, scenario, sanitized).status.success());
}
fn assertion_fails(variant: Variant, scenario: u8, sanitized: bool, marker: &str) {
    let output = observe(variant, scenario, sanitized);
    assert_eq!(output.status.signal(), Some(libc::SIGABRT));
    assert!(String::from_utf8_lossy(&output.stderr).contains(marker));
}

#[test]
fn sqlite_generated_copy_uses_distinct_private_storage_and_recovers() {
    for sanitized in [false, true] {
        succeeds(Variant::Pristine, 0, sanitized);
    }
    for (scenario, required) in [(5, "heap-buffer-overflow"), (6, "signed integer overflow")] {
        let uninstrumented = observe(Variant::Pristine, scenario, false);
        assert!(
            !String::from_utf8_lossy(&uninstrumented.stderr).contains(required),
            "uninstrumented generic success/crash cannot earn sanitizer diagnostic credit"
        );
        let instrumented = observe(Variant::Pristine, scenario, true);
        assert!(!instrumented.status.success());
        let stderr = String::from_utf8_lossy(&instrumented.stderr);
        assert!(stderr.contains(required));
        if scenario == 5 {
            assert!(stderr.contains("AddressSanitizer"));
        }
        succeeds(Variant::Pristine, 0, true);
    }
}

#[test]
fn sqlite_generated_private_copy_failure_releases_sqlite_owner() {
    for sanitized in [false, true] {
        succeeds(Variant::Pristine, 1, sanitized);
        assertion_fails(Variant::Pristine, 2, sanitized, "private-copy failure must trap");
        succeeds(Variant::Pristine, 0, sanitized);
    }
}

#[test]
fn sqlite_generated_malformed_lengths_release_matching_sqlite_allocation() {
    for sanitized in [false, true] {
        for count in [0, 2, 4097] {
            succeeds(Variant::WrongCount(count), 3, sanitized);
        }
        assertion_fails(
            Variant::Pristine,
            3,
            sanitized,
            "malformed SQLite result must be rejected",
        );
        succeeds(Variant::Pristine, 0, sanitized);
    }
}

#[test]
fn sqlite_generated_missing_release_oracle_fails_then_pristine_recovers() {
    for sanitized in [false, true] {
        assertion_fails(
            Variant::MissingRelease,
            4,
            sanitized,
            "selected SQLite owner must be released exactly once",
        );
        succeeds(Variant::Pristine, 0, sanitized);
    }
}
