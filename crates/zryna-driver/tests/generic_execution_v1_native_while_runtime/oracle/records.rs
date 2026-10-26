use super::{Actual, Value, address, number, text};
impl Actual {
    pub(super) fn observe(&mut self, _case: &str, record: &Value, expected: &[Value]) {
        match text(record, "event") {
            "entry" => {}
            "host-malloc" => {
                let id = number(&record["id"]);
                assert!(
                    self.physical
                        .insert(
                            id,
                            (
                                text(record, "base").into(),
                                record["bytes"].as_u64().expect("actual malloc extent")
                            )
                        )
                        .is_none()
                );
            }
            "acquire" => self.acquire(record, expected),
            "release-entry" => {
                let id = self.check_live(record);
                assert!(self.open_release.replace(id).is_none());
                self.release_entries.push(id);
            }
            "release" => {
                let id = self.check_live(record);
                assert_eq!(self.open_release, Some(id));
                self.live.remove(text(record, "pointer")).expect("one live owner");
                self.released.push(id);
                self.events.push(format!("release:{id}"));
            }
            "host-free" => {
                let id = number(&record["id"]);
                assert_eq!(self.open_release.take(), Some(id));
                let (base, _) = self.physical.remove(&id).expect("one real live allocation");
                assert_eq!(base, text(record, "base"));
                self.freed.push(id);
            }
            "out-zero" => {
                self.zeros += 1;
                assert_eq!(number(&record["id"]), self.zeros);
            }
            "clone-entry" => {
                self.check_live(record);
                self.clones += 1;
            }
            "failure" => {
                let kind = text(record, "kind");
                let ordinal = number(&record["id"]);
                self.failure.push((kind.into(), ordinal));
                self.events.push(format!("failure:{kind}:{ordinal}"));
            }
            "concat-left" => {
                self.check_live(record);
                self.concats += 1;
            }
            "concat-right" => {
                self.check_live(record);
            }
            "unchanged-source" => {
                let id = self.check_live(record);
                self.unchanged_sources.push(id);
            }
            "done" => assert!(self.done.replace(record.clone()).is_none()),
            other => panic!("unknown canonical observer event {other}"),
        }
    }
    fn check_live(&self, row: &Value) -> usize {
        let (id, _, bytes) =
            self.live.get(text(row, "pointer")).expect("actual live payload identity");
        assert_eq!(*id, number(&row["id"]));
        assert_eq!(bytes, text(row, "bytes_hex"));
        assert_eq!(bytes.len(), number(&row["length"]) * 2);
        *id
    }
    fn acquire(&mut self, row: &Value, expected: &[Value]) {
        let id = number(&row["id"]);
        assert_eq!(id, self.acquisitions.len() + 1);
        let wanted = &expected[id - 1];
        assert_eq!(row["bytes_hex"], wanted["utf8_hex"]);
        assert_eq!(text(row, "bytes_hex").len(), number(&row["length"]) * 2);
        let (base, size) =
            self.physical.get(&id).expect("actual canonical malloc preceded publication");
        assert_eq!(base, text(row, "base"));
        let offset = address(text(row, "pointer"))
            .checked_sub(address(base))
            .expect("payload inside real malloc extent");
        assert!(offset <= *size && row["length"].as_u64().expect("String extent") <= size - offset);
        let sources = wanted["source_ids"].as_array().expect("exact source arity");
        for (index, key) in ["source", "right"].iter().enumerate() {
            if let Some(source) = sources.get(index) {
                assert_eq!(
                    self.live.get(text(row, key)).expect("observed source remains live").0,
                    number(source)
                );
            } else {
                assert_eq!(text(row, key), "0");
            }
        }
        assert!(
            self.live
                .insert(
                    text(row, "pointer").into(),
                    (id, base.clone(), text(row, "bytes_hex").into())
                )
                .is_none(),
            "pointer reuse allowed only after previous free"
        );
        self.acquisitions.push(text(row, "pointer").into());
        self.events.push(format!("acquire:{id}"));
    }
}
