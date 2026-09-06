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
        "downgrade" => {
            RawExpressionKind::Downgrade { keyword_span, open_paren_span, value, close_paren_span }
        }
        "clone" => {
            RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span }
        }
        _ => unreachable!("fixture unary"),
    };
    f.expression(start, kind)
}

fn local(f: &mut Builder, name: &str, ty: &Ty, value: impl FnOnce(&mut Builder) -> u32) {
    let start = f.source.len();
    let keyword_span = f.text("const");
    f.text(" ");
    let name = f.name(name);
    f.text(": ");
    let type_syntax = f.ty(ty);
    f.text(" ");
    let equals_span = f.text("=");
    f.text(" ");
    let value = value(f);
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(start, f.source.len()),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span,
            initializer: value,
            semicolon_span,
        },
    });
    f.text(" ");
}

fn returned(f: &mut Builder, name: &str) {
    let start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = f.reference(name);
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
}

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1) enum Case {
    Addressable,
    Temporary,
    WrongType,
    Missing,
    ExpiredBindingUse,
    Moved,
    Reused,
    ActiveBorrow,
    UnequalJoin,
    RetainedJoin,
}

pub(in crate::data_ownership_v1) fn fixture(temporary: bool) -> (String, RawProjectSyntaxSnapshot) {
    fixture_case(if temporary { Case::Temporary } else { Case::Addressable })
}

pub(in crate::data_ownership_v1) fn fixture_case(case: Case) -> (String, RawProjectSyntaxSnapshot) {
    let mut f = Builder {
        source: String::new(),
        types: Vec::new(),
        expressions: Vec::new(),
        statements: Vec::new(),
    };
    let string = Ty::String;
    let shared = Ty::Shared(Box::new(string.clone()));
    let weak = Ty::Weak(Box::new(string.clone()));
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("upgrade");
    f.text("(");
    let parameters = vec![f.parameter("payload", &string)];
    f.text("): ");
    let result_type = f.ty(&shared);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");

    local(&mut f, "owner", &shared, |f| unary(f, "shared", |f| f.reference("payload")));
    local(&mut f, "weak", &weak, |f| unary(f, "downgrade", |f| f.reference("owner")));

    if matches!(case, Case::Moved | Case::Reused) {
        local(&mut f, "saved", &weak, |f| f.reference("weak"));
    }
    if matches!(case, Case::Reused) {
        local(&mut f, "again", &weak, |f| f.reference("weak"));
    }
    if matches!(case, Case::ActiveBorrow) {
        active_borrow(&mut f, &weak);
    }
    let (root_statements, success_block, expired_block) = upgrade_blocks(&mut f, case);
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start as usize, f.source.len());
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
            blocks: vec![
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span,
                    statements: root_statements,
                    close_brace_span,
                },
                success_block,
                expired_block,
            ],
            statements: f.statements,
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

fn upgrade_blocks(f: &mut Builder, case: Case) -> (Vec<u32>, RawBlockSyntax, RawBlockSyntax) {
    let upgrade_id = f.statements.len();

    let upgrade_start = f.source.len();
    let keyword_span = f.text("upgradeWeak");
    f.text(" ");
    let weak = match case {
        Case::Temporary => unary(f, "clone", |f| f.reference("weak")),
        Case::WrongType => f.reference("owner"),
        Case::Missing => f.reference("ghost"),
        _ => f.reference("weak"),
    };
    f.text(" ");
    let binding = f.name("upgraded");
    f.text(" ");
    let as_span = f.text("=>");
    f.text(" ");
    let success_open = f.text("{");
    f.text(" ");
    let success_statement = u32::try_from(f.statements.len() + 1).expect("success statement");
    if matches!(case, Case::UnequalJoin) {
        local(f, "taken", &weak_type(), |f| f.reference("weak"));
    } else if !matches!(case, Case::RetainedJoin) {
        returned(f, "upgraded");
    }
    let success_close = f.text("}");
    f.text(" ");
    let else_span = f.text("=>");
    f.text(" ");
    let expired_open = f.text("{");
    f.text(" ");
    let expired_statement = u32::try_from(f.statements.len() + 1).expect("expired statement");
    if !matches!(case, Case::UnequalJoin | Case::RetainedJoin) {
        returned(f, if matches!(case, Case::ExpiredBindingUse) { "upgraded" } else { "owner" });
    }
    let expired_close = f.text("}");
    let upgrade = RawStatementSyntax {
        span: at(upgrade_start, f.source.len()),
        kind: RawStatementKind::WeakUpgrade {
            keyword_span,
            weak,
            as_span,
            binding,
            success_block: 1,
            else_span,
            failure_block: 2,
        },
    };
    f.statements.insert(upgrade_id, upgrade);
    let mut root_statements =
        (0..=u32::try_from(upgrade_id).expect("upgrade id")).collect::<Vec<_>>();
    if matches!(case, Case::UnequalJoin | Case::RetainedJoin) {
        f.text(" ");
        root_statements.push(u32::try_from(f.statements.len()).expect("return id"));
        returned(f, "owner");
    }
    (
        root_statements,
        RawBlockSyntax {
            span: at(success_open.start as usize, success_close.end as usize),
            open_brace_span: success_open,
            statements: if matches!(case, Case::RetainedJoin) {
                vec![]
            } else {
                vec![success_statement]
            },
            close_brace_span: success_close,
        },
        RawBlockSyntax {
            span: at(expired_open.start as usize, expired_close.end as usize),
            open_brace_span: expired_open,
            statements: if matches!(case, Case::UnequalJoin | Case::RetainedJoin) {
                vec![]
            } else {
                vec![expired_statement]
            },
            close_brace_span: expired_close,
        },
    )
}

fn weak_type() -> Ty {
    Ty::Weak(Box::new(Ty::String))
}

fn active_borrow(f: &mut Builder, weak: &Ty) {
    let start = f.source.len();
    let keyword_span = f.text("const");
    f.text(" ");
    let name = f.name("loan");
    f.text(": ");
    let type_start = f.source.len();
    let borrow_keyword = f.text("Borrow");
    let open_angle_span = f.text("<");
    let referent = f.ty(weak);
    let close_angle_span = f.text(">");
    let type_syntax = u32::try_from(f.types.len()).expect("borrow type");
    f.types.push(RawTypeSyntax {
        span: at(type_start, f.source.len()),
        kind: RawTypeSyntaxKind::Borrow {
            keyword_span: borrow_keyword,
            less_than_span: open_angle_span,
            argument: referent,
            greater_than_span: close_angle_span,
        },
    });
    f.text(" ");
    let equals_span = f.text("=");
    f.text(" ");
    let borrow_start = f.source.len();
    let keyword = f.text("borrow");
    let open_paren_span = f.text("(");
    let value = f.reference("weak");
    let close_paren_span = f.text(")");
    let initializer = f.expression(
        borrow_start,
        RawExpressionKind::Borrow {
            keyword_span: keyword,
            open_paren_span,
            value,
            close_paren_span,
        },
    );
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
