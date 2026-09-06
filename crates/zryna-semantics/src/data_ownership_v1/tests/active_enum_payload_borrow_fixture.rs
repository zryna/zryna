use super::*;

#[derive(Clone, Copy)]
pub(super) enum Case {
    Shared,
    Exclusive,
    Inactive,
    Foreign,
}

impl Builder {
    fn string_literal(&mut self, text: &str) -> u32 {
        let start = self.source.len();
        let spelling = format!("\"{text}\"");
        self.text(&spelling);
        self.expression(start, RawExpressionKind::StringLiteral { spelling })
    }

    fn string_vec(&mut self, text: &str) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("Vec");
        let less_than_span = self.text("<");
        let argument = self.ty(&Ty::String);
        let greater_than_span = self.text(">");
        let type_syntax = u32::try_from(self.types.len()).expect("Vec type");
        self.types.push(RawTypeSyntax {
            span: at(start, self.source.len()),
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            },
        });
        let open_paren_span = self.text("(");
        let open_bracket_span = self.text("[");
        let element = self.string_literal(text);
        let close_bracket_span = self.text("]");
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::VecConstruction {
                type_syntax,
                open_paren_span,
                open_bracket_span,
                elements: vec![element],
                close_bracket_span,
                close_paren_span,
            },
        )
    }

    fn choice_some(&mut self, text: &str) -> u32 {
        let start = self.source.len();
        let type_name = self.name("Choice");
        let dot_span = self.text(".");
        let variant = self.name("some");
        let open_paren_span = self.text("(");
        let payload = self.string_vec(text);
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::EnumConstruction {
                type_name,
                dot_span,
                variant,
                open_paren_span,
                payload: Some(payload),
                close_paren_span,
            },
        )
    }

    fn root_local(&mut self, choice: &Ty) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("let");
        self.text(" ");
        let name = self.name("item");
        self.text(": ");
        let type_syntax = self.ty(choice);
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let initializer = self.choice_some("initial");
        let semicolon_span = self.text(";");
        let id = u32::try_from(self.statements.len()).expect("statement");
        self.statements.push(RawStatementSyntax {
            span: at(start, self.source.len()),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span,
                mutable: true,
                name,
                type_syntax,
                equals_span,
                initializer,
                semicolon_span,
            },
        });
        self.text(" ");
        id
    }

    fn payload(&mut self, variant_text: &str) -> u32 {
        let start = self.source.len();
        let base = self.reference("item");
        let dot_span = self.text(".");
        let field = self.name(variant_text);
        self.expression(start, RawExpressionKind::FieldAccess { base, dot_span, field })
    }

    fn borrow_alias(&mut self, case: Case, payload: &Ty) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("const");
        self.text(" ");
        let name = self.name("loan");
        self.text(": ");
        let ty_start = self.source.len();
        let exclusive = matches!(case, Case::Exclusive);
        let borrow_keyword = self.text(if exclusive { "BorrowMut" } else { "Borrow" });
        let less_than_span = self.text("<");
        let argument = self.ty(payload);
        let greater_than_span = self.text(">");
        let type_syntax = u32::try_from(self.types.len()).expect("borrow type");
        self.types.push(RawTypeSyntax {
            span: at(ty_start, self.source.len()),
            kind: if exclusive {
                RawTypeSyntaxKind::BorrowMut {
                    keyword_span: borrow_keyword,
                    less_than_span,
                    argument,
                    greater_than_span,
                }
            } else {
                RawTypeSyntaxKind::Borrow {
                    keyword_span: borrow_keyword,
                    less_than_span,
                    argument,
                    greater_than_span,
                }
            },
        });
        self.text(" = ");
        let borrow_start = self.source.len();
        let borrow_keyword = self.text(if exclusive { "borrowMut" } else { "borrow" });
        let open_paren_span = self.text("(");
        let variant = match case {
            Case::Inactive => "none",
            Case::Foreign => "absent",
            Case::Shared | Case::Exclusive => "some",
        };
        let value = self.payload(variant);
        let close_paren_span = self.text(")");
        let initializer = self.expression(
            borrow_start,
            if exclusive {
                RawExpressionKind::BorrowMut {
                    keyword_span: borrow_keyword,
                    open_paren_span,
                    value,
                    close_paren_span,
                }
            } else {
                RawExpressionKind::Borrow {
                    keyword_span: borrow_keyword,
                    open_paren_span,
                    value,
                    close_paren_span,
                }
            },
        );
        let semicolon_span = self.text(";");
        let id = u32::try_from(self.statements.len()).expect("statement");
        self.statements.push(RawStatementSyntax {
            span: at(start, self.source.len()),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span,
                mutable: false,
                name,
                type_syntax,
                equals_span: at(borrow_start - 2, borrow_start - 1),
                initializer,
                semicolon_span,
            },
        });
        self.text(" ");
        id
    }

    fn shared_read(&mut self, payload: &Ty) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("const");
        self.text(" ");
        let name = self.name("seen");
        self.text(": ");
        let type_syntax = self.ty(payload);
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let initializer = self.clone_value(|builder| builder.reference("loan"));
        let semicolon_span = self.text(";");
        let id = u32::try_from(self.statements.len()).expect("statement");
        self.statements.push(RawStatementSyntax {
            span: at(start, self.source.len()),
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
        self.text(" ");
        id
    }

    fn exclusive_replace(&mut self) -> u32 {
        let start = self.source.len();
        let target = self.reference("loan");
        self.text(" = ");
        let value = self.string_vec("replacement");
        let semicolon_span = self.text(";");
        let id = u32::try_from(self.statements.len()).expect("statement");
        self.statements.push(RawStatementSyntax {
            span: at(start, self.source.len()),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: at(start + 5, start + 6),
                value,
                semicolon_span,
            },
        });
        self.text(" ");
        id
    }
}

pub(super) fn fixture(case: Case) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations, choice) = initial(&Element::Enum);
    let payload = Ty::Vec(Box::new(Ty::String));
    f.text("\n");
    let function_start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("observe");
    f.text("(): ");
    let result_type = f.ty(&choice);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let root = f.root_local(&choice);
    let nested = u32::try_from(f.statements.len()).expect("nested statement");
    let inner_open = f.text("{");
    f.statements
        .push(RawStatementSyntax { span: inner_open, kind: RawStatementKind::Block { block: 1 } });
    f.text(" ");
    let alias = f.borrow_alias(case, &payload);
    let action = if matches!(case, Case::Exclusive) {
        f.exclusive_replace()
    } else {
        f.shared_read(&payload)
    };
    let inner_close = f.text("}");
    f.statements[nested as usize].span.end = inner_close.end;
    f.text(" return ");
    let return_start = f.source.len() - "return ".len();
    let value = f.reference("item");
    let semicolon_span = f.text(";");
    let returned = u32::try_from(f.statements.len()).expect("return statement");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return {
            keyword_span: at(return_start, return_start + "return".len()),
            value,
            semicolon_span,
        },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start as usize, close_brace_span.end as usize);
    let function = RawFunctionSyntax {
        span: at(function_start, f.source.len()),
        export_span: None,
        function_span,
        name,
        parameters: Vec::new(),
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span,
                    statements: vec![root, nested, returned],
                    close_brace_span,
                },
                RawBlockSyntax {
                    span: at(inner_open.start as usize, inner_close.end as usize),
                    open_brace_span: inner_open,
                    statements: vec![alias, action],
                    close_brace_span: inner_close,
                },
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
                data_declarations: declarations,
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}
