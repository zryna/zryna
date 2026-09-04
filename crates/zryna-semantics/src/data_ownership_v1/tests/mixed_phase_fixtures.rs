use super::*;
use serde_json::json;
use zryna_syntax::v4::{RawExpressionKind, RawTypeSyntax, RawTypeSyntaxKind};

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1) enum PhaseChild {
    Projection,
    StringClone,
    AggregateClone,
}

// Fixed source edits over the already authenticated Pair fixture. The shared span-shift
// utility and protocol decoder own syntax encoding; this does not infer types or effects.
#[allow(clippy::too_many_lines)]
pub(in crate::data_ownership_v1) fn phase_fixture(
    mode: PhaseChild,
    invalid: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) =
        constructor_envelope_fixtures::snapshot(constructor_envelope_fixtures::Fixture::Pair);
    let result = raw.files[0].functions[0].result_type;
    assert_eq!(result, 2);
    let original = raw.files[0].type_syntax[result as usize].span;
    source.insert(original.end as usize, '>');
    raw = shift_snapshot(raw, original.end, 1);
    let named = &mut raw.files[0].type_syntax[result as usize];
    named.span.end = original.end;
    let RawTypeSyntaxKind::Named { name } = &mut named.kind else { panic!("Pair result") };
    name.span.end = original.end;
    source.insert_str(original.start as usize, "Vec<");
    raw = shift_snapshot(raw, original.start, 4);
    let at = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    raw.files[0].type_syntax.insert(
        3,
        RawTypeSyntax {
            span: at(original.start, original.end + 5),
            kind: RawTypeSyntaxKind::Vec {
                keyword_span: at(original.start, original.start + 3),
                less_than_span: at(original.start + 3, original.start + 4),
                argument: 2,
                greater_than_span: at(original.end + 4, original.end + 5),
            },
        },
    );
    raw.files[0].functions[0].result_type = 3;
    for statement in &mut raw.files[0].functions[0].body.statements {
        if let RawStatementKind::LocalDeclaration { type_syntax, .. } = &mut statement.kind {
            assert_eq!(*type_syntax, 3);
            *type_syntax = 4;
        }
    }
    let child = match mode {
        PhaseChild::Projection => "OwnedPair({ first: p.first, flag: true })",
        PhaseChild::StringClone => "OwnedPair({ first: clone(p.first), flag: true })",
        PhaseChild::AggregateClone => "clone(p)",
    };
    let replacement = format!("Vec<OwnedPair>([{child}{}])", if invalid { ", bad" } else { "" });
    let start = source.rfind("p;").expect("final Pair return");
    source.replace_range(start..=start, &replacement);
    raw = shift_snapshot(
        raw,
        u32::try_from(start + 1).expect("offset"),
        u32::try_from(replacement.len() - 1).expect("growth"),
    );
    let s = |a: usize, b: usize| json!({"file":0,"start":start+a,"end":start+b});
    let mut encoded = serde_json::to_value(raw).expect("fixture DTO");
    let file = &mut encoded["files"][0];
    let types = file["type_syntax"].as_array_mut().expect("types");
    assert_eq!(types.len(), 5);
    types.push(
        json!({"span":s(4,13),"kind":{"kind":"named","name":{"text":"OwnedPair","span":s(4,13)}}}),
    );
    types.push(json!({"span":s(0,14),"kind":{"kind":"vec","keyword_span":s(0,3),"less_than_span":s(3,4),"argument":5,"greater_than_span":s(13,14)}}));
    let body = &mut file["functions"][0]["body"];
    let expressions = body["expressions"].as_array_mut().expect("expressions");
    assert_eq!(expressions.len(), 4);
    expressions.pop();
    let child_start = 16;
    let child_end = child_start + child.len();
    let child_id = if matches!(mode, PhaseChild::AggregateClone) {
        let p = child_start + 6;
        let base = expressions.len();
        expressions.push(json!({"span":s(p,p+1),"kind":{"kind":"reference","name":{"text":"p","span":s(p,p+1)}}}));
        expressions.push(json!({"span":s(child_start,child_end),"kind":{"kind":"clone","keyword_span":s(child_start,child_start+5),"open_paren_span":s(child_start+5,child_start+6),"value":base,"close_paren_span":s(child_end-1,child_end)}}));
        base + 1
    } else {
        let p = replacement.find("p.first").expect("source projection");
        let base = expressions.len();
        expressions.push(json!({"span":s(p,p+1),"kind":{"kind":"reference","name":{"text":"p","span":s(p,p+1)}}}));
        expressions.push(json!({"span":s(p,p+7),"kind":{"kind":"field-access","base":base,"dot_span":s(p+1,p+2),"field":{"text":"first","span":s(p+2,p+7)}}}));
        let mut field = base + 1;
        let mut field_end = p + 7;
        if matches!(mode, PhaseChild::StringClone) {
            let begin = p - 6;
            field = expressions.len();
            field_end += 1;
            expressions.push(json!({"span":s(begin,field_end),"kind":{"kind":"clone","keyword_span":s(begin,begin+5),"open_paren_span":s(begin+5,begin+6),"value":base+1,"close_paren_span":s(field_end-1,field_end)}}));
        }
        let flag = replacement.find("flag: true").expect("Copy field");
        let boolean = expressions.len();
        expressions
            .push(json!({"span":s(flag+6,flag+10),"kind":{"kind":"bool-literal","value":true}}));
        let first = replacement.find("first:").expect("String field");
        let id = expressions.len();
        expressions.push(json!({"span":s(child_start,child_end),"kind":{"kind":"struct-construction",
            "type_name":{"text":"OwnedPair","span":s(child_start,child_start+9)},"open_paren_span":s(child_start+9,child_start+10),"open_brace_span":s(child_start+10,child_start+11),
            "fields":[{"span":s(first,field_end),"kind":{"kind":"explicit","name":{"text":"first","span":s(first,first+5)},"colon_span":s(first+5,first+6),"value":field}},
                {"span":s(flag,flag+10),"kind":{"kind":"explicit","name":{"text":"flag","span":s(flag,flag+4)},"colon_span":s(flag+4,flag+5),"value":boolean}}],
            "close_brace_span":s(child_end-2,child_end-1),"close_paren_span":s(child_end-1,child_end)}}));
        id
    };
    let mut elements = vec![child_id];
    if invalid {
        let bad = replacement.find("bad").expect("later invalid child");
        elements.push(expressions.len());
        expressions.push(json!({"span":s(bad,bad+3),"kind":{"kind":"reference","name":{"text":"bad","span":s(bad,bad+3)}}}));
    }
    let root = expressions.len();
    expressions.push(json!({"span":s(0,replacement.len()),"kind":{"kind":"vec-construction","type_syntax":6,
        "open_paren_span":s(14,15),"open_bracket_span":s(15,16),"elements":elements,
        "close_bracket_span":s(replacement.len()-2,replacement.len()-1),"close_paren_span":s(replacement.len()-1,replacement.len())}}));
    body["statements"][1]["kind"]["value"] = json!(root);
    let raw: RawProjectSyntaxSnapshot =
        serde_json::from_value(encoded).expect("fixed mixed phase DTO");
    assert!(
        raw.files[0].functions[0]
            .body
            .expressions
            .iter()
            .any(|e| matches!(e.kind, RawExpressionKind::VecConstruction { .. }))
    );
    (source, raw)
}
