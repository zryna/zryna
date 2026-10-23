use super::{Actual, Value, address, number, text};
impl Actual {
    pub(super) fn observe(&mut self, record: &Value, expected: &[Value]) {
        match text(record, "event") {
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
            "failure" => self.failure.push((text(record, "kind").into(), number(&record["id"]))),
            "old-drop-entry" | "old-drop-complete" | "root-committed" => self.replacement(record),
            "unchanged-source" => {
                let id = self.check_live(record);
                self.unchanged_sources.push(id);
            }
            "unchanged-root" => {
                let id = self.check_live(record);
                self.unchanged_roots.push(id);
            }
            "done" => assert!(self.done.replace(record.clone()).is_none()),
            other => panic!("unknown canonical observer event {other}"),
        }
    }
    fn replacement(&mut self, record: &Value) {
        match text(record, "event") {
            "old-drop-entry" => {
                let id = number(&record["id"]);
                assert_eq!(self.open_release, Some(id));
                let rhs = self.live.get(text(record, "source")).expect("prepared RHS remains live");
                assert_eq!(rhs.0, id + 1);
                if let Some(root) = &self.root_address {
                    assert_eq!(root, text(record, "base"));
                } else {
                    self.root_address = Some(text(record, "base").into());
                }
                self.old_entries.push(id);
            }
            "old-drop-complete" => {
                let id = number(&record["id"]);
                assert_eq!(self.released.last(), Some(&id));
                assert_eq!(self.root_address.as_deref(), Some(text(record, "base")));
                assert_eq!(
                    self.live.get(text(record, "source")).expect("RHS still live after old free").0,
                    id + 1
                );
                self.old_complete.push(id);
            }
            "root-committed" => {
                let id = self.check_live(record);
                assert_eq!(self.old_complete.last(), Some(&(id - 1)));
                assert_eq!(self.root_address.as_deref(), Some(text(record, "base")));
                self.committed.push(id);
            }
            other => panic!("unknown replacement event {other}"),
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
        let source = number(&wanted["source_id"]);
        if source == 0 {
            assert_eq!(text(row, "source"), "0");
        } else {
            assert_eq!(
                self.live.get(text(row, "source")).expect("actual clone source remains live").0,
                source
            );
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
    }
}
