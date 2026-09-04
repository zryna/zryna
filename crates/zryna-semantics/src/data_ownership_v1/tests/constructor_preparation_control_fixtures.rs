use super::{Fixture, fixtures};
use zryna_syntax::v4::{
    RawDataDeclarationKind, RawExpressionKind, RawIdentifierSyntax, RawParameterSyntax,
    RawProjectSyntaxSnapshot, RawStatementKind, RawTypeSyntax, RawTypeSyntaxKind,
};
pub(super) fn zero_array() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut snapshot) = fixtures::snapshot(Fixture::Array);
    for ty in &mut snapshot.files[0].type_syntax {
        if let RawTypeSyntaxKind::FixedArray { length, length_spelling, length_span, .. } =
            &mut ty.kind
        {
            assert_eq!(*length, 2);
            *length = 0;
            *length_spelling = "0".to_owned();
            source.replace_range(length_span.start as usize..length_span.end as usize, "0");
        }
    }
    let body = &mut snapshot.files[0].functions[0].body;
    let RawExpressionKind::FixedArrayConstruction {
        elements,
        open_bracket_span,
        close_bracket_span,
        ..
    } = &mut body.expressions[2].kind
    else {
        panic!("array fixture")
    };
    assert_eq!(elements, &[0, 1]);
    elements.clear();
    let start = open_bracket_span.end as usize;
    let end = close_bracket_span.start as usize;
    source.replace_range(start..end, &" ".repeat(end - start));
    body.expressions.drain(..2);
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[0].kind
    else {
        panic!("array local")
    };
    *initializer = 0;
    let RawStatementKind::Return { value, .. } = &mut body.statements[1].kind else {
        panic!("array return")
    };
    *value = 1;
    (source, snapshot)
}

// Same source-faithful parameter insertion used by the existing dense-type fixture;
// only syntax spans/IDs are rebuilt here, never parameter lowering or owner state.
fn shift_spans(value: &mut serde_json::Value, cutoff: u32, amount: u32) {
    match value {
        serde_json::Value::Object(object) => {
            if object.contains_key("file")
                && object.contains_key("start")
                && object.contains_key("end")
            {
                for key in ["start", "end"] {
                    let number = object.get_mut(key).expect("span coordinate");
                    let current = number.as_u64().expect("span integer");
                    if current >= u64::from(cutoff) {
                        *number = serde_json::Value::from(current + u64::from(amount));
                    }
                }
            } else {
                for child in object.values_mut() {
                    shift_spans(child, cutoff, amount);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                shift_spans(child, cutoff, amount);
            }
        }
        _ => {}
    }
}

pub(super) fn empty_nested() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut snapshot) = fixtures::snapshot(Fixture::Nested);
    let file = &mut snapshot.files[0];
    let RawDataDeclarationKind::Struct { fields, .. } = &mut file.data_declarations[0].kind else {
        panic!("Inner declaration")
    };
    let field = fields.pop().expect("single String field");
    source.replace_range(
        field.span.start as usize..field.span.end as usize,
        &" ".repeat((field.span.end - field.span.start) as usize),
    );
    assert!(fields.is_empty());
    file.type_syntax.remove(0);
    for declaration in &mut file.data_declarations {
        let RawDataDeclarationKind::Struct { fields, .. } = &mut declaration.kind else {
            panic!("struct fixture")
        };
        for field in fields {
            field.type_syntax -= 1;
        }
    }
    let function = &mut file.functions[0];
    function.result_type -= 1;
    let body = &mut function.body;
    let RawExpressionKind::StructConstruction { fields, .. } = &mut body.expressions[2].kind else {
        panic!("Inner construction")
    };
    let field = fields.pop().expect("single constructor field");
    source.replace_range(
        field.span.start as usize..field.span.end as usize,
        &" ".repeat((field.span.end - field.span.start) as usize),
    );
    body.expressions.remove(1);
    let RawExpressionKind::StructConstruction { fields, .. } = &mut body.expressions[2].kind else {
        panic!("Outer construction")
    };
    for field in fields {
        let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } = &mut field.kind
        else {
            panic!("explicit field")
        };
        if *value > 1 {
            *value -= 1;
        }
    }
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value -= 1;
    (source, snapshot)
}

