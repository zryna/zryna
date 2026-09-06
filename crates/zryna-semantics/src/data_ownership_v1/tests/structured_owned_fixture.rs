use super::*;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};

#[path = "structured_match_fixture.rs"]
mod match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::Payload;
pub(in crate::data_ownership_v1) use match_fixture::call_fixture as call_match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::continued_indexed_fixture;
pub(in crate::data_ownership_v1) use match_fixture::continued_indexed_rejection_fixture;
pub(in crate::data_ownership_v1) use match_fixture::fixture as match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::formal_fixture as formal_match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::fresh_indexed_assignment_fixture;
pub(in crate::data_ownership_v1) use match_fixture::fresh_indexed_literal_fixture;
pub(in crate::data_ownership_v1) use match_fixture::fresh_indexed_match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::fresh_indexed_match_implicit_read_fixture;
pub(in crate::data_ownership_v1) use match_fixture::fresh_indexed_match_mismatch_fixture;
pub(in crate::data_ownership_v1) use match_fixture::indexed_fixture as indexed_match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::indexed_nested_fixture;
pub(in crate::data_ownership_v1) use match_fixture::lexical_fixture as lexical_match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::nested_fixture as nested_match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::string_fixture as string_match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::string_move_fixture as string_move_match_fixture;
pub(in crate::data_ownership_v1) use match_fixture::vec_fixture as vec_match_fixture;

pub(super) enum Statement {
    Local(&'static str, &'static str, bool),
    Return(&'static str),
    If(Vec<Self>, Vec<Self>),
    While(Vec<Self>),
}

#[derive(Default)]
struct Builder {
    owned_payload: Option<Payload>,
    text: String,
    types: Vec<RawTypeSyntax>,
    blocks: Vec<RawBlockSyntax>,
    statements: Vec<RawStatementSyntax>,
    expressions: Vec<RawExpressionSyntax>,
}

impl Builder {
    fn span(&self, start: usize) -> UntrustedSpan {
        UntrustedSpan {
            file: 0,
            start: u32::try_from(start).expect("source offset"),
            end: u32::try_from(self.text.len()).expect("source offset"),
        }
    }
    fn text(&mut self, text: &str) -> UntrustedSpan {
        let start = self.text.len();
        self.text.push_str(text);
        self.span(start)
    }
    fn name(&mut self, text: &str) -> RawIdentifierSyntax {
        RawIdentifierSyntax { text: text.into(), span: self.text(text) }
    }
    fn ty(&mut self, owned: bool) -> u32 {
        if owned && let Some(payload) = self.owned_payload.take() {
            let id = self.payload_type(payload);
            self.owned_payload = Some(payload);
            return id;
        }
        let start = self.text.len();
        let kind = if owned {
            RawTypeSyntaxKind::String { keyword_span: self.text("String") }
        } else {
            RawTypeSyntaxKind::Named { name: self.name("bool") }
        };
        let id = u32::try_from(self.types.len()).expect("type count");
        self.types.push(RawTypeSyntax { span: self.span(start), kind });
        id
    }
    fn reference(&mut self, text: &str) -> u32 {
        let name = self.name(text);
        let id = u32::try_from(self.expressions.len()).expect("expression count");
        self.expressions.push(RawExpressionSyntax {
            span: name.span,
            kind: RawExpressionKind::Reference { name },
        });
        id
    }
    fn clone_value(&mut self, text: &str) -> u32 {
        let start = self.text.len();
        let keyword_span = self.text("clone");
        let open_paren_span = self.text("(");
        let value = self.reference(text);
        let close_paren_span = self.text(")");
        let id = u32::try_from(self.expressions.len()).expect("expression count");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(start),
            kind: RawExpressionKind::Clone {
                keyword_span,
                open_paren_span,
                value,
                close_paren_span,
            },
        });
        id
    }
    fn block(&mut self, statements: &[Statement]) -> u32 {
        let start = self.text.len();
        let open_brace_span = self.text("{");
        let id = u32::try_from(self.blocks.len()).expect("block count");
        self.blocks.push(RawBlockSyntax {
            span: open_brace_span,
            open_brace_span,
            close_brace_span: open_brace_span,
            statements: Vec::new(),
        });
        let statements = statements
            .iter()
            .map(|statement| {
                self.text(" ");
                self.statement(statement)
            })
            .collect();
        self.text(" ");
        let close_brace_span = self.text("}");
        self.blocks[id as usize] = RawBlockSyntax {
            span: self.span(start),
            open_brace_span,
            close_brace_span,
            statements,
        };
        id
    }
    fn statement(&mut self, statement: &Statement) -> u32 {
        let start = self.text.len();
        let placeholder = self.span(start);
        let id = u32::try_from(self.statements.len()).expect("statement count");
        self.statements.push(RawStatementSyntax {
            span: placeholder,
            kind: RawStatementKind::Block { block: 0 },
        });
        let kind = match statement {
            Statement::Local(binding, source, cloned) => {
                let keyword_span = self.text("const");
                self.text(" ");
                let name = self.name(binding);
                self.text(": ");
                let type_syntax = self.ty(true);
                self.text(" ");
                let equals_span = self.text("=");
                self.text(" ");
                let initializer =
                    if *cloned { self.clone_value(source) } else { self.reference(source) };
                let semicolon_span = self.text(";");
                RawStatementKind::LocalDeclaration {
                    keyword_span,
                    mutable: false,
                    name,
                    type_syntax,
                    equals_span,
                    initializer,
                    semicolon_span,
                }
            }
            Statement::Return(source) => {
                let keyword_span = self.text("return");
                self.text(" ");
                let value = self.reference(source);
                let semicolon_span = self.text(";");
                RawStatementKind::Return { keyword_span, value, semicolon_span }
            }
            Statement::If(yes, no) => {
                let keyword_span = self.text("if");
                self.text(" ");
                let open_paren_span = self.text("(");
                let condition = self.reference("flag");
                let close_paren_span = self.text(")");
                self.text(" ");
                let then_block = self.block(yes);
                self.text(" ");
                let else_keyword = self.text("else");
                self.text(" ");
                let block = self.block(no);
                RawStatementKind::If {
                    keyword_span,
                    open_paren_span,
                    condition,
                    close_paren_span,
                    then_block,
                    else_clause: Some(RawElseSyntax { keyword_span: else_keyword, block }),
                }
            }
            Statement::While(body) => {
                let keyword_span = self.text("while");
                self.text(" ");
                let open_paren_span = self.text("(");
                let condition = self.reference("flag");
                let close_paren_span = self.text(")");
                self.text(" ");
                let body_block = self.block(body);
                RawStatementKind::While {
                    keyword_span,
                    open_paren_span,
                    condition,
                    close_paren_span,
                    body_block,
                }
            }
        };
        self.statements[id as usize] = RawStatementSyntax { span: self.span(start), kind };
        id
    }
}

