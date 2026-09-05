use super::*;
use zryna_syntax::v4::{RawDataDeclaration, RawDataDeclarationKind, RawEnumVariant, RawMatchArm};

#[path = "structured_match_callee.rs"]
mod callee;
#[path = "structured_match_operands.rs"]
mod operands;
#[path = "structured_match_payloads.rs"]
mod payloads;
pub(in crate::data_ownership_v1) use payloads::Payload;

impl Builder {
    fn named_type(&mut self, text: &str) -> u32 {
        let name = self.name(text);
        let id = u32::try_from(self.types.len()).expect("fixture type count");
        self.types.push(RawTypeSyntax { span: name.span, kind: RawTypeSyntaxKind::Named { name } });
        id
    }

    fn enumeration(&mut self, payload: Payload) -> RawDataDeclaration {
        let start = self.text.len();
        let interface_span = self.text("interface");
        self.text(" ");
        let name = self.name("Choice");
        self.text(" ");
        let extends_span = self.text("extends");
        self.text(" ");
        let marker_span = self.text("ZrynaEnum");
        self.text(" ");
        let open_brace_span = self.text("{");
        let mut variants = Vec::new();
        for text in ["left", "right"] {
            self.text(" ");
            let start = self.text.len();
            let name = self.name(text);
            let colon_span = self.text(":");
            self.text(" ");
            let payload_type = Some(self.payload_type(payload));
            let semicolon_span = self.text(";");
            variants.push(RawEnumVariant {
                span: self.span(start),
                name,
                colon_span,
                payload_type,
                none_span: None,
                semicolon_span,
            });
        }
        self.text(" ");
        let close_brace_span = self.text("}");
        RawDataDeclaration {
            span: self.span(start),
            export_span: None,
            kind: RawDataDeclarationKind::Enum {
                interface_span,
                name,
                extends_span,
                marker_span,
                open_brace_span,
                variants,
                close_brace_span,
            },
        }
    }

    fn matched(&mut self, cloned: bool) -> u32 {
        let start = self.text.len();
        let keyword_span = self.text("match");
        let open_paren_span = self.text("(");
        let scrutinee = self.reference("source");
        self.text(", ");
        let open_brace_span = self.text("{");
        let mut arms = Vec::new();
        for variant in ["left", "right"] {
            if !arms.is_empty() {
                self.text(",");
            }
            self.text(" ");
            let start = self.text.len();
            self.text("\"");
            let type_name = self.name("Choice");
            let dot_span = self.text(".");
            let variant = self.name(variant);
            self.text("\": (");
            let binding = Some(self.name(if arms.is_empty() { "first" } else { "second" }));
            self.text(") ");
            let arrow_span = self.text("=>");
            self.text(" ");
            let text = if arms.is_empty() { "first" } else { "second" };
            let value = if cloned { self.clone_value(text) } else { self.reference(text) };
            arms.push(RawMatchArm {
                span: self.span(start),
                type_name,
                dot_span,
                variant,
                binding,
                arrow_span,
                value,
            });
        }
        self.text(" ");
        let close_brace_span = self.text("}");
        let close_paren_span = self.text(")");
        let id = u32::try_from(self.expressions.len()).expect("fixture expression count");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(start),
            kind: RawExpressionKind::Match {
                keyword_span,
                open_paren_span,
                scrutinee,
                close_paren_span,
                open_brace_span,
                arms,
                close_brace_span,
            },
        });
        id
    }
}

fn parameters(builder: &mut Builder, payload: Payload) -> Vec<RawParameterSyntax> {
    let parameter_start = builder.text.len();
    let parameter_name = builder.name("source");
    builder.text(": ");
    let type_syntax = builder.named_type("Choice");
    let parameter = RawParameterSyntax {
        span: builder.span(parameter_start),
        name: parameter_name,
        type_syntax,
    };
    let mut parameters = vec![parameter];
    if matches!(payload, Payload::I32) {
        builder.text(", ");
        let parameter_start = builder.text.len();
        let name = builder.name("retained");
        builder.text(": ");
        let type_syntax = builder.ty(true);
        parameters.push(RawParameterSyntax {
            span: builder.span(parameter_start),
            name,
            type_syntax,
        });
    }
    parameters
}

pub(in crate::data_ownership_v1) fn fixture(
    payload: Payload,
    cloned: bool,
    local: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build(payload, cloned, local, OperandKind::Plain)
}

pub(in crate::data_ownership_v1) fn nested_fixture(
    cloned: bool,
    local: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build(Payload::String, cloned, local, OperandKind::Array)
}

pub(in crate::data_ownership_v1) fn call_fixture(
    cloned: bool,
    local: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build(Payload::String, cloned, local, OperandKind::Call)
}

#[derive(Clone, Copy)]
enum OperandKind {
    Plain,
    Array,
    Call,
}

fn build(
    payload: Payload,
    cloned: bool,
    local: bool,
    operand: OperandKind,
) -> (String, RawProjectSyntaxSnapshot) {
    let nested = !matches!(operand, OperandKind::Plain);
    let call = matches!(operand, OperandKind::Call);
    let mut builder = Builder::default();
    let mut declarations = builder.payload_declaration(payload).into_iter().collect::<Vec<_>>();
    if !declarations.is_empty() {
        builder.text("\n");
    }
    declarations.push(builder.enumeration(payload));
    builder.text("\n");
    let start = builder.text.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("extract");
    builder.text("(");
    let parameters = parameters(&mut builder, payload);
    builder.text("): ");
    let result_payload = if nested { Payload::Array(2) } else { payload };
    let result_type = builder.payload_type(result_payload);
    builder.text(" ");
    let body_start = builder.text.len();
    let open_brace_span = builder.text("{");
    builder.text(" ");
    let statement_start = builder.text.len();
    let kind = if local {
        let keyword_span = builder.text("const");
        builder.text(" ");
        let name = builder.name("output");
        builder.text(": ");
        let type_syntax = builder.payload_type(result_payload);
        builder.text(" ");
        let equals_span = builder.text("=");
        builder.text(" ");
        let initializer = builder.match_operand(cloned, nested, call);
        let semicolon_span = builder.text(";");
        RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span,
        }
    } else {
        let keyword_span = builder.text("return");
        builder.text(" ");
        let value = builder.match_operand(cloned, nested, call);
        let semicolon_span = builder.text(";");
        RawStatementKind::Return { keyword_span, value, semicolon_span }
    };
    builder.statements.push(RawStatementSyntax { span: builder.span(statement_start), kind });
    if local {
        builder.text(" ");
        builder.statement(&Statement::Return("output"));
    }
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let body_span = builder.span(body_start);
    let function = RawFunctionSyntax {
        span: builder.span(start),
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
                close_brace_span,
                statements: if local { vec![0, 1] } else { vec![0] },
            }],
            statements: std::mem::take(&mut builder.statements),
            expressions: std::mem::take(&mut builder.expressions),
        },
    };
    let mut functions = vec![function];
    if call {
        functions.push(builder.match_callee());
    }
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
                functions,
            }],
            diagnostics: Vec::new(),
        },
    )
}
