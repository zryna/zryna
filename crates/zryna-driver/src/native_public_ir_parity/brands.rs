//! Compare complete derived owner state while respecting independently minted source authority.
//! Only an exact admitted source-map brand is renamed; source literals and all other data remain.

use std::fmt::Debug;
use zryna_source::SourceMap;

pub fn debug(value: &(impl Debug + ?Sized), sources: &SourceMap) -> String {
    let identity = format!("{:?}", sources.identity());
    let id = identity.strip_prefix("SourceMapIdentity(").unwrap().strip_suffix(')').unwrap();
    normalize(&format!("{value:?}"), id)
}

fn normalize(input: &str, expected_id: &str) -> String {
    let mut result = String::new();
    let mut tail = input;
    let mut quoted = false;
    let mut escaped = false;
    while !tail.is_empty() {
        if !quoted {
            let matched = ["SourceMapIdentity(", "source_map_id: "]
                .into_iter()
                .find(|prefix| tail.starts_with(prefix));
            if let Some(prefix) = matched {
                let digits = &tail[prefix.len()..];
                let count = digits.bytes().take_while(u8::is_ascii_digit).count();
                assert!(count > 0, "unknown source authority Debug representation");
                assert_eq!(
                    &digits[..count],
                    expected_id,
                    "owner state contains foreign source-map authority"
                );
                result.push_str(prefix);
                result.push_str("$SOURCE_MAP");
                tail = &digits[count..];
                continue;
            }
        }
        let character = tail.chars().next().unwrap();
        result.push(character);
        tail = &tail[character.len_utf8()..];
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
        } else if character == '"' {
            quoted = true;
        }
    }
    assert!(!quoted, "complete derived Debug string");
    result
}

#[test]
fn brand_comparison_keeps_all_values_and_quoted_source_text_exact() {
    let left = "Program { source: SourceMapIdentity(17), file: FileId { source_map_id: 17, index: 4 }, value: 17, text: \"SourceMapIdentity(17) source_map_id: 17 \\\" π\" }";
    let right = "Program { source: SourceMapIdentity(29), file: FileId { source_map_id: 29, index: 4 }, value: 17, text: \"SourceMapIdentity(17) source_map_id: 17 \\\" π\" }";
    assert_eq!(normalize(left, "17"), normalize(right, "29"));
    assert_ne!(normalize(left, "17"), normalize(&right.replace("value: 17", "value: 29"), "29"));
    assert_ne!(normalize(left, "17"), normalize(&right.replace("index: 4", "index: 5"), "29"));
    assert_ne!(
        normalize(left, "17"),
        normalize(&right.replace("(17) source_map_id", "(29) source_map_id"), "29")
    );
}

#[test]
#[should_panic(expected = "foreign source-map authority")]
fn brand_comparison_rejects_foreign_authority_instead_of_masking_it() {
    let _ = normalize(
        "Program { source: SourceMapIdentity(17), file: FileId { source_map_id: 29, index: 0 } }",
        "17",
    );
}