pub(super) fn repeated_clone() -> (String, RawProjectSyntaxSnapshot) {
    const RETURN: &str = "OwnedPair({ flag: clone(p.first), first: clone(p.first) })";
    let (mut source, snapshot) = fixtures::snapshot(Fixture::Pair);
    let mut json = serde_json::to_value(snapshot).expect("Pair syntax");
    let types = json["files"][0]["type_syntax"].as_array().expect("types");
    let bool_id =
        types.iter().position(|ty| ty["kind"]["name"]["text"] == "bool").expect("bool type");
    let start = usize::try_from(types[bool_id]["span"]["start"].as_u64().expect("start"))
        .expect("bounded type offset");
    source.replace_range(start..start + 4, "String");
    shift_spans(&mut json, u32::try_from(start + 4).expect("type end"), 2);
    let ty = &mut json["files"][0]["type_syntax"][bool_id];
    ty["kind"] = serde_json::json!({"kind":"string", "keyword_span":ty["span"]});
    let body = &mut json["files"][0]["functions"][0]["body"];
    let literal = &mut body["expressions"][0];
    let begin = usize::try_from(literal["span"]["start"].as_u64().expect("literal start"))
        .expect("bounded literal offset");
    source.replace_range(begin..begin + 4, "\"b\" ");
    literal["span"]["end"] = serde_json::json!(begin + 3);
    literal["kind"] = serde_json::json!({"kind":"string-literal", "spelling":"\"b\""});
    let start = source.rfind("p;").expect("final return");
    source.replace_range(start..=start, RETURN);
    shift_spans(
        &mut json,
        u32::try_from(start + 1).expect("return end"),
        u32::try_from(RETURN.len() - 1).expect("growth"),
    );
    let s = |a, b| serde_json::json!({"file":0,"start":start+a,"end":start+b});
    let body = &mut json["files"][0]["functions"][0]["body"];
    let expressions = body["expressions"].as_array_mut().expect("expressions");
    expressions.pop();
    let mut fields = Vec::new();
    for (label, offset) in [("flag", 12usize), ("first", 34usize)] {
        let clone_start = RETURN[offset..].find("clone").expect("clone") + offset;
        let base = expressions.len();
        expressions.push(serde_json::json!({"span":s(clone_start+6,clone_start+7),"kind":{
            "kind":"reference","name":{"text":"p","span":s(clone_start+6,clone_start+7)}}}));
        expressions.push(serde_json::json!({"span":s(clone_start+6,clone_start+13),"kind":{
            "kind":"field-access","base":base,"dot_span":s(clone_start+7,clone_start+8),
            "field":{"text":"first","span":s(clone_start+8,clone_start+13)}}}));
        expressions.push(serde_json::json!({"span":s(clone_start,clone_start+14),"kind":{
            "kind":"clone","keyword_span":s(clone_start,clone_start+5),"open_paren_span":s(clone_start+5,clone_start+6),
            "value":base+1,"close_paren_span":s(clone_start+13,clone_start+14)}}));
        fields.push(serde_json::json!({"span":s(offset,clone_start+14),"kind":{
            "kind":"explicit","name":{"text":label,"span":s(offset,offset+label.len())},
            "colon_span":s(offset+label.len(),offset+label.len()+1),"value":base+2}}));
    }
    let root = expressions.len();
    expressions.push(serde_json::json!({"span":s(0,RETURN.len()),"kind":{
        "kind":"struct-construction","type_name":{"text":"OwnedPair","span":s(0,9)},
        "open_paren_span":s(9,10),"open_brace_span":s(10,11),"fields":fields,
        "close_brace_span":s(RETURN.len()-2,RETURN.len()-1),"close_paren_span":s(RETURN.len()-1,RETURN.len())}}));
    body["statements"][1]["kind"]["value"] = serde_json::json!(root);
    (source, serde_json::from_value(json).expect("two-clone syntax"))
}

pub(super) fn copy_parameter(use_i32: bool) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, snapshot) = fixtures::snapshot(Fixture::Pair);
    let insertion =
        u32::try_from(source.find("()").expect("parameter list") + 1).expect("source offset");
    source.insert_str(insertion as usize, "flag: bool");
    let mut json = serde_json::to_value(snapshot).expect("raw syntax");
    shift_spans(&mut json, insertion, 10);
    let mut snapshot: RawProjectSyntaxSnapshot =
        serde_json::from_value(json).expect("shifted syntax");
    let at = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    snapshot.files[0].type_syntax.insert(
        2,
        RawTypeSyntax {
            span: at(insertion + 6, insertion + 10),
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "bool".to_owned(),
                    span: at(insertion + 6, insertion + 10),
                },
            },
        },
    );
    let function = &mut snapshot.files[0].functions[0];
    function.result_type += 1;
    function.parameters.push(RawParameterSyntax {
        span: at(insertion, insertion + 10),
        name: RawIdentifierSyntax { text: "flag".to_owned(), span: at(insertion, insertion + 4) },
        type_syntax: 2,
    });
    for statement in &mut function.body.statements {
        if let RawStatementKind::LocalDeclaration { type_syntax, .. } = &mut statement.kind {
            *type_syntax += 1;
        }
    }
    let literal = function
        .body
        .expressions
        .iter_mut()
        .find(|expression| matches!(expression.kind, RawExpressionKind::BoolLiteral { .. }))
        .expect("Copy bool child");
    source.replace_range(literal.span.start as usize..literal.span.end as usize, "flag");
    literal.kind = RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "flag".to_owned(), span: literal.span },
    };
    if use_i32 {
        function.parameters[0].span.end -= 1;
        for ty in &mut snapshot.files[0].type_syntax {
            if let RawTypeSyntaxKind::Named { name } = &mut ty.kind
                && name.text == "bool"
            {
                source.replace_range(name.span.start as usize..name.span.end as usize, "i32 ");
                name.text = "i32".to_owned();
                name.span.end -= 1;
                ty.span.end -= 1;
            }
        }
    }
    (source, snapshot)
}