pub(super) fn fixture(statements: &[Statement]) -> (String, RawProjectSyntaxSnapshot) {
    payload_fixture(statements, Payload::String)
}

pub(super) fn payload_fixture(
    statements: &[Statement],
    payload: Payload,
) -> (String, RawProjectSyntaxSnapshot) {
    let mut builder = Builder::default();
    let declarations = builder.payload_declaration(payload).into_iter().collect::<Vec<_>>();
    if !declarations.is_empty() {
        builder.text("\n");
    }
    builder.owned_payload = Some(payload);
    let function_start = builder.text.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("compose");
    builder.text("(");
    let mut parameters = Vec::new();
    for (index, (name, owned)) in [("flag", false), ("seed", true)].into_iter().enumerate() {
        if index != 0 {
            builder.text(", ");
        }
        let start = builder.text.len();
        let name = builder.name(name);
        builder.text(": ");
        let type_syntax = builder.ty(owned);
        parameters.push(RawParameterSyntax { span: builder.span(start), name, type_syntax });
    }
    builder.text("): ");
    let result_type = builder.ty(true);
    builder.text(" ");
    let root_block = builder.block(statements);
    let function = RawFunctionSyntax {
        span: builder.span(function_start),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: builder.blocks[0].span,
            root_block,
            blocks: builder.blocks,
            statements: builder.statements,
            expressions: builder.expressions,
        },
    };
    (
        builder.text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: Vec::new(),
                type_syntax: builder.types,
                data_declarations: declarations,
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}
