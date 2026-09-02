use super::*;

pub(super) const VEC_ASSIGN_STRING_SOURCE: &str = "function a(): Vec<String> { let x: Vec<String> = Vec<String>([\"a\"]); x = Vec<String>([\"b\"]); return x; }";
pub(super) const VEC_ASSIGN_STRING_RESPONSE: &str = r#"{"id":210,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":18,"end":24},"kind":{"kind":"string","keyword_span":{"file":0,"start":18,"end":24}}},{"span":{"file":0,"start":14,"end":25},"kind":{"kind":"vec","keyword_span":{"file":0,"start":14,"end":17},"less_than_span":{"file":0,"start":17,"end":18},"argument":0,"greater_than_span":{"file":0,"start":24,"end":25}}},{"span":{"file":0,"start":39,"end":45},"kind":{"kind":"string","keyword_span":{"file":0,"start":39,"end":45}}},{"span":{"file":0,"start":35,"end":46},"kind":{"kind":"vec","keyword_span":{"file":0,"start":35,"end":38},"less_than_span":{"file":0,"start":38,"end":39},"argument":2,"greater_than_span":{"file":0,"start":45,"end":46}}},{"span":{"file":0,"start":53,"end":59},"kind":{"kind":"string","keyword_span":{"file":0,"start":53,"end":59}}},{"span":{"file":0,"start":49,"end":60},"kind":{"kind":"vec","keyword_span":{"file":0,"start":49,"end":52},"less_than_span":{"file":0,"start":52,"end":53},"argument":4,"greater_than_span":{"file":0,"start":59,"end":60}}},{"span":{"file":0,"start":77,"end":83},"kind":{"kind":"string","keyword_span":{"file":0,"start":77,"end":83}}},{"span":{"file":0,"start":73,"end":84},"kind":{"kind":"vec","keyword_span":{"file":0,"start":73,"end":76},"less_than_span":{"file":0,"start":76,"end":77},"argument":6,"greater_than_span":{"file":0,"start":83,"end":84}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":104},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"a","span":{"file":0,"start":9,"end":10}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":26,"end":104},"root_block":0,"blocks":[{"span":{"file":0,"start":26,"end":104},"open_brace_span":{"file":0,"start":26,"end":27},"statements":[0,1,2],"close_brace_span":{"file":0,"start":103,"end":104}}],"statements":[{"span":{"file":0,"start":28,"end":68},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":28,"end":31},"mutable":true,"name":{"text":"x","span":{"file":0,"start":32,"end":33}},"type_syntax":3,"equals_span":{"file":0,"start":47,"end":48},"initializer":1,"semicolon_span":{"file":0,"start":67,"end":68}}},{"span":{"file":0,"start":69,"end":92},"kind":{"kind":"assignment","target":2,"equals_span":{"file":0,"start":71,"end":72},"value":4,"semicolon_span":{"file":0,"start":91,"end":92}}},{"span":{"file":0,"start":93,"end":102},"kind":{"kind":"return","keyword_span":{"file":0,"start":93,"end":99},"value":5,"semicolon_span":{"file":0,"start":101,"end":102}}}],"expressions":[{"span":{"file":0,"start":62,"end":65},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":49,"end":67},"kind":{"kind":"vec-construction","type_syntax":5,"open_paren_span":{"file":0,"start":60,"end":61},"open_bracket_span":{"file":0,"start":61,"end":62},"elements":[0],"close_bracket_span":{"file":0,"start":65,"end":66},"close_paren_span":{"file":0,"start":66,"end":67}}},{"span":{"file":0,"start":69,"end":70},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":69,"end":70}}}},{"span":{"file":0,"start":86,"end":89},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":73,"end":91},"kind":{"kind":"vec-construction","type_syntax":7,"open_paren_span":{"file":0,"start":84,"end":85},"open_bracket_span":{"file":0,"start":85,"end":86},"elements":[3],"close_bracket_span":{"file":0,"start":89,"end":90},"close_paren_span":{"file":0,"start":90,"end":91}}},{"span":{"file":0,"start":100,"end":101},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":100,"end":101}}}}]}}]}],"diagnostics":[]}}"#;
pub(super) const VEC_ASSIGN_I32_SOURCE: &str = "function a(): Vec<i32> { let x: Vec<i32> = Vec<i32>([]); const y: Vec<i32> = Vec<i32>([]); x = y; return x; }";
pub(super) const VEC_ASSIGN_I32_RESPONSE: &str = r#"{"id":211,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":18,"end":21},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":18,"end":21}}}},{"span":{"file":0,"start":14,"end":22},"kind":{"kind":"vec","keyword_span":{"file":0,"start":14,"end":17},"less_than_span":{"file":0,"start":17,"end":18},"argument":0,"greater_than_span":{"file":0,"start":21,"end":22}}},{"span":{"file":0,"start":36,"end":39},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":36,"end":39}}}},{"span":{"file":0,"start":32,"end":40},"kind":{"kind":"vec","keyword_span":{"file":0,"start":32,"end":35},"less_than_span":{"file":0,"start":35,"end":36},"argument":2,"greater_than_span":{"file":0,"start":39,"end":40}}},{"span":{"file":0,"start":47,"end":50},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":47,"end":50}}}},{"span":{"file":0,"start":43,"end":51},"kind":{"kind":"vec","keyword_span":{"file":0,"start":43,"end":46},"less_than_span":{"file":0,"start":46,"end":47},"argument":4,"greater_than_span":{"file":0,"start":50,"end":51}}},{"span":{"file":0,"start":70,"end":73},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":70,"end":73}}}},{"span":{"file":0,"start":66,"end":74},"kind":{"kind":"vec","keyword_span":{"file":0,"start":66,"end":69},"less_than_span":{"file":0,"start":69,"end":70},"argument":6,"greater_than_span":{"file":0,"start":73,"end":74}}},{"span":{"file":0,"start":81,"end":84},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":81,"end":84}}}},{"span":{"file":0,"start":77,"end":85},"kind":{"kind":"vec","keyword_span":{"file":0,"start":77,"end":80},"less_than_span":{"file":0,"start":80,"end":81},"argument":8,"greater_than_span":{"file":0,"start":84,"end":85}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":109},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"a","span":{"file":0,"start":9,"end":10}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":23,"end":109},"root_block":0,"blocks":[{"span":{"file":0,"start":23,"end":109},"open_brace_span":{"file":0,"start":23,"end":24},"statements":[0,1,2,3],"close_brace_span":{"file":0,"start":108,"end":109}}],"statements":[{"span":{"file":0,"start":25,"end":56},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":25,"end":28},"mutable":true,"name":{"text":"x","span":{"file":0,"start":29,"end":30}},"type_syntax":3,"equals_span":{"file":0,"start":41,"end":42},"initializer":0,"semicolon_span":{"file":0,"start":55,"end":56}}},{"span":{"file":0,"start":57,"end":90},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":57,"end":62},"mutable":false,"name":{"text":"y","span":{"file":0,"start":63,"end":64}},"type_syntax":7,"equals_span":{"file":0,"start":75,"end":76},"initializer":1,"semicolon_span":{"file":0,"start":89,"end":90}}},{"span":{"file":0,"start":91,"end":97},"kind":{"kind":"assignment","target":2,"equals_span":{"file":0,"start":93,"end":94},"value":3,"semicolon_span":{"file":0,"start":96,"end":97}}},{"span":{"file":0,"start":98,"end":107},"kind":{"kind":"return","keyword_span":{"file":0,"start":98,"end":104},"value":4,"semicolon_span":{"file":0,"start":106,"end":107}}}],"expressions":[{"span":{"file":0,"start":43,"end":55},"kind":{"kind":"vec-construction","type_syntax":5,"open_paren_span":{"file":0,"start":51,"end":52},"open_bracket_span":{"file":0,"start":52,"end":53},"elements":[],"close_bracket_span":{"file":0,"start":53,"end":54},"close_paren_span":{"file":0,"start":54,"end":55}}},{"span":{"file":0,"start":77,"end":89},"kind":{"kind":"vec-construction","type_syntax":9,"open_paren_span":{"file":0,"start":85,"end":86},"open_bracket_span":{"file":0,"start":86,"end":87},"elements":[],"close_bracket_span":{"file":0,"start":87,"end":88},"close_paren_span":{"file":0,"start":88,"end":89}}},{"span":{"file":0,"start":91,"end":92},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":91,"end":92}}}},{"span":{"file":0,"start":95,"end":96},"kind":{"kind":"reference","name":{"text":"y","span":{"file":0,"start":95,"end":96}}}},{"span":{"file":0,"start":105,"end":106},"kind":{"kind":"reference","name":{"text":"x","span":{"file":0,"start":105,"end":106}}}}]}}]}],"diagnostics":[]}}"#;
pub(super) const VEC_STRING_SOURCE: &str = "function make(): Vec<String> { const first: String = \"a\"; const values: Vec<String> = Vec<String>([first, \"b\"]); return values; }";
pub(super) const VEC_STRING_RESPONSE: &str = r#"{"id":30,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":21,"end":27},"kind":{"kind":"string","keyword_span":{"file":0,"start":21,"end":27}}},{"span":{"file":0,"start":17,"end":28},"kind":{"kind":"vec","keyword_span":{"file":0,"start":17,"end":20},"less_than_span":{"file":0,"start":20,"end":21},"argument":0,"greater_than_span":{"file":0,"start":27,"end":28}}},{"span":{"file":0,"start":44,"end":50},"kind":{"kind":"string","keyword_span":{"file":0,"start":44,"end":50}}},{"span":{"file":0,"start":76,"end":82},"kind":{"kind":"string","keyword_span":{"file":0,"start":76,"end":82}}},{"span":{"file":0,"start":72,"end":83},"kind":{"kind":"vec","keyword_span":{"file":0,"start":72,"end":75},"less_than_span":{"file":0,"start":75,"end":76},"argument":3,"greater_than_span":{"file":0,"start":82,"end":83}}},{"span":{"file":0,"start":90,"end":96},"kind":{"kind":"string","keyword_span":{"file":0,"start":90,"end":96}}},{"span":{"file":0,"start":86,"end":97},"kind":{"kind":"vec","keyword_span":{"file":0,"start":86,"end":89},"less_than_span":{"file":0,"start":89,"end":90},"argument":5,"greater_than_span":{"file":0,"start":96,"end":97}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":129},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"make","span":{"file":0,"start":9,"end":13}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":29,"end":129},"root_block":0,"blocks":[{"span":{"file":0,"start":29,"end":129},"open_brace_span":{"file":0,"start":29,"end":30},"statements":[0,1,2],"close_brace_span":{"file":0,"start":128,"end":129}}],"statements":[{"span":{"file":0,"start":31,"end":57},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":31,"end":36},"mutable":false,"name":{"text":"first","span":{"file":0,"start":37,"end":42}},"type_syntax":2,"equals_span":{"file":0,"start":51,"end":52},"initializer":0,"semicolon_span":{"file":0,"start":56,"end":57}}},{"span":{"file":0,"start":58,"end":112},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":58,"end":63},"mutable":false,"name":{"text":"values","span":{"file":0,"start":64,"end":70}},"type_syntax":4,"equals_span":{"file":0,"start":84,"end":85},"initializer":3,"semicolon_span":{"file":0,"start":111,"end":112}}},{"span":{"file":0,"start":113,"end":127},"kind":{"kind":"return","keyword_span":{"file":0,"start":113,"end":119},"value":4,"semicolon_span":{"file":0,"start":126,"end":127}}}],"expressions":[{"span":{"file":0,"start":53,"end":56},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":99,"end":104},"kind":{"kind":"reference","name":{"text":"first","span":{"file":0,"start":99,"end":104}}}},{"span":{"file":0,"start":106,"end":109},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":86,"end":111},"kind":{"kind":"vec-construction","type_syntax":6,"open_paren_span":{"file":0,"start":97,"end":98},"open_bracket_span":{"file":0,"start":98,"end":99},"elements":[1,2],"close_bracket_span":{"file":0,"start":109,"end":110},"close_paren_span":{"file":0,"start":110,"end":111}}},{"span":{"file":0,"start":120,"end":126},"kind":{"kind":"reference","name":{"text":"values","span":{"file":0,"start":120,"end":126}}}}]}}]}],"diagnostics":[]}}"#;
pub(super) const VEC_PUSH_SOURCE: &str = "function append(): Vec<String> { let values: Vec<String> = Vec<String>([\"a\"]); push(values, \"b\"); return values; }";
pub(super) const VEC_PUSH_RESPONSE: &str = r#"{"id":40,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":23,"end":29},"kind":{"kind":"string","keyword_span":{"file":0,"start":23,"end":29}}},{"span":{"file":0,"start":19,"end":30},"kind":{"kind":"vec","keyword_span":{"file":0,"start":19,"end":22},"less_than_span":{"file":0,"start":22,"end":23},"argument":0,"greater_than_span":{"file":0,"start":29,"end":30}}},{"span":{"file":0,"start":49,"end":55},"kind":{"kind":"string","keyword_span":{"file":0,"start":49,"end":55}}},{"span":{"file":0,"start":45,"end":56},"kind":{"kind":"vec","keyword_span":{"file":0,"start":45,"end":48},"less_than_span":{"file":0,"start":48,"end":49},"argument":2,"greater_than_span":{"file":0,"start":55,"end":56}}},{"span":{"file":0,"start":63,"end":69},"kind":{"kind":"string","keyword_span":{"file":0,"start":63,"end":69}}},{"span":{"file":0,"start":59,"end":70},"kind":{"kind":"vec","keyword_span":{"file":0,"start":59,"end":62},"less_than_span":{"file":0,"start":62,"end":63},"argument":4,"greater_than_span":{"file":0,"start":69,"end":70}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":114},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"append","span":{"file":0,"start":9,"end":15}},"parameters":[],"result_type":1,"body":{"span":{"file":0,"start":31,"end":114},"root_block":0,"blocks":[{"span":{"file":0,"start":31,"end":114},"open_brace_span":{"file":0,"start":31,"end":32},"statements":[0,1,2],"close_brace_span":{"file":0,"start":113,"end":114}}],"statements":[{"span":{"file":0,"start":33,"end":78},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":33,"end":36},"mutable":true,"name":{"text":"values","span":{"file":0,"start":37,"end":43}},"type_syntax":3,"equals_span":{"file":0,"start":57,"end":58},"initializer":1,"semicolon_span":{"file":0,"start":77,"end":78}}},{"span":{"file":0,"start":79,"end":97},"kind":{"kind":"expression-statement","expression":4,"semicolon_span":{"file":0,"start":96,"end":97}}},{"span":{"file":0,"start":98,"end":112},"kind":{"kind":"return","keyword_span":{"file":0,"start":98,"end":104},"value":5,"semicolon_span":{"file":0,"start":111,"end":112}}}],"expressions":[{"span":{"file":0,"start":72,"end":75},"kind":{"kind":"string-literal","spelling":"\"a\""}},{"span":{"file":0,"start":59,"end":77},"kind":{"kind":"vec-construction","type_syntax":5,"open_paren_span":{"file":0,"start":70,"end":71},"open_bracket_span":{"file":0,"start":71,"end":72},"elements":[0],"close_bracket_span":{"file":0,"start":75,"end":76},"close_paren_span":{"file":0,"start":76,"end":77}}},{"span":{"file":0,"start":84,"end":90},"kind":{"kind":"reference","name":{"text":"values","span":{"file":0,"start":84,"end":90}}}},{"span":{"file":0,"start":92,"end":95},"kind":{"kind":"string-literal","spelling":"\"b\""}},{"span":{"file":0,"start":79,"end":96},"kind":{"kind":"vec-push","keyword_span":{"file":0,"start":79,"end":83},"open_paren_span":{"file":0,"start":83,"end":84},"vector":2,"comma_span":{"file":0,"start":90,"end":91},"value":3,"close_paren_span":{"file":0,"start":95,"end":96}}},{"span":{"file":0,"start":105,"end":111},"kind":{"kind":"reference","name":{"text":"values","span":{"file":0,"start":105,"end":111}}}}]}}]}],"diagnostics":[]}}"#;
pub(super) const VEC_INDEX_SOURCE: &str =
    "function get(): i32 { const values: Vec<i32> = Vec<i32>([10, 20]); return values[-1]; }";
