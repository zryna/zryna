//! Exact String trace projection; Vec buffer events are intentionally not present in this protocol.
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Write};
pub(super) fn snapshot(
    store: &mut wasmtime::Store<()>,
    getter: &wasmtime::TypedFunc<i32, i32>,
) -> (i32, Vec<u32>) {
    let category = getter.call(&mut *store, 0).expect("actual exit category");
    let count = usize::try_from(getter.call(&mut *store, 0x4000_0000).expect("word count"))
        .expect("nonnegative trace extent");
    assert!(count <= 4096, "existing trace4097 rejection unchanged");
    let words = (0..count)
        .map(|i| {
            getter
                .call(
                    &mut *store,
                    (0x4000_0001_u32 + u32::try_from(i).expect("bounded selector")).cast_signed(),
                )
                .expect("actual trace word")
                .cast_unsigned()
        })
        .collect();
    (category, words)
}
pub(super) fn check(words: &[u32], run: &Value) {
    let expected = run["trace"].as_array().expect("pre-output String projection");
    let mut actual = Vec::new();
    let mut live = BTreeMap::new();
    let mut cursor = 0;
    while cursor < words.len() {
        assert!(cursor + 5 <= words.len());
        let header = &words[cursor..cursor + 5];
        cursor += 5;
        let length = usize::try_from(header[4]).expect("bounded UTF8 length");
        assert!(length <= words.len() - cursor);
        let mut hex = String::new();
        for byte in &words[cursor..cursor + length] {
            assert!(*byte <= 255);
            write!(&mut hex, "{byte:02x}").expect("hex formatting");
        }
        cursor += length;
        let row = &expected[actual.len()];
        let id = header[1];
        assert_eq!(u64::from(id), row["id"].as_u64().expect("frozen physical success/fault ID"));
        assert_eq!(u64::from(header[0]), row["tag"].as_u64().expect("existing trace tag"));
        assert_eq!(hex, row["utf8"].as_str().expect("exact source UTF8"));
        let source =
            u32::try_from(row["source"].as_u64().expect("original source ID")).expect("bounded ID");
        match header[0] {
            1 | 2 => {
                assert_ne!(header[2], 0);
                assert!(live.values().all(|pointer| *pointer != header[2]));
                if source == 0 {
                    assert_eq!(header[3], 0);
                } else {
                    assert_eq!(header[3], *live.get(&source).expect("clone source retained"));
                }
                assert!(live.insert(id, header[2]).is_none());
            }
            3 => {
                assert_eq!(Some(header[2]), live.remove(&id));
                assert_eq!(header[3], 0);
            }
            4 => {
                assert_eq!(header[2], 0);
                if source == 0 {
                    assert_eq!(header[3], 0);
                } else {
                    assert_eq!(
                        header[3],
                        *live.get(&source).expect("failed clone source retained")
                    );
                }
            }
            _ => panic!("no new observer tags"),
        }
        actual.push((header[0], id));
    }
    assert_eq!(actual.len(), expected.len());
    assert!(live.is_empty(), "every observed successful String is released exactly once");
}
