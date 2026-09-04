use super::{check_sites, site};
use crate::data_ownership_v1::tests::*;
use zryna_syntax::v4::RawExpressionKind;

pub(super) fn two_element_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (original, raw) = super::super::nested_enum_fixture();
    let mut source = original.0.to_owned();
    let start = source.find("\"a\"").expect("fixed first element");
    let end = start + 3;
    source.insert_str(end, ", \"b\"");
    let mut raw = shift_snapshot(raw, u32::try_from(end).expect("source offset"), 5);
    let body = &mut raw.files[0].functions[0].body;
    // Restore the first literal's end: insertion belongs to the enclosing Vec, not its child.
    body.expressions[0].span.end = u32::try_from(end).expect("first literal end");
    body.expressions.insert(
        1,
        RawExpressionSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: u32::try_from(end + 2).expect("second literal start"),
                end: u32::try_from(end + 5).expect("second literal end"),
            },
            kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".into() },
        },
    );
    let RawExpressionKind::VecConstruction { elements, .. } = &mut body.expressions[2].kind else {
        panic!("fixed inner Vec");
    };
    *elements = vec![0, 1];
    let RawExpressionKind::EnumConstruction { payload, .. } = &mut body.expressions[3].kind else {
        panic!("fixed selected Enum");
    };
    *payload = Some(2);
    let RawExpressionKind::VecConstruction { elements, .. } = &mut body.expressions[4].kind else {
        panic!("fixed outer Vec");
    };
    *elements = vec![3];
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("fixed return");
    };
    *value = 4;
    (source, raw)
}

#[test]
fn mixed_selected_enum_two_string_elements_keep_reverse_completed_prefix_on_failure() {
    let (source, snapshot) = two_element_fixture();
    check_sites(
        &source,
        snapshot,
        5,
        &[
            site(0, LogicalOperation::StringFromUtf8Copy, &[], &[]),
            site(1, LogicalOperation::StringFromUtf8Copy, &[], &[0]),
            site(2, LogicalOperation::VecAllocate, &[0, 1], &[1, 0]),
            site(4, LogicalOperation::VecAllocate, &[3], &[3]),
        ],
    );
}