pub(super) const VEC_INDEX_RESPONSE: &str = r#"{"id":41,"result":{"schema_version":4,"files":[{"id":0,"path":"src/main.zry","imports":[],"type_syntax":[{"span":{"file":0,"start":16,"end":19},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":16,"end":19}}}},{"span":{"file":0,"start":40,"end":43},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":40,"end":43}}}},{"span":{"file":0,"start":36,"end":44},"kind":{"kind":"vec","keyword_span":{"file":0,"start":36,"end":39},"less_than_span":{"file":0,"start":39,"end":40},"argument":1,"greater_than_span":{"file":0,"start":43,"end":44}}},{"span":{"file":0,"start":51,"end":54},"kind":{"kind":"named","name":{"text":"i32","span":{"file":0,"start":51,"end":54}}},{"span":{"file":0,"start":47,"end":55},"kind":{"kind":"vec","keyword_span":{"file":0,"start":47,"end":50},"less_than_span":{"file":0,"start":50,"end":51},"argument":3,"greater_than_span":{"file":0,"start":54,"end":55}}}],"data_declarations":[],"functions":[{"span":{"file":0,"start":0,"end":87},"export_span":null,"function_span":{"file":0,"start":0,"end":8},"name":{"text":"get","span":{"file":0,"start":9,"end":12}},"parameters":[],"result_type":0,"body":{"span":{"file":0,"start":20,"end":87},"root_block":0,"blocks":[{"span":{"file":0,"start":20,"end":87},"open_brace_span":{"file":0,"start":20,"end":21},"statements":[0,1],"close_brace_span":{"file":0,"start":86,"end":87}}],"statements":[{"span":{"file":0,"start":22,"end":66},"kind":{"kind":"local-declaration","keyword_span":{"file":0,"start":22,"end":27},"mutable":false,"name":{"text":"values","span":{"file":0,"start":28,"end":34}},"type_syntax":2,"equals_span":{"file":0,"start":45,"end":46},"initializer":2,"semicolon_span":{"file":0,"start":65,"end":66}}},{"span":{"file":0,"start":67,"end":85},"kind":{"kind":"return","keyword_span":{"file":0,"start":67,"end":73},"value":5,"semicolon_span":{"file":0,"start":84,"end":85}}}],"expressions":[{"span":{"file":0,"start":57,"end":59},"kind":{"kind":"i32-literal","spelling":"10"}},{"span":{"file":0,"start":61,"end":63},"kind":{"kind":"i32-literal","spelling":"20"}},{"span":{"file":0,"start":47,"end":65},"kind":{"kind":"vec-construction","type_syntax":4,"open_paren_span":{"file":0,"start":55,"end":56},"open_bracket_span":{"file":0,"start":56,"end":57},"elements":[0,1],"close_bracket_span":{"file":0,"start":63,"end":64},"close_paren_span":{"file":0,"start":64,"end":65}}},{"span":{"file":0,"start":74,"end":80},"kind":{"kind":"reference","name":{"text":"values","span":{"file":0,"start":74,"end":80}}}},{"span":{"file":0,"start":81,"end":83},"kind":{"kind":"i32-literal","spelling":"-1"}},{"span":{"file":0,"start":74,"end":84},"kind":{"kind":"index","base":3,"open_bracket_span":{"file":0,"start":80,"end":81},"index":4,"close_bracket_span":{"file":0,"start":83,"end":84}}}]}}]}],"diagnostics":[]}}"#;
#[allow(clippy::too_many_lines)]
pub(super) fn private_vec_nested_string_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let mut source = VEC_PUSH_SOURCE.to_owned();
    let mut raw = response_snapshot(VEC_PUSH_RESPONSE);
    let replacements = [("\"a\"", "concat(\"a\", \"b\")"), ("\"b\"", "concat(\"c\", \"d\")")];
    for (ordinal, (old, replacement)) in replacements.into_iter().enumerate() {
        let found = if ordinal == 0 { source.find(old) } else { source.rfind(old) };
        let start = u32::try_from(found.expect("Vec String literal")).expect("offset");
        let old_end = start + u32::try_from(old.len()).expect("length");
        let extra = u32::try_from(replacement.len() - old.len()).expect("growth");
        source.replace_range(
            usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
            replacement,
        );
        raw = shift_snapshot(raw, old_end, extra);
        let body = &mut raw.files[0].functions[0].body;
        let inner = if ordinal == 0 { 0 } else { 3 };
        let first = zryna_source::UntrustedSpan { file: 0, start: start + 7, end: start + 10 };
        body.expressions[inner] = RawExpressionSyntax {
            span: first,
            kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                spelling: if ordinal == 0 { "\"a\"" } else { "\"c\"" }.to_owned(),
            },
        };
        let second_id = u32::try_from(body.expressions.len()).expect("second literal id");
        let second = zryna_source::UntrustedSpan { file: 0, start: start + 12, end: start + 15 };
        body.expressions.push(RawExpressionSyntax {
            span: second,
            kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                spelling: if ordinal == 0 { "\"b\"" } else { "\"d\"" }.to_owned(),
            },
        });
        let concat_id = u32::try_from(body.expressions.len()).expect("concat id");
        body.expressions.push(RawExpressionSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start, end: start + 16 },
            kind: zryna_syntax::v4::RawExpressionKind::Call {
                callee: RawIdentifierSyntax {
                    text: "concat".to_owned(),
                    span: zryna_source::UntrustedSpan { file: 0, start, end: start + 6 },
                },
                open_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: start + 6,
                    end: start + 7,
                },
                arguments: vec![u32::try_from(inner).expect("inner id"), second_id],
                close_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: start + 15,
                    end: start + 16,
                },
            },
        });
        if ordinal == 0 {
            let zryna_syntax::v4::RawExpressionKind::VecConstruction { elements, .. } =
                &mut body.expressions[1].kind
            else {
                panic!("Vec construction")
            };
            *elements = vec![concat_id];
        } else {
            let zryna_syntax::v4::RawExpressionKind::VecPush { value, .. } =
                &mut body.expressions[4].kind
            else {
                panic!("Vec push")
            };
            *value = concat_id;
        }
    }
    let body = &mut raw.files[0].functions[0].body;
    let mut vec_construct = body.expressions[1].clone();
    let vector_reference = body.expressions[2].clone();
    let mut push = body.expressions[4].clone();
    let return_reference = body.expressions[5].clone();
    let mut first_concat = body.expressions[7].clone();
    let mut second_concat = body.expressions[9].clone();
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut first_concat.kind else {
        panic!("first concat")
    };
    *arguments = vec![0, 1];
    let zryna_syntax::v4::RawExpressionKind::VecConstruction { elements, .. } =
        &mut vec_construct.kind
    else {
        panic!("Vec construction")
    };
    *elements = vec![2];
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut second_concat.kind
    else {
        panic!("second concat")
    };
    *arguments = vec![5, 6];
    let zryna_syntax::v4::RawExpressionKind::VecPush { vector, value, .. } = &mut push.kind else {
        panic!("Vec push")
    };
    *vector = 4;
    *value = 7;
    body.expressions = vec![
        body.expressions[0].clone(),
        body.expressions[6].clone(),
        first_concat,
        vec_construct,
        vector_reference,
        body.expressions[3].clone(),
        body.expressions[8].clone(),
        second_concat,
        push,
        return_reference,
    ];
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[0].kind
    else {
        panic!("Vec declaration")
    };
    *initializer = 3;
    let RawStatementKind::ExpressionStatement { expression, .. } = &mut body.statements[1].kind
    else {
        panic!("push statement")
    };
    *expression = 8;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("Vec return")
    };
    *value = 9;
    (source, raw)
}

