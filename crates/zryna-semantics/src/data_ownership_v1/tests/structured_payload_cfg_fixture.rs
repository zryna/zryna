use super::*;
use zryna_syntax::v4::RawElseSyntax;

#[path = "structured_payload_cfg_declarations.rs"]
mod declarations;
pub(in crate::data_ownership_v1) use declarations::Payload;

struct Composer {
    f: Builder,
    blocks: Vec<RawBlockSyntax>,
}

impl Composer {
    fn block(&mut self, body: impl FnOnce(&mut Self, &mut Vec<u32>)) -> u32 {
        let start = self.f.source.len();
        let open_brace_span = self.f.text("{");
        self.f.text(" ");
        let id = u32::try_from(self.blocks.len()).expect("block id");
        self.blocks.push(RawBlockSyntax {
            span: open_brace_span,
            open_brace_span,
            statements: Vec::new(),
            close_brace_span: open_brace_span,
        });
        let mut statements = Vec::new();
        body(self, &mut statements);
        let close_brace_span = self.f.text("}");
        self.f.text(" ");
        self.blocks[id as usize] = RawBlockSyntax {
            span: at(start, close_brace_span.end as usize),
            open_brace_span,
            statements,
            close_brace_span,
        };
        id
    }

    fn statement(
        &mut self,
        statements: &mut Vec<u32>,
        build: impl FnOnce(&mut Self) -> RawStatementKind,
    ) {
        let start = self.f.source.len();
        let id = u32::try_from(self.f.statements.len()).expect("statement id");
        statements.push(id);
        self.f.statements.push(RawStatementSyntax {
            span: at(start, start),
            kind: RawStatementKind::Block { block: 0 },
        });
        let kind = build(self);
        self.f.statements[id as usize] =
            RawStatementSyntax { span: at(start, self.f.source.len()), kind };
    }

    fn local(
        &mut self,
        statements: &mut Vec<u32>,
        name: &str,
        ty: &Ty,
        value: impl FnOnce(&mut Self) -> u32,
    ) {
        let start = self.f.source.len();
        let keyword_span = self.f.text("const");
        self.f.text(" ");
        let name = self.f.name(name);
        self.f.text(": ");
        let type_syntax = self.f.ty(ty);
        self.f.text(" ");
        let equals_span = self.f.text("=");
        self.f.text(" ");
        let initializer = value(self);
        let semicolon_span = self.f.text(";");
        let id = u32::try_from(self.f.statements.len()).expect("local id");
        statements.push(id);
        self.f.statements.push(RawStatementSyntax {
            span: at(start, self.f.source.len()),
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
        self.f.text(" ");
    }

    fn unary(&mut self, spelling: &str, value: impl FnOnce(&mut Self) -> u32) -> u32 {
        let start = self.f.source.len();
        let keyword_span = self.f.text(spelling);
        let open_paren_span = self.f.text("(");
        let value = value(self);
        let close_paren_span = self.f.text(")");
        let kind = match spelling {
            "shared" => {
                RawExpressionKind::Shared { keyword_span, open_paren_span, value, close_paren_span }
            }
            "clone" => {
                RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span }
            }
            "downgrade" => RawExpressionKind::Downgrade {
                keyword_span,
                open_paren_span,
                value,
                close_paren_span,
            },
            _ => unreachable!("fixture unary"),
        };
        self.f.expression(start, kind)
    }

    fn call(&mut self, source: &str) -> u32 {
        let start = self.f.source.len();
        let callee = self.f.name("relay");
        let open_paren_span = self.f.text("(");
        let argument = self.unary("clone", |c| c.f.reference(source));
        let close_paren_span = self.f.text(")");
        self.f.expression(
            start,
            RawExpressionKind::Call {
                callee,
                open_paren_span,
                arguments: vec![argument],
                close_paren_span,
            },
        )
    }

    fn vector(&mut self, element: &Ty, source: &str) -> u32 {
        let vector = Ty::Vec(Box::new(element.clone()));
        let start = self.f.source.len();
        let type_syntax = self.f.ty(&vector);
        let open_paren_span = self.f.text("(");
        let open_bracket_span = self.f.text("[");
        let value = self.unary("clone", |c| c.f.reference(source));
        let close_bracket_span = self.f.text("]");
        let close_paren_span = self.f.text(")");
        self.f.expression(
            start,
            RawExpressionKind::VecConstruction {
                type_syntax,
                open_paren_span,
                open_bracket_span,
                elements: vec![value],
                close_bracket_span,
                close_paren_span,
            },
        )
    }

    fn upgrade(&mut self, statements: &mut Vec<u32>) {
        self.statement(statements, |c| {
            let keyword_span = c.f.text("upgradeWeak");
            c.f.text(" ");
            let weak = c.f.reference("weak");
            c.f.text(" ");
            let binding = c.f.name("upgraded");
            c.f.text(" ");
            let as_span = c.f.text("=>");
            c.f.text(" ");
            let success_block = c.block(|_, _| {});
            let else_span = c.f.text("=>");
            c.f.text(" ");
            let failure_block = c.block(|_, _| {});
            RawStatementKind::WeakUpgrade {
                keyword_span,
                weak,
                as_span,
                binding,
                success_block,
                else_span,
                failure_block,
            }
        });
    }

    fn returned(&mut self, statements: &mut Vec<u32>, source: &str) {
        self.statement(statements, |c| {
            let keyword_span = c.f.text("return");
            c.f.text(" ");
            let value = c.f.reference(source);
            let semicolon_span = c.f.text(";");
            RawStatementKind::Return { keyword_span, value, semicolon_span }
        });
    }
}

pub(in crate::data_ownership_v1) fn fixture(
    payload: Payload,
) -> (String, RawProjectSyntaxSnapshot) {
    let mut c = Composer {
        f: Builder {
            source: String::new(),
            types: vec![],
            expressions: vec![],
            statements: vec![],
        },
        blocks: Vec::new(),
    };
    let declarations = declarations::for_payload(&mut c.f, payload);
    let payload_ty = payload.ty();
    let weak = Ty::Weak(Box::new(payload_ty.clone()));
    let relay = relay(&mut c, &weak);
    let compose = compose(&mut c, &payload_ty, &weak);
    (
        c.f.source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: Vec::new(),
                type_syntax: c.f.types,
                data_declarations: declarations,
                functions: vec![relay, compose],
            }],
            diagnostics: Vec::new(),
        },
    )
}

