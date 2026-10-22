//! Whole-source fixture construction and faithful identifier mutation.

use super::*;

pub(super) fn value(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("independent raw fixture")
}

pub(super) fn sources(raw: &Value, source: &str) -> SourceMap {
    SourceMap::build(
        raw["files"]
            .as_array()
            .expect("files")
            .iter()
            .enumerate()
            .map(|(index, unit)| SourceFileInput {
                path: unit["path"].as_str().expect("path").into(),
                text: if index == 0 { source } else { VALUES_SOURCE }.into(),
            })
            .collect(),
    )
    .expect("source contexts")
}

pub(super) fn rejects(raw: Value, source: &str) {
    let map = sources(&raw, source);
    let raw: RawProjectSyntaxSnapshot = serde_json::from_value(raw).expect("typed raw claims");
    let errors = verify_snapshot(raw, &map).expect_err("no syntax authority");
    assert!(errors.iter().all(|error| error.code == "ZRYNA-Y5001"), "{errors:?}");
}

// Keep the whole independently authored source/arena context when changing a single name.
pub(super) fn shift(raw: &mut Value, start: u64, end: u64, delta: i64) {
    match raw {
        Value::Object(object) => {
            if object.get("file").and_then(Value::as_u64) == Some(0)
                && object.contains_key("start")
                && object.contains_key("end")
            {
                for key in ["start", "end"] {
                    let offset = object[key].as_u64().expect("offset");
                    if offset >= end && !(start == end && key == "start" && offset == start) {
                        object.insert(
                            key.into(),
                            json!(
                                offset.checked_add_signed(delta).expect("shifted fixture offset")
                            ),
                        );
                    }
                }
            }
            for child in object.values_mut() {
                shift(child, start, end, delta);
            }
        }
        Value::Array(array) => {
            for child in array {
                shift(child, start, end, delta);
            }
        }
        _ => {}
    }
}

pub(super) fn renamed(
    bytes: &[u8],
    source: &str,
    pointer: &str,
    replacement: &str,
) -> (Value, String) {
    let mut raw = value(bytes);
    let name = raw.pointer(pointer).expect("selected source name");
    let start = name["span"]["start"].as_u64().expect("start");
    let end = name["span"]["end"].as_u64().expect("end");
    assert_eq!(
        &source[usize::try_from(start).expect("fixture start")
            ..usize::try_from(end).expect("fixture end")],
        name["text"].as_str().expect("name")
    );
    let mut source = source.to_owned();
    source.replace_range(
        usize::try_from(start).expect("fixture start")..usize::try_from(end).expect("fixture end"),
        replacement,
    );
    shift(
        &mut raw,
        start,
        end,
        i64::try_from(replacement.len()).expect("replacement length")
            - i64::try_from(end - start).expect("original length"),
    );
    raw.pointer_mut(pointer).expect("selected source name")["text"] = json!(replacement);
    (raw, source)
}
