use super::*;

fn unary(f: &mut Builder, spelling: &str, value: impl FnOnce(&mut Builder) -> u32) -> u32 {
    let start = f.source.len();
    let keyword_span = f.text(spelling);
    let open_paren_span = f.text("(");
    let value = value(f);
    let close_paren_span = f.text(")");
    let kind = match spelling {
        "shared" => {
            RawExpressionKind::Shared { keyword_span, open_paren_span, value, close_paren_span }
        }
        "clone" => {
            RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span }
        }
        "downgrade" => {
            RawExpressionKind::Downgrade { keyword_span, open_paren_span, value, close_paren_span }
        }
        _ => unreachable!("fixture unary operation"),
    };
    f.expression(start, kind)
}

fn local(f: &mut Builder, name: &str, ty: &Ty, initializer: impl FnOnce(&mut Builder) -> u32) {
    let start = f.source.len();
    let keyword_span = f.text("const");
    f.text(" ");
    let name = f.name(name);
    f.text(": ");
    let type_syntax = f.ty(ty);
    f.text(" ");
    let equals_span = f.text("=");
    f.text(" ");
    let initializer = initializer(f);
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(start, f.source.len()),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span,
        },
    });
    f.text(" ");
}

#[derive(Clone, Copy)]
pub(crate) enum Case {
    Positive,
    NestedShared,
    ScalarBool,
    ScalarI32,
    MovedReuse,
    WrongCloneType,
}

pub(crate) fn fixture() -> (String, RawProjectSyntaxSnapshot) {
    fixture_case(Case::Positive)
}

pub(crate) fn fixture_case(case: Case) -> (String, RawProjectSyntaxSnapshot) {
    let mut f = Builder {
        source: String::new(),
        types: Vec::new(),
        expressions: Vec::new(),
        statements: Vec::new(),
    };
    let parameter = match case {
        Case::ScalarBool => Ty::Named("bool"),
        Case::ScalarI32 => Ty::Named("i32"),
        _ => Ty::String,
    };
    let payload = if matches!(case, Case::NestedShared) {
        Ty::Shared(Box::new(parameter.clone()))
    } else {
        parameter.clone()
    };
    let shared = Ty::Shared(Box::new(payload.clone()));
    let weak = Ty::Weak(Box::new(payload.clone()));
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("handles");
    f.text("(");
    let parameters = vec![f.parameter("payload", &parameter)];
    f.text("): ");
    let result_type = f.ty(&shared);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    if matches!(case, Case::NestedShared) {
        local(&mut f, "inner", &payload, |f| unary(f, "shared", |f| f.reference("payload")));
        local(&mut f, "owner", &shared, |f| unary(f, "shared", |f| f.reference("inner")));
    } else {
        local(&mut f, "owner", &shared, |f| unary(f, "shared", |f| f.reference("payload")));
    }
    match case {
        Case::Positive | Case::NestedShared | Case::ScalarBool | Case::ScalarI32 => {
            local(&mut f, "copy", &shared, |f| unary(f, "clone", |f| f.reference("owner")));
            local(&mut f, "weak", &weak, |f| unary(f, "downgrade", |f| f.reference("copy")));
            local(&mut f, "weakCopy", &weak, |f| unary(f, "clone", |f| f.reference("weak")));
        }
        Case::MovedReuse => {
            local(&mut f, "moved", &shared, |f| f.reference("owner"));
            local(&mut f, "copy", &shared, |f| unary(f, "clone", |f| f.reference("owner")));
        }
        Case::WrongCloneType => {
            local(&mut f, "weak", &weak, |f| unary(f, "downgrade", |f| f.reference("owner")));
            local(&mut f, "copy", &shared, |f| unary(f, "clone", |f| f.reference("weak")));
        }
    }
    let return_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = f.reference(if matches!(case, Case::MovedReuse) { "moved" } else { "owner" });
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start as usize, f.source.len());
    let statements = std::mem::take(&mut f.statements);
    let function = RawFunctionSyntax {
        span: at(start, f.source.len()),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                statements: (0..statements.len())
                    .map(|id| u32::try_from(id).expect("statement"))
                    .collect(),
                close_brace_span,
            }],
            statements,
            expressions: f.expressions,
        },
    };
    (
        f.source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: Vec::new(),
                type_syntax: f.types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}
