use super::generic_vec_fixture::{self, Element, Operation};
use super::*;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawExpressionKind;

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Container {
    Array(u64),
    Vec,
}
#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Action {
    Read,
    Clone,
    Replace,
    ReplaceClone,
    OwnerMove,
    OwnerReplace,
    RootShared,
    SiblingVec,
    SameVec,
    Conflict,
    Collision,
    Escape,
    Call,
    GrowthInside,
    GrowthAfter,
}

pub(super) fn call_fixture(
    container: Container,
    element: &Element,
    exclusive: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    call_access_fixture(container, element, exclusive, exclusive)
}

pub(super) fn call_access_fixture(
    container: Container,
    element: &Element,
    exclusive: bool,
    callee_exclusive: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let (source, mut raw) = fixture(container, element, exclusive, Action::Call, None);
    let file = &mut raw.files[0];
    let element_type = file.functions[0].parameters[2].type_syntax;
    let mut body = file.functions[0].body.clone();
    body.expressions.clear();
    body.statements.clear();
    body.blocks.clear();
    let mut builder = Builder { source, types: &mut file.type_syntax, body: &mut body, slot: None };
    let callee = builder.callee(
        element_type,
        callee_exclusive,
        matches!(element, Element::I32 | Element::Bool),
    );
    file.functions.push(callee);
    (builder.source, raw)
}

#[path = "explicit_indexed_builder.rs"]
mod builder;
use builder::Builder;

fn shift(value: &mut serde_json::Value, position: u32, amount: u32) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                shift(value, position, amount);
            }
        }
        serde_json::Value::Object(values) => {
            if values.contains_key("file")
                && values.contains_key("start")
                && values.contains_key("end")
            {
                for key in ["start", "end"] {
                    let offset = values[key].as_u64().expect("span");
                    if offset > u64::from(position)
                        || (key == "start" && offset == u64::from(position))
                    {
                        values.insert(key.into(), (offset + u64::from(amount)).into());
                    }
                }
            } else {
                for value in values.values_mut() {
                    shift(value, position, amount);
                }
            }
        }
        _ => {}
    }
}

fn array_container(source: &mut String, raw: &mut RawProjectSyntaxSnapshot, length: u64) {
    let function = &raw.files[0].functions[0];
    let RawStatementKind::LocalDeclaration { type_syntax, .. } = function.body.statements[0].kind
    else {
        panic!("container local");
    };
    let mut ids = vec![function.parameters[0].type_syntax, function.result_type, type_syntax];
    ids.sort_unstable_by_key(|id| {
        std::cmp::Reverse(raw.files[0].type_syntax[*id as usize].span.start)
    });
    for id in ids {
        let old = raw.files[0].type_syntax[id as usize].clone();
        let RawTypeSyntaxKind::Vec { keyword_span, greater_than_span, .. } = old.kind else {
            panic!("Vec container");
        };
        let suffix = format!(", {length}");
        source.insert_str(greater_than_span.start as usize, &suffix);
        source.replace_range(keyword_span.start as usize..keyword_span.end as usize, "FixedArray");
        let mut json = serde_json::to_value(&*raw).expect("snapshot");
        shift(&mut json, greater_than_span.start, u32::try_from(suffix.len()).expect("length"));
        shift(&mut json, keyword_span.end, 7);
        *raw = serde_json::from_value(json).expect("shifted snapshot");
        let ty = &mut raw.files[0].type_syntax[id as usize];
        let RawTypeSyntaxKind::Vec {
            mut keyword_span,
            less_than_span,
            argument,
            greater_than_span,
        } = ty.kind.clone()
        else {
            panic!("shifted Vec");
        };
        keyword_span.end += 7;
        let comma = greater_than_span.start - u32::try_from(suffix.len()).expect("length");
        ty.kind = RawTypeSyntaxKind::FixedArray {
            keyword_span,
            less_than_span,
            element: argument,
            comma_span: at(comma, comma + 1),
            length_span: at(comma + 2, greater_than_span.start),
            length_spelling: length.to_string(),
            length: u32::try_from(length).expect("bounded fixture array length"),
            greater_than_span,
        };
    }
}

pub(super) fn at(start: u32, end: u32) -> UntrustedSpan {
    UntrustedSpan { file: 0, start, end }
}

pub(in crate::data_ownership_v1) fn fixture(
    container: Container,
    element: &Element,
    exclusive: bool,
    action: Action,
    index: Option<i32>,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = generic_vec_fixture::fixture(element, Operation::Replace, index);
    if let Container::Array(length) = container {
        array_container(&mut source, &mut raw, length);
    }
    let file = &mut raw.files[0];
    let function = &mut file.functions[0];
    let assignment = &function.body.statements[2];
    source.truncate(assignment.span.start as usize);
    let RawStatementKind::Assignment { target, .. } = assignment.kind else {
        panic!("assignment");
    };
    let RawExpressionKind::Index { base, .. } = function.body.expressions[target as usize].kind
    else {
        panic!("index");
    };
    function.body.expressions.truncate(base as usize);
    function.body.statements.truncate(2);
    let mut element_type = function.parameters[2].type_syntax;
    if matches!(action, Action::SiblingVec | Action::SameVec) {
        let RawTypeSyntaxKind::Vec { argument, .. } = file.type_syntax[element_type as usize].kind
        else {
            panic!("sibling Vec container");
        };
        element_type = argument;
    }
    let mut builder =
        Builder { source, types: &mut file.type_syntax, body: &mut function.body, slot: None };
    builder.lexical(element_type, exclusive, action, index);
    builder.finish();
    function.span.end = u32::try_from(builder.source.len()).expect("function end");
    (builder.source, raw)
}
