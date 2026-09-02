use super::*;

pub(super) fn response_snapshot(response: &str) -> RawProjectSyntaxSnapshot {
    let value: serde_json::Value = serde_json::from_str(response).expect("adapter response JSON");
    let result = value.get("result").expect("adapter result");
    decode_snapshot(&serde_json::to_vec(result).expect("snapshot JSON")).expect("v4 snapshot")
}
pub(super) fn sources_for(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("source map")
}
pub(super) fn nth_untrusted_span(
    text: &str,
    needle: &str,
    ordinal: usize,
) -> zryna_source::UntrustedSpan {
    let start =
        text.match_indices(needle).nth(ordinal).map(|(start, _)| start).expect("fixture token");
    zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(start + needle.len()).expect("fixture offset"),
    }
}
pub(super) fn untrusted_range(
    text: &str,
    start: (&str, usize),
    end: (&str, usize),
) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file: 0,
        start: nth_untrusted_span(text, start.0, start.1).start,
        end: nth_untrusted_span(text, end.0, end.1).end,
    }
}
pub(super) fn shift_snapshot(
    raw: RawProjectSyntaxSnapshot,
    cutoff: u32,
    amount: u32,
) -> RawProjectSyntaxSnapshot {
    fn visit(value: &mut serde_json::Value, cutoff: u32, amount: u32) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("file")
                    && object.contains_key("start")
                    && object.contains_key("end")
                {
                    for key in ["start", "end"] {
                        if let Some(number) = object.get_mut(key) {
                            let current = u32::try_from(number.as_u64().expect("span number"))
                                .expect("u32 span");
                            if current >= cutoff {
                                *number = serde_json::Value::from(current + amount);
                            }
                        }
                    }
                } else {
                    for child in object.values_mut() {
                        visit(child, cutoff, amount);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    visit(child, cutoff, amount);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    visit(&mut value, cutoff, amount);
    serde_json::from_value(value).expect("shifted snapshot")
}
pub(super) fn shift_snapshot_signed(
    raw: RawProjectSyntaxSnapshot,
    cutoff: u32,
    amount: i32,
) -> RawProjectSyntaxSnapshot {
    fn visit(value: &mut serde_json::Value, cutoff: u32, amount: i32) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("file")
                    && object.contains_key("start")
                    && object.contains_key("end")
                {
                    for key in ["start", "end"] {
                        let number = object.get_mut(key).expect("span field");
                        let current =
                            i64::try_from(number.as_u64().expect("span number")).expect("i64 span");
                        if current >= i64::from(cutoff) {
                            *number = serde_json::Value::from(
                                u64::try_from(current + i64::from(amount)).expect("shifted span"),
                            );
                        }
                    }
                } else {
                    for child in object.values_mut() {
                        visit(child, cutoff, amount);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    visit(child, cutoff, amount);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(raw).expect("snapshot value");
    visit(&mut value, cutoff, amount);
    serde_json::from_value(value).expect("shifted snapshot")
}
pub(super) fn pair_sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: PAIR_SOURCE.to_owned(),
    }])
    .expect("Pair source map")
}
pub(super) fn pair_input<'a>(
    syntax: &'a zryna_syntax::v4::ProjectSyntaxSnapshot,
    sources: &'a SourceMap,
) -> SemanticInput<'a> {
    let path = NormalizedSourcePath::new("src/main.zry").expect("path");
    let entry = sources.file_id(&path).expect("entry");
    SemanticInput::try_new(syntax, sources, entry).expect("authenticated Pair input")
}
