//! Existing five-word variable-byte records; container identities and dual concat sources are not invented.
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Write};
pub(super) fn check(
    store: &mut wasmtime::Store<()>,
    observe: &wasmtime::TypedFunc<i32, i32>,
    row: &Value,
    fault: Option<&Value>,
) {
    let count = observe.call(&mut *store, 0x4000_0000).expect("current trace word count");
    let count = usize::try_from(count).expect("nonnegative trace word count");
    assert!(count <= 4096, "first-extra4097 trace overflow is rejected");
    let words = (0..count)
        .map(|index| {
            let selector = 0x4000_0001_u32 + u32::try_from(index).expect("bounded trace index");
            observe
                .call(&mut *store, selector.cast_signed())
                .expect("checked trace word")
                .cast_unsigned()
        })
        .collect::<Vec<_>>();
    let acquisitions = row["acquisitions"].as_array().expect("original acquisition identities");
    let successful = fault.map_or(acquisitions.len(), |f| {
        usize::try_from(f["successful_acquisitions"].as_u64().expect("original success prefix"))
            .expect("bounded prefix")
    });
    let expected = acquisitions
        .iter()
        .take(successful)
        .enumerate()
        .filter(|(_, item)| item["kind"] == "String")
        .collect::<Vec<_>>();
    let release_expected = fault.unwrap_or(row)["releases"]
        .as_array()
        .expect("original reverse releases")
        .iter()
        .map(|id| usize::try_from(id.as_u64().expect("logical ID")).expect("small ID"))
        .filter(|id| acquisitions[*id - 1]["kind"] == "String")
        .collect::<Vec<_>>();
    let mut acquired = Vec::new();
    let mut released = Vec::new();
    let mut addresses = BTreeMap::new();
    let mut cursor = 0;
    while cursor < words.len() {
        assert!(cursor + 5 <= words.len(), "whole existing record header");
        let header = &words[cursor..cursor + 5];
        cursor += 5;
        let length = usize::try_from(header[4]).expect("bounded payload length");
        assert!(length <= words.len() - cursor, "whole existing byte payload");
        let bytes = &words[cursor..cursor + length];
        cursor += length;
        assert!(bytes.iter().all(|b| *b <= 255));
        let mut hex = String::new();
        for byte in bytes {
            write!(&mut hex, "{byte:02x}").expect("String formatting");
        }
        match header[0] {
            1 | 2 => {
                let id = usize::try_from(header[1]).expect("actual logical acquisition");
                let original = &acquisitions[id - 1];
                assert_eq!(original["kind"], "String");
                assert_eq!(original["utf8_hex"], hex);
                assert_ne!(header[2], 0, "actual allocated pointer");
                assert!(
                    addresses.values().all(|p| *p != header[2]),
                    "fresh physical String allocation"
                );
                if header[0] == 2 {
                    let source = usize::try_from(
                        original["source_id"].as_u64().expect("original clone source"),
                    )
                    .expect("small source");
                    assert_eq!(
                        addresses.get(&source),
                        Some(&header[3]),
                        "exact source pointer for clone"
                    );
                }
                assert!(addresses.insert(id, header[2]).is_none());
                acquired.push(id);
            }
            3 => {
                let id = usize::try_from(header[1]).expect("actual released ID");
                assert_eq!(addresses.get(&id), Some(&header[2]), "release exact acquired pointer");
                assert_eq!(acquisitions[id - 1]["utf8_hex"], hex);
                released.push(id);
            }
            4 | 5 => {}
            _ => panic!("unextended current trace tag"),
        }
    }
    assert_eq!(acquired, expected.iter().map(|(index, _)| index + 1).collect::<Vec<_>>());
    assert_eq!(released, release_expected);
}
