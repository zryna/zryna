//! Literal Rust port of the already frozen borrowed-target current-protocol trace projection.
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Write};
pub(super) fn check(
    store: &mut wasmtime::Store<()>,
    getter: &wasmtime::TypedFunc<i32, i32>,
    run: &Value,
    fault: Option<&Value>,
) {
    assert_eq!(
        getter.call(&mut *store, 0).expect("actual category"),
        i32::from(fault.is_some()) * 2
    );
    let count = usize::try_from(getter.call(&mut *store, 0x4000_0000).expect("trace word count"))
        .expect("nonnegative current count");
    assert!(count <= 4096, "first-extra4097 trace rejection unchanged");
    let words = (0..count)
        .map(|i| {
            let selector = 0x4000_0001_u32 + u32::try_from(i).expect("bounded selector");
            getter
                .call(&mut *store, selector.cast_signed())
                .expect("checked actual word")
                .cast_unsigned()
        })
        .collect::<Vec<_>>();
    let acquisitions = run["acquisitions"].as_array().expect("frozen acquisitions");
    let prefix = fault.map_or(acquisitions.len(), |f| {
        usize::try_from(f["successful_acquisitions"].as_u64().expect("frozen success prefix"))
            .expect("bounded prefix")
    });
    let mut trace = Trace::default();
    let mut cursor = 0;
    while cursor < words.len() {
        assert!(cursor + 5 <= words.len(), "whole existing record header");
        let h = &words[cursor..cursor + 5];
        cursor += 5;
        let length = usize::try_from(h[4]).expect("bounded UTF8 length");
        assert!(length <= words.len() - cursor, "whole exact byte payload");
        let bytes = &words[cursor..cursor + length];
        cursor += length;
        assert!(bytes.iter().all(|b| *b <= 255));
        let mut hex = String::new();
        for byte in bytes {
            write!(&mut hex, "{byte:02x}").expect("String formatting");
        }
        trace.record(h, hex, acquisitions, prefix, fault);
    }
    assert!(trace.live.is_empty(), "every successful allocation released exactly once");
    assert_eq!(trace.acquired, (1..=prefix).collect::<Vec<_>>());
    let expected = fault.unwrap_or(&run["pristine"]);
    let ids = expected["release_acquisition_ids"]
        .as_array()
        .expect("original release order")
        .iter()
        .map(|id| usize::try_from(id.as_u64().expect("original ID")).expect("small ID"))
        .collect::<Vec<_>>();
    assert_eq!(trace.released, ids);
    let wanted = expected["events"]
        .as_array()
        .expect("original source event interleaving")
        .iter()
        .map(|e| e.as_str().expect("source event").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(trace.events, wanted);
    assert_eq!(trace.failures, usize::from(fault.is_some()));
}
#[derive(Default)]
struct Trace {
    live: BTreeMap<usize, (u32, String)>,
    acquired: Vec<usize>,
    released: Vec<usize>,
    events: Vec<String>,
    failures: usize,
}
impl Trace {
    fn record(
        &mut self,
        h: &[u32],
        hex: String,
        acquisitions: &[Value],
        prefix: usize,
        fault: Option<&Value>,
    ) {
        let id = usize::try_from(h[1]).expect("actual logical ID or fault ordinal");
        match h[0] {
            1 | 2 => {
                assert_eq!(id, self.acquired.len() + 1);
                assert!(id <= prefix);
                assert_ne!(h[2], 0);
                assert!(
                    self.live.values().all(|row| row.0 != h[2]),
                    "no live physical allocation alias"
                );
                let expected = &acquisitions[id - 1];
                assert_eq!(expected["utf8_hex"], hex);
                let sources = expected["source_ids"].as_array().expect("original source roles");
                if sources.len() == 1 {
                    assert_eq!(h[0], 2);
                    let source =
                        usize::try_from(sources[0].as_u64().expect("source ID")).expect("small ID");
                    assert_eq!(
                        self.live.get(&source).expect("source retained through clone").0,
                        h[3]
                    );
                } else {
                    assert!(
                        sources.is_empty(),
                        "these frozen matches contain no concat source pair"
                    );
                    assert_eq!((h[0], h[3]), (1, 0));
                }
                assert!(self.live.insert(id, (h[2], hex)).is_none());
                self.acquired.push(id);
                self.events.push(format!("acquire:{id}"));
            }
            3 => {
                let live = self.live.remove(&id).expect("exact initialized live owner");
                assert_eq!((h[2], hex), live);
                assert_eq!(h[3], 0);
                self.released.push(id);
                self.events.push(format!("release:{id}"));
            }
            4 | 5 => {
                let fault = fault.expect("only configured failure may emit failed record");
                let kind = fault["kind"].as_str().expect("original kind");
                assert_eq!(h[0], if kind == "allocation" { 4 } else { 5 });
                assert_eq!(h[2], 0);
                assert_eq!(u64::from(h[1]), fault["ordinal"].as_u64().expect("original ordinal"));
                self.failures += 1;
                self.events.push(format!("failure:{kind}:{id}"));
            }
            _ => panic!("no observer tag extension"),
        }
    }
}
