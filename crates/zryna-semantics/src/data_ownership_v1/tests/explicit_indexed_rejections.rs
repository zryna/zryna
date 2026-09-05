use super::explicit_indexed_fixture::{Action, Container, fixture};
use super::generic_vec_fixture::Element;
use super::*;

#[test]
fn explicit_indexed_rejections_are_authenticated_and_deterministic() {
    let cases = [
        (Action::Replace, "ZRYNA-M3017", "shared authority cannot mutate"),
        (Action::Read, "ZRYNA-M3017", "owned referent cannot be implicitly read"),
        (Action::OwnerMove, "ZRYNA-M3014", "container cannot move under element authority"),
        (
            Action::OwnerReplace,
            "ZRYNA-M3014",
            "container cannot be replaced under element authority",
        ),
        (Action::Conflict, "ZRYNA-M3014", "unequal indices retain whole-container conflicts"),
        (Action::Collision, "ZRYNA-M3002", "active alias names cannot collide"),
        (Action::Escape, "ZRYNA-M3002", "alias cannot escape lexical scope"),
    ];
    for container in [Container::Array(2), Container::Vec] {
        for element in
            [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec]
        {
            for (action, code, label) in cases {
                let (source, raw) = fixture(container, &element, false, action, None);
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).unwrap_or_else(|errors| {
                    panic!("{label}: invalid fixture {errors:?}\n{source}")
                });
                let check = || lower(pair_input(&syntax, &sources)).expect_err(label);
                let first = check();
                assert_eq!(first.len(), 1, "{label}: {first:?}\n{source}");
                assert_eq!(
                    first[0].code(),
                    code,
                    "{container:?} {element:?} {label}: {first:?}\n{source}"
                );
                assert_eq!(first, check(), "{label}");
            }
        }
    }
}
