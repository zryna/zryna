//! Nested source authority must precede the preserved native amplification boundary.
use super::{lower, with_source};

#[test]
fn nested_wrapped_enum_amplification_is_charged_after_full_source_authority() {
    const FUTURE_MINIMUM: u64 = 24 * 1024 * (2 * 16 + 3 + 6);
    assert_eq!(FUTURE_MINIMUM, 1_007_616, "unchanged enum-only lower bound");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/generic_owned_v2/struct_tests/fixtures");
    for drops in [1, 24] {
        let source = std::fs::read_to_string(root.join(format!("wrapped-{drops}.zry")))
            .expect("unchanged nested source");
        let bytes = std::fs::read(root.join(format!("wrapped-{drops}.json")))
            .expect("unchanged complete raw DTO");
        // This obtains original ownership, exact demand, both layouts, runtime and v4 seal.
        // An earlier source/resource rejection cannot qualify the native boundary.
        with_source(source, &bytes, |program| {
            let result = lower(program);
            if drops == 1 {
                result.expect("one nested wrapped enum fits unchanged native caps");
            } else {
                const {
                    assert!(FUTURE_MINIMUM > super::super::MAX_CODEGEN_UNITS);
                }
                let error = result.expect_err("repeated actual enum cleanup exceeds native work");
                assert_eq!(error.code(), "ZRYNA-N7001");
            }
        });
    }
}
