use super::*;
use zryna_syntax::v4::RawElseSyntax;

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Context {
    Nested,
    Repeated,
    Conditional,
    Loop,
    Call,
    Match,
    WrongArm,
    Nonuniform,
}

impl Context {
    fn is_match(self) -> bool {
        matches!(self, Self::Match | Self::WrongArm | Self::Nonuniform)
    }
}

struct Composer {
    f: Builder,
    blocks: Vec<RawBlockSyntax>,
}

impl Composer {
    fn block(&mut self, body: impl FnOnce(&mut Self, &mut Vec<u32>)) -> u32 {
        let open_brace_span = self.f.text("{");
        self.f.text(" ");
        let id = u32::try_from(self.blocks.len()).expect("block id");
        self.blocks.push(RawBlockSyntax {
            span: open_brace_span,
            open_brace_span,
            statements: vec![],
            close_brace_span: open_brace_span,
        });
        let mut statements = Vec::new();
        body(self, &mut statements);
        let close_brace_span = self.f.text("}");
        self.f.text(" ");
        self.blocks[id as usize] = RawBlockSyntax {
            span: at(open_brace_span.start as usize, close_brace_span.end as usize),
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
        value: impl FnOnce(&mut Builder) -> u32,
    ) {
        statements.push(u32::try_from(self.f.statements.len()).expect("local id"));
        local(&mut self.f, name, ty, value);
    }

    fn upgrade(&mut self, statements: &mut Vec<u32>, name: &str, context: Context) {
        self.statement(statements, |c| {
            let keyword_span = c.f.text("upgradeWeak");
            c.f.text(" ");
            let weak = match context {
                Context::Call => c.call_operand(),
                Context::Match | Context::WrongArm | Context::Nonuniform => {
                    c.match_operand(context)
                }
                _ => c.f.reference("weak"),
            };
            c.f.text(" ");
            let binding = c.f.name(name);
            c.f.text(" ");
            let as_span = c.f.text("=>");
            c.f.text(" ");
            let success_block = c.block(|c, statements| {
                if matches!(context, Context::Nested) {
                    c.upgrade(statements, "inner", Context::Repeated);
                }
            });
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

    fn control(&mut self, statements: &mut Vec<u32>, context: Context) {
        self.statement(statements, |c| {
            let keyword_span =
                c.f.text(if matches!(context, Context::Loop) { "while" } else { "if" });
            c.f.text(" ");
            let open_paren_span = c.f.text("(");
            let condition = c.f.reference("flag");
            let close_paren_span = c.f.text(")");
            c.f.text(" ");
            let then_block = c.block(|c, statements| {
                c.upgrade(statements, "first", Context::Nested);
                c.upgrade(statements, "second", Context::Repeated);
            });
            if matches!(context, Context::Loop) {
                RawStatementKind::While {
                    keyword_span,
                    open_paren_span,
                    condition,
                    close_paren_span,
                    body_block: then_block,
                }
            } else {
                let else_span = c.f.text("else");
                c.f.text(" ");
                let block = c
                    .block(|c, statements| c.upgrade(statements, "alternative", Context::Repeated));
                RawStatementKind::If {
                    keyword_span,
                    open_paren_span,
                    condition,
                    close_paren_span,
                    then_block,
                    else_clause: Some(RawElseSyntax { keyword_span: else_span, block }),
                }
            }
        });
    }

    fn call_operand(&mut self) -> u32 {
        let start = self.f.source.len();
        let callee = self.f.name("produce");
        let open_paren_span = self.f.text("(");
        let argument = unary(&mut self.f, "clone", |f| f.reference("weak"));
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

    fn match_operand(&mut self, context: Context) -> u32 {
        let start = self.f.source.len();
        let keyword_span = self.f.text("match");
        let open_paren_span = self.f.text("(");
        let scrutinee = self.f.reference("choice");
        self.f.text(", ");
        let open_brace_span = self.f.text("{");
        self.f.text(" ");
        let mut arms = Vec::new();
        for (ordinal, member) in ["first", "second"].into_iter().enumerate() {
            if ordinal != 0 {
                self.f.text(", ");
            }
            let arm_start = self.f.source.len();
            self.f.text("\"");
            let type_name = self.f.name("Choice");
            let dot_span = self.f.text(".");
            let variant = self.f.name(if ordinal == 1 && matches!(context, Context::WrongArm) {
                "absent"
            } else {
                member
            });
            self.f.text("\": (");
            let bound = if ordinal == 0 { "left" } else { "right" };
            let binding = Some(self.f.name(bound));
            self.f.text(") ");
            let arrow_span = self.f.text("=>");
            self.f.text(" ");
            let value = unary(&mut self.f, "clone", |f| {
                f.reference(if ordinal == 1 && matches!(context, Context::Nonuniform) {
                    "owner"
                } else {
                    bound
                })
            });
            let arm = zryna_syntax::v4::RawMatchArm {
                span: at(arm_start, self.f.source.len()),
                type_name,
                dot_span,
                variant,
                binding,
                arrow_span,
                value,
            };
            arms.push(arm);
        }
        self.f.text(" ");
        let close_brace_span = self.f.text("}");
        let close_paren_span = self.f.text(")");
        self.f.expression(
            start,
            RawExpressionKind::Match {
                keyword_span,
                open_paren_span,
                scrutinee,
                close_paren_span,
                open_brace_span,
                arms,
                close_brace_span,
            },
        )
    }
}

fn choice(f: &mut Builder) -> RawDataDeclaration {
    let start = f.source.len();
    let interface_span = f.text("interface");
    f.text(" ");
    let name = f.name("Choice");
    f.text(" ");
    let extends_span = f.text("extends");
    f.text(" ");
    let marker_span = f.text("ZrynaEnum");
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let mut variants = Vec::new();
    for member in ["first", "second"] {
        let member_start = f.source.len();
        let member = f.name(member);
        let colon_span = f.text(":");
        f.text(" ");
        let payload_type = Some(f.ty(&weak_type()));
        let semicolon_span = f.text(";");
        f.text(" ");
        variants.push(zryna_syntax::v4::RawEnumVariant {
            span: at(member_start, semicolon_span.end as usize),
            name: member,
            colon_span,
            payload_type,
            none_span: None,
            semicolon_span,
        });
    }
    let close_brace_span = f.text("}");
    f.text("\n");
    RawDataDeclaration {
        span: at(start, close_brace_span.end as usize),
        export_span: None,
        kind: RawDataDeclarationKind::Enum {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            close_brace_span,
            variants,
        },
    }
}

pub(in crate::data_ownership_v1) fn fixture(
    context: Context,
) -> (String, RawProjectSyntaxSnapshot) {
    let mut c = Composer {
        f: Builder {
            source: String::new(),
            types: vec![],
            expressions: vec![],
            statements: vec![],
        },
        blocks: vec![],
    };
    let declarations = if context.is_match() { vec![choice(&mut c.f)] } else { vec![] };
    let function = main_function(&mut c, context);
    let mut functions = vec![function];
    if matches!(context, Context::Call) {
        functions.push(producer(&mut c));
    }
    (
        c.f.source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: vec![],
                type_syntax: c.f.types,
                data_declarations: declarations,
                functions,
            }],
            diagnostics: vec![],
        },
    )
}

fn main_function(c: &mut Composer, context: Context) -> RawFunctionSyntax {
    let start = c.f.source.len();
    let function_span = c.f.text("function");
    c.f.text(" ");
    let name = c.f.name("upgrade");
    c.f.text("(");
    let mut parameters = vec![c.f.parameter("payload", &Ty::String)];
    if context.is_match() || matches!(context, Context::Conditional | Context::Loop) {
        c.f.text(", ");
        parameters.push(c.f.parameter(
            if context.is_match() { "choice" } else { "flag" },
            &Ty::Named(if context.is_match() { "Choice" } else { "bool" }),
        ));
    }
    c.f.text("): ");
    let shared = Ty::Shared(Box::new(Ty::String));
    let result_type = c.f.ty(&shared);
    c.f.text(" ");
    let root_block = c.block(|c, statements| {
        c.local(statements, "owner", &shared, |f| unary(f, "shared", |f| f.reference("payload")));
        c.local(statements, "weak", &weak_type(), |f| {
            unary(f, "downgrade", |f| f.reference("owner"))
        });
        if matches!(context, Context::Conditional | Context::Loop) {
            c.control(statements, context);
        } else {
            c.upgrade(statements, "first", context);
            if matches!(context, Context::Repeated) {
                c.upgrade(statements, "second", context);
            }
        }
        statements.push(u32::try_from(c.f.statements.len()).expect("return id"));
        returned(&mut c.f, "owner");
    });
    finish_function(c, start, function_span, name, parameters, result_type, root_block)
}

fn producer(c: &mut Composer) -> RawFunctionSyntax {
    let start = c.f.source.len();
    let function_span = c.f.text("function");
    c.f.text(" ");
    let name = c.f.name("produce");
    c.f.text("(");
    let parameters = vec![c.f.parameter("value", &weak_type())];
    c.f.text("): ");
    let result_type = c.f.ty(&weak_type());
    c.f.text(" ");
    let root_block = c.block(|c, statements| {
        statements.push(u32::try_from(c.f.statements.len()).expect("return id"));
        returned(&mut c.f, "value");
    });
    finish_function(c, start, function_span, name, parameters, result_type, root_block)
}

fn finish_function(
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