#[allow(clippy::too_many_lines)]
pub(super) fn private_vec_clone_fixture(element: &str) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;

    let elements = if element == "String" { "[\"a\", \"b\", \"c\"]" } else { "[]" };
    let source = format!(
        "function copy(): Vec<{element}> {{ const source: Vec<{element}> = Vec<{element}>({elements}); return clone(source); }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&source, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&source, (start, start_ordinal), (end, end_ordinal))
    };
    let spelling = format!("Vec<{element}>");
    let mut types = Vec::new();
    let mut vec_types = Vec::new();
    for ordinal in 0..3 {
        let vec_span = token(&spelling, ordinal);
        let element_span = zryna_source::UntrustedSpan {
            file: 0,
            start: vec_span.start + 4,
            end: vec_span.end - 1,
        };
        let element_type = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: element_span,
            kind: if element == "String" {
                RawTypeSyntaxKind::String { keyword_span: element_span }
            } else {
                RawTypeSyntaxKind::Named {
                    name: RawIdentifierSyntax { text: element.to_owned(), span: element_span },
                }
            },
        });
        let vec_type = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: vec_span,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: vec_span.start,
                    end: vec_span.start + 3,
                },
                less_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: vec_span.start + 3,
                    end: vec_span.start + 4,
                },
                argument: element_type,
                greater_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: vec_span.end - 1,
                    end: vec_span.end,
                },
            },
        });
        vec_types.push(vec_type);
    }
    let root = range("{", 0, "}", 0);
    let local = range("const", 0, ";", 0);
    let returned = range("return", 0, ";", 1);
    let mut expressions = Vec::new();
    let element_ids = if element == "String" {
        ["\"a\"", "\"b\"", "\"c\""]
            .into_iter()
            .map(|spelling| {
                let id = u32::try_from(expressions.len()).expect("expression id");
                expressions.push(RawExpressionSyntax {
                    span: token(spelling, 0),
                    kind: RawExpressionKind::StringLiteral { spelling: spelling.to_owned() },
                });
                id
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let construct = u32::try_from(expressions.len()).expect("expression id");
    expressions.push(RawExpressionSyntax {
        span: range(&spelling, 2, ")", 1),
        kind: RawExpressionKind::VecConstruction {
            type_syntax: vec_types[2],
            open_paren_span: token("(", 1),
            open_bracket_span: token("[", 0),
            elements: element_ids,
            close_bracket_span: token("]", 0),
            close_paren_span: token(")", 1),
        },
    });
    let reference = u32::try_from(expressions.len()).expect("expression id");
    expressions.push(RawExpressionSyntax {
        span: token("source", 1),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "source".to_owned(), span: token("source", 1) },
        },
    });
    let cloned = u32::try_from(expressions.len()).expect("expression id");
    expressions.push(RawExpressionSyntax {
        span: range("clone", 0, ")", 2),
        kind: RawExpressionKind::Clone {
            keyword_span: token("clone", 0),
            open_paren_span: token("(", 2),
            value: reference,
            close_paren_span: token(")", 2),
        },
    });
    let statements = vec![
        RawStatementSyntax {
            span: local,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "source".to_owned(), span: token("source", 0) },
                type_syntax: vec_types[1],
                equals_span: token("=", 0),
                initializer: construct,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: returned,
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: cloned,
                semicolon_span: token(";", 1),
            },
        },
    ];
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(source.len()).expect("source length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "copy".to_owned(), span: token("copy", 0) },
        parameters: Vec::new(),
        result_type: vec_types[0],
        body: RawFunctionBodySyntax {
            span: root,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: root,
                open_brace_span: token("{", 0),
                statements: vec![0, 1],
                close_brace_span: token("}", 0),
            }],
            statements,
            expressions,
        },
    };
    (
        source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}
