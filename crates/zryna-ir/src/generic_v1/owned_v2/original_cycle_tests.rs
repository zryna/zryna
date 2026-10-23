//! Proposed real-source independent IR adapter RED; no semantic issuer is called.
use super::{Failure, check_original_bodies, check_original_bodies_v4};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;

#[test]
fn pinned_unused_original_self_cycle_reports_whole_member_span_in_v4_only() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-catalogue-fixtures/by-value-cycle");
    let text = std::fs::read_to_string(root.join("main.zry")).expect("unchanged literal source");
    assert_eq!(text, "interface Node<T extends ZrynaValue> extends ZrynaStruct { next: Node<T>; }");
    assert_eq!(text.len(), 75);
    assert_eq!(&text.as_bytes()[65..72], b"Node<T>");
    for provider in ["native", "typescript"] {
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.clone() }])
                .expect("exact original immutable SourceMap");
        let dto = std::fs::read(root.join(format!("{provider}.json")))
            .expect("pinned independent producer original facts");
        let syntax = v5::verify_snapshot(v5::decode_snapshot(&dto).expect("raw schema"), &sources)
            .expect("complete source authentication, no invented function or instance");
        let Failure::Diagnostics(errors) = check_original_bodies_v4(&syntax, &sources)
            .expect_err("a direct original by-value self edge cannot have a layout")
        else {
            panic!("owning rejection diagnostic required")
        };
        assert_eq!(errors.len(), 1, "{provider}: {errors:?}");
        assert_eq!(errors[0].code(), "ZRYNA-L3002", "{provider}: {errors:?}");
        let span = errors[0].primary_span().expect("whole original member application");
        assert_eq!((span.file().index(), span.start(), span.end()), (0, 65, 72));
        let Failure::Diagnostics(legacy) = check_original_bodies(&syntax, &sources)
            .expect_err("old source lane still rejects original Struct declarations")
        else {
            panic!("legacy source diagnostic required")
        };
        assert_eq!(legacy[0].code(), "ZRYNA-I7001");
    }
}
