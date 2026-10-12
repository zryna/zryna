use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);

#[test]
fn failed_postlink_export_retains_exact_bytes_and_denials_create_only() {
    let parent = std::env::temp_dir().join(format!(
        "zryna-failed-export-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&parent).expect("create fresh private failure-export parent");
    let destination = parent.join("actual-failure");
    // Serializer fixture, not a valid ELF or native/process qualification.
    let raw = b"\x7fELF-independent-failed-bytes";
    let mut failure: Failure = rejected().into();
    failure.final_elf = Some(raw.to_vec());
    failure.comparator_stage = Some("postlink-declared-provider-closure");
    failure.artifacts.push(("foreign.c", b"independent retained source".to_vec()));
    failure.export(&destination).expect("create-only failed artifact snapshot");
    assert_eq!(
        fs::read(destination.join("failed-final.elf")).expect("retained exact failed ELF bytes"),
        raw
    );
    assert_eq!(
        fs::read(destination.join("foreign.c")).expect("retained original foreign source bytes"),
        b"independent retained source"
    );
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(destination.join("receipt.json")).expect("retained failure receipt exists"),
    )
    .expect("parse actual exported failure receipt");
    assert_eq!(report["failed_final_elf_sha256"], digest(raw));
    assert_eq!(report["failed_final_elf_bytes"], raw.len());
    assert_eq!(report["comparator_stage"], "postlink-declared-provider-closure");
    assert_eq!(report["execution_authorized"], false);
    assert_eq!(report["target_executed"], false);
    assert!(failure.export(&destination).is_err());
    assert_eq!(
        fs::read(destination.join("failed-final.elf")).expect("retained exact failed ELF bytes"),
        raw
    );
    failure.final_elf = Some(vec![0; native::MAX_NATIVE_EXECUTABLE_BYTES + 1]);
    assert!(failure.export(&parent.join("oversized")).is_err());
    assert!(!parent.join("oversized").exists());
    fs::remove_dir_all(parent).expect("parse actual exported failure receipt");
}