fn relay(c: &mut Composer, weak: &Ty) -> RawFunctionSyntax {
    let start = c.f.source.len();
    let function_span = c.f.text("function");
    c.f.text(" ");
    let name = c.f.name("relay");
    c.f.text("(");
    let parameters = vec![c.f.parameter("value", weak)];
    c.f.text("): ");
    let result_type = c.f.ty(weak);
    c.f.text(" ");
    let root_block = c.block(|c, statements| c.returned(statements, "value"));
    finish(c, start, function_span, name, parameters, result_type, root_block)
}

fn compose(c: &mut Composer, payload: &Ty, weak: &Ty) -> RawFunctionSyntax {
    let shared = Ty::Shared(Box::new(payload.clone()));
    let shared_vec = Ty::Vec(Box::new(shared.clone()));
    let start = c.f.source.len();
    let function_span = c.f.text("function");
    c.f.text(" ");
    let name = c.f.name("compose");
    c.f.text("(");
    let mut parameters = vec![c.f.parameter("payload", payload)];
    c.f.text(", ");
    parameters.push(c.f.parameter("flag", &Ty::Named("bool")));
    c.f.text("): ");
    let result_type = c.f.ty(&shared);
    c.f.text(" ");
    let root_block = c.block(|c, statements| {
        c.local(statements, "owner", &shared, |c| c.unary("shared", |c| c.f.reference("payload")));
        c.local(statements, "weak", weak, |c| c.unary("downgrade", |c| c.f.reference("owner")));
        c.statement(statements, |c| {
            let keyword_span = c.f.text("while");
            c.f.text(" (");
            let open_paren_span = at(keyword_span.end as usize + 1, c.f.source.len());
            let condition = c.f.reference("flag");
            let close_paren_span = c.f.text(")");
            c.f.text(" ");
            let body_block = c.block(|c, statements| {
                c.statement(statements, |c| {
                    let block = c.block(|c, statements| {
                        c.local(statements, "called", weak, |c| c.call("weak"));
                        c.local(statements, "owners", &shared_vec, |c| c.vector(&shared, "owner"));
                    });
                    RawStatementKind::Block { block }
                });
                c.upgrade(statements);
                c.statement(statements, |c| {
                    let keyword_span = c.f.text("if");
                    c.f.text(" (");
                    let open_paren_span = at(keyword_span.end as usize + 1, c.f.source.len());
                    let condition = c.f.reference("flag");
                    let close_paren_span = c.f.text(")");
                    c.f.text(" ");
                    let then_block = c.block(|c, statements| c.returned(statements, "owner"));
                    let else_keyword = c.f.text("else");
                    c.f.text(" ");
                    let block = c.block(|c, statements| {
                        c.local(statements, "weakCopy", weak, |c| {
                            c.unary("clone", |c| c.f.reference("weak"))
                        });
                    });
                    RawStatementKind::If {
                        keyword_span,
                        open_paren_span,
                        condition,
                        close_paren_span,
                        then_block,
                        else_clause: Some(RawElseSyntax { keyword_span: else_keyword, block }),
                    }
                });
            });
            RawStatementKind::While {
                keyword_span,
                open_paren_span,
                condition,
                close_paren_span,
                body_block,
            }
        });
        c.returned(statements, "owner");
    });
    finish(c, start, function_span, name, parameters, result_type, root_block)
}

fn finish(
    c: &mut Composer,
    start: usize,
    function_span: UntrustedSpan,
    name: RawIdentifierSyntax,
    parameters: Vec<RawParameterSyntax>,
    result_type: u32,
    root_block: u32,
) -> RawFunctionSyntax {
    RawFunctionSyntax {
        span: at(start, c.f.source.len()),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: c.blocks[root_block as usize].span,
            root_block,
            blocks: std::mem::take(&mut c.blocks),
            statements: std::mem::take(&mut c.f.statements),
            expressions: std::mem::take(&mut c.f.expressions),
        },
    }
}
