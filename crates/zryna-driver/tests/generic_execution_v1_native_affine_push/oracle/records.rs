//! Actual canonical allocation intervals and same-seal call metadata; no source-label substitutes.
use super::{Actual, Value, address, number, text};
impl Actual {
    pub(super) fn observe_physical(&mut self, record: &Value) {
        match text(record, "event") {
            "host-malloc" => {
                self.mallocs += 1;
                assert_eq!(number(&record["id"]), self.mallocs);
                let base = text(record, "base");
                assert_ne!(address(base), 0);
                assert!(
                    self.physical
                        .insert(
                            base.into(),
                            (
                                self.mallocs,
                                record["bytes"].as_u64().expect("physical malloc bytes")
                            )
                        )
                        .is_none()
                );
            }
            "host-free" => {
                let (id, _) = self
                    .physical
                    .remove(text(record, "base"))
                    .expect("actual live physical allocation base");
                assert_eq!(number(&record["id"]), id);
                assert!(self.released.contains(&id));
                assert!(self.freed.insert(id));
            }
            "bind-allocation" => {
                let base = text(record, "base");
                let (id, size) = self
                    .physical
                    .get(base)
                    .expect("returned pointer belongs to actual live malloc");
                assert_eq!(number(&record["id"]), *id);
                let offset = address(text(record, "pointer"))
                    .checked_sub(address(base))
                    .expect("payload inside actual allocation");
                assert!(offset + record["bytes"].as_u64().expect("payload bytes") <= *size);
                assert!(self.bindings.insert(*id, text(record, "pointer").into()).is_none());
            }
            _ => unreachable!("physical observer dispatch"),
        }
    }
    pub(super) fn observe_metadata(&mut self, record: &Value, metadata: &Value) {
        match text(record, "event") {
            "control-metadata" => {
                assert!(
                    metadata["control_layouts"]
                        .as_array()
                        .expect("NEWseal control metadata")
                        .iter()
                        .any(|row| row["size"] == record["size"]
                            && row["alignment"] == record["alignment"])
                );
                assert!(self.metadata.insert(text(record, "pointer").into()));
            }
            "vec-metadata" => {
                let shape = metadata["element_layouts"]
                    .as_array()
                    .expect("NEWseal element metadata")
                    .iter()
                    .find(|row| row["element"] == record["element"])
                    .expect("same-seal element");
                assert_eq!(shape["stride"], record["stride"]);
                assert_eq!(shape["alignment"], record["alignment"]);
                assert!(
                    record["capacity"].as_u64().expect("actual capacity")
                        >= record["required"].as_u64().expect("required length")
                );
                assert!(self.metadata.insert(text(record, "pointer").into()));
            }
            _ => unreachable!("runtime metadata dispatch"),
        }
    }
}
