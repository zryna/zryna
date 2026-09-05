use super::*;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawExpressionKind;

#[derive(Clone, Copy)]
pub(super) enum Case {
    Empty,
    Effects,
    GrowthInside,
    GrowthAfter,
}

struct Builder {
    source: String,
    types: Vec<RawTypeSyntax>,
    expressions: Vec<RawExpressionSyntax>,
    statements: Vec<RawStatementSyntax>,
    blocks: Vec<RawBlockSyntax>,
}

fn at(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: start.try_into().expect("start"),
        end: end.try_into().expect("end"),
    }
}

impl Builder {
    fn text(&mut self, value: &str) -> UntrustedSpan {
        let start = self.source.len();
        self.source.push_str(value);
        at(start, self.source.len())
    }
    fn name(&mut self, value: &str) -> RawIdentifierSyntax {
        RawIdentifierSyntax { text: value.into(), span: self.text(value) }
    }
    fn ty(&mut self, value: &str) -> u32 {
        let start = self.source.len();
        let kind = if matches!(value, "Vec" | "BorrowMut") {
            let keyword_span = self.text(value);
            let less_than_span = self.text("<");
            let argument = self.ty("bool");
            let greater_than_span = self.text(">");
            if value == "Vec" {
                RawTypeSyntaxKind::Vec { keyword_span, less_than_span, argument, greater_than_span }
            } else {
                RawTypeSyntaxKind::BorrowMut {
                    keyword_span,
                    less_than_span,
                    argument,
                    greater_than_span,
                }
            }
        } else {
            RawTypeSyntaxKind::Named { name: self.name(value) }
        };
        let id = self.types.len().try_into().expect("type");
        self.types.push(RawTypeSyntax { span: at(start, self.source.len()), kind });
        id
    }
    fn expression(&mut self, start: usize, kind: RawExpressionKind) -> u32 {
        let id = self.expressions.len().try_into().expect("expression");
        self.expressions.push(RawExpressionSyntax { span: at(start, self.source.len()), kind });
        id
    }
    fn reference(&mut self, value: &str) -> u32 {
        let start = self.source.len();
        let name = self.name(value);
        self.expression(start, RawExpressionKind::Reference { name })
    }
    fn call(&mut self, name: &str, argument: &str) -> u32 {
        let start = self.source.len();
        let callee = self.name(name);
        let open_paren_span = self.text("(");
        let argument = self.reference(argument);
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::Call {
                callee,
                open_paren_span,
                arguments: vec![argument],
                close_paren_span,
            },
        )
    }
    fn parameter(&mut self, name: &str, ty: &str) -> RawParameterSyntax {
        let start = self.source.len();
        let name = self.name(name);
        self.text(": ");
        let type_syntax = self.ty(ty);
        RawParameterSyntax { span: at(start, self.source.len()), name, type_syntax }
    }
    fn statement(&mut self, start: usize, kind: RawStatementKind) -> u32 {
        let id = self.statements.len().try_into().expect("statement");
        self.statements.push(RawStatementSyntax { span: at(start, self.source.len()), kind });
        self.text(" ");
        id
    }
    fn local(&mut self, alias: bool, case: Case) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text(if alias { "const" } else { "let" });
        self.text(" ");
        let name = self.name(if alias { "loan" } else { "items" });
        self.text(": ");
        let type_syntax = self.ty(if alias { "BorrowMut" } else { "Vec" });
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let initializer = if alias {
            self.borrow(case)
        } else if matches!(case, Case::Empty) {
            let start = self.source.len();
            let type_syntax = self.ty("Vec");
            let open_paren_span = self.text("(");
            let open_bracket_span = self.text("[");
            let close_bracket_span = self.text("]");
            let close_paren_span = self.text(")");
            self.expression(
                start,
                RawExpressionKind::VecConstruction {
                    type_syntax,
                    open_paren_span,
                    open_bracket_span,
                    elements: Vec::new(),
                    close_bracket_span,
                    close_paren_span,
                },
            )
        } else {
            self.reference("incoming")
        };
        let semicolon_span = self.text(";");
        self.statement(
            start,
            RawStatementKind::LocalDeclaration {
                keyword_span,
                mutable: !alias,
                name,
                type_syntax,
                equals_span,
                initializer,
                semicolon_span,
            },
        )
    }
    fn borrow(&mut self, case: Case) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("borrowMut");
        let open_paren_span = self.text("(");
        let target_start = self.source.len();
        let base = self.reference("items");
        let open_bracket_span = self.text("[");
        let index = if matches!(case, Case::Effects) {
            self.call("indexer", "index")
        } else {
            let start = self.source.len();
            self.text("0");
            self.expression(start, RawExpressionKind::I32Literal { spelling: "0".into() })
        };
        let close_bracket_span = self.text("]");
        let value = self.expression(
            target_start,
            RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
        );
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::BorrowMut { keyword_span, open_paren_span, value, close_paren_span },
        )
    }
    fn assignment(&mut self, case: Case) -> u32 {
        let start = self.source.len();
        let target = self.reference("loan");
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let value = if matches!(case, Case::Effects) {
            self.call("fresh", "next")
        } else {
            self.reference("next")
        };
        let semicolon_span = self.text(";");
        self.statement(
            start,
            RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
        )
    }
    fn push(&mut self) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("push");
        let open_paren_span = self.text("(");
        let vector = self.reference("items");
        let comma_span = self.text(",");
        self.text(" ");
        let value = self.reference("next");
        let close_paren_span = self.text(")");
        let expression = self.expression(
            start,
            RawExpressionKind::VecPush {
                keyword_span,
                open_paren_span,
                vector,
                comma_span,
                value,
                close_paren_span,
            },
        );
        let semicolon_span = self.text(";");
        self.statement(start, RawStatementKind::ExpressionStatement { expression, semicolon_span })
    }
    fn lexical(&mut self, case: Case) -> u32 {
        let start = self.source.len();
        let open_brace_span = self.text("{");
        let block = self.blocks.len().try_into().expect("block");
        let statement = self.statements.len();
        self.statements.push(RawStatementSyntax {
            span: open_brace_span,
            kind: RawStatementKind::Block { block },
        });
        self.text(" ");
        let first = self.local(true, case);
        let second =
            if matches!(case, Case::GrowthInside) { self.push() } else { self.assignment(case) };
        let close_brace_span = self.text("}");
        self.blocks.push(RawBlockSyntax {
            span: at(start, self.source.len()),
            open_brace_span,
            statements: vec![first, second],
            close_brace_span,
        });
        self.statements[statement].span = at(start, self.source.len());
        self.text(" ");
        u32::try_from(statement).expect("statement")
    }
    fn returned(&mut self, name: &str) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("return");
        self.text(" ");
        let value = self.reference(name);
        let semicolon_span = self.text(";");
        self.statement(start, RawStatementKind::Return { keyword_span, value, semicolon_span })
    }
    fn function(&mut self, case: Case, helper: Option<(&str, &str)>) -> RawFunctionSyntax {
        self.text("\n");
        let start = self.source.len();
        let function_span = self.text("function");
        self.text(" ");
        let name = self.name(helper.map_or("observe", |(name, _)| name));
        self.text("(");
        let parameters = if let Some((_, ty)) = helper {
            vec![self.parameter("value", ty)]
        } else {
            let first = self.parameter("incoming", "Vec");
            self.text(", ");
            let second = self.parameter("index", "i32");
            self.text(", ");
            vec![first, second, self.parameter("next", "bool")]
        };
        self.text("): ");
        let result_type = self.ty(helper.map_or("Vec", |(_, ty)| ty));
        self.text(" ");
        let open_brace_span = self.text("{");
        self.text(" ");
        self.blocks.push(RawBlockSyntax {
            span: open_brace_span,
            open_brace_span,
            statements: Vec::new(),
            close_brace_span: open_brace_span,
        });
        let mut statements = if helper.is_some() {
            Vec::new()
        } else {
            vec![self.local(false, case), self.lexical(case)]
        };
        if helper.is_none() && matches!(case, Case::GrowthAfter) {
            statements.push(self.push());
        }
        statements.push(self.returned(if helper.is_some() { "value" } else { "items" }));
        let close_brace_span = self.text("}");
        let body_span = at(open_brace_span.start as usize, self.source.len());
        self.blocks[0] =
            RawBlockSyntax { span: body_span, open_brace_span, statements, close_brace_span };
        RawFunctionSyntax {
            span: at(start, self.source.len()),
            export_span: None,
            function_span,
            name,
            parameters,
            result_type,
            body: RawFunctionBodySyntax {
                span: body_span,
                root_block: 0,
                blocks: std::mem::take(&mut self.blocks),
                statements: std::mem::take(&mut self.statements),
                expressions: std::mem::take(&mut self.expressions),
            },
        }
    }
}

pub(super) fn fixture(case: Case) -> (String, RawProjectSyntaxSnapshot) {
    let mut f = Builder {
        source: String::new(),
        types: Vec::new(),
        expressions: Vec::new(),
        statements: Vec::new(),
        blocks: Vec::new(),
    };
    let mut functions = vec![f.function(case, None)];
    if matches!(case, Case::Effects) {
        functions.push(f.function(case, Some(("indexer", "i32"))));
        functions.push(f.function(case, Some(("fresh", "bool"))));
    }
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".into(),
            imports: Vec::new(),
            type_syntax: f.types,
            data_declarations: Vec::new(),
            functions,
        }],
        diagnostics: Vec::new(),
    };
    (f.source, raw)
}
