//! Compare real fixed-space canonical observations against pre-output source/law oracle.
pub(super) fn check(
    case: &str,
    run: &serde_json::Value,
    repeats: u64,
    denied: bool,
    bytes: &[u8],
) -> Vec<Vec<u64>> {
    assert_eq!(bytes.len(), usize::try_from(repeats).expect("two") * 22 * 8);
    bytes
        .chunks_exact(22 * 8)
        .enumerate()
        .map(|(index, record)| {
            let fields = record
                .chunks_exact(8)
                .map(|f| u64::from_le_bytes(f.try_into().expect("u64")))
                .collect::<Vec<_>>();
            assert_eq!(fields[0], 0x4d34_3136_4143_4331);
            assert_eq!(fields[1], if denied { 1 } else { 2 });
            assert_eq!(fields[2], u64::try_from(index).expect("bounded") + 1);
            for (slot, role) in [
                (3, "allocate"),
                (4, "strongClone"),
                (5, "weakDowngrade"),
                (6, "weakClone"),
                (7, "weakUpgrade"),
                (8, "strongReleaseBegin"),
                (9, "strongReleaseFinish"),
                (10, "weakRelease"),
            ] {
                assert_eq!(
                    fields[slot],
                    run["canonical_logical_entries_per_invocation"][role]
                        .as_u64()
                        .expect("frozen count")
                );
            }
            for (slot, name) in [
                (11, "malloc_calls_per_invocation"),
                (12, "successful_nonzero_allocations_per_invocation"),
                (13, "free_calls_per_invocation"),
                (15, "peak_live_blocks_per_invocation"),
            ] {
                assert_eq!(fields[slot], run[name].as_u64().expect("frozen physical count"));
            }
            assert_eq!(fields[14], 0, "no live physical block");
            assert_eq!(fields[16], 0, "all strong owners cleaned");
            assert_eq!(fields[17], 0, "all explicit weak owners cleaned");
            assert_eq!(
                fields[18],
                fields[3..=10].iter().sum::<u64>(),
                "no internal C factoring event"
            );
            assert_eq!(
                fields[18],
                run["end_context_per_invocation"]["S"].as_u64().expect("frozen S")
            );
            assert_eq!(
                fields[19],
                fields[3] + fields[8] + fields[10],
                "each admitted output starts zero"
            );
            let expired =
                if case != "control-expired-call-loop" || run["label"] == "allocation-fault" {
                    0
                } else {
                    run["completed_iterations_per_invocation"].as_u64().expect("completed")
                };
            assert_eq!(
                fields[20], expired,
                "exact frozen expired source edge, no manufactured successful owner"
            );
            assert_eq!(fields[21], u64::from(run["label"] == "allocation-fault"));
            fields
        })
        .collect()
}
