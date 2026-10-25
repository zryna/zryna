//! Frozen oracle comparisons against actual canonical events and physical ownership.
pub(super) fn check(
    run: &serde_json::Value,
    repeats: u64,
    denied: bool,
    bytes: &[u8],
) -> Vec<Vec<u64>> {
    assert_eq!(bytes.len(), usize::try_from(repeats).expect("two") * 14 * 8);
    bytes
        .chunks_exact(14 * 8)
        .enumerate()
        .map(|(index, record)| {
            let fields = record
                .chunks_exact(8)
                .map(|f| u64::from_le_bytes(f.try_into().expect("u64")))
                .collect::<Vec<_>>();
            assert_eq!(fields[0], 0x4d34_3136_4143_4331);
            assert_eq!(fields[1], if denied { 1 } else { 2 });
            assert_eq!(fields[2], u64::try_from(index).expect("bounded") + 1);
            for (slot, operation) in
                [(3, "vecAllocate"), (4, "vecReserve"), (5, "vecReleaseStorage")]
            {
                assert_eq!(
                    fields[slot],
                    run["canonical_logical_entries_per_invocation"][operation]
                        .as_u64()
                        .expect("frozen count")
                );
            }
            for (slot, name) in [
                (6, "malloc_calls_per_invocation"),
                (7, "successful_nonzero_allocations_per_invocation"),
                (8, "free_calls_per_invocation"),
                (10, "peak_live_blocks_per_invocation"),
            ] {
                assert_eq!(fields[slot], run[name].as_u64().expect("frozen physical count"));
            }
            assert_eq!(fields[9], 0, "no physical block at boundary");
            assert_eq!(fields[11], 0, "all storage owners cleaned including empty");
            assert_eq!(
                fields[12],
                fields[3] + fields[4] + fields[5],
                "no internal C factoring double event"
            );
            assert_eq!(
                fields[12],
                run["end_context_per_invocation"]["S"].as_u64().expect("frozen logical S")
            );
            assert_eq!(
                fields[13],
                fields[3] + fields[4],
                "each admitted acquisition/transfer receives caller-zero out"
            );
            fields
        })
        .collect()
}
