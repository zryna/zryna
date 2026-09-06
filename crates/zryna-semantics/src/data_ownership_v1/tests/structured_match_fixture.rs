use super::*;
use zryna_syntax::v4::{RawDataDeclaration, RawDataDeclarationKind, RawEnumVariant, RawMatchArm};

#[path = "structured_match_callee.rs"]
mod callee;
#[path = "structured_match_complete_fixture.rs"]
mod complete;
#[path = "continued_indexed_fixture.rs"]
mod continued;
#[path = "fresh_indexed_match_fixture.rs"]
mod fresh;
pub(in crate::data_ownership_v1) use continued::fixture as continued_indexed_fixture;
pub(in crate::data_ownership_v1) use continued::rejection_fixture as continued_indexed_rejection_fixture;
pub(in crate::data_ownership_v1) use fresh::assignment_fixture as fresh_indexed_assignment_fixture;
pub(in crate::data_ownership_v1) use fresh::fresh_match_base_fixture as fresh_indexed_match_fixture;
pub(in crate::data_ownership_v1) use fresh::fresh_match_base_implicit_read_fixture as fresh_indexed_match_implicit_read_fixture;
pub(in crate::data_ownership_v1) use fresh::fresh_match_base_mismatch_fixture as fresh_indexed_match_mismatch_fixture;
pub(in crate::data_ownership_v1) use fresh::literal_fixture as fresh_indexed_literal_fixture;
#[path = "structured_match_formal.rs"]
mod formal;
#[path = "structured_match_indexed.rs"]
mod indexed;
#[path = "structured_match_operands.rs"]
mod operands;
#[path = "structured_match_payloads.rs"]
mod payloads;
#[path = "structured_match_string.rs"]
mod string;
pub(in crate::data_ownership_v1) use complete::{
    InvalidMatch, invalid_mixed_variant_fixture, mixed_variant_fixture, nested_variant_fixture,
    reordered_mixed_variant_fixture,
};
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
    Vec,
    FormalShared,
    FormalExclusive,
    Lexical,
    StringClone,
    StringConcat,
    StringNamed,
    IndexedArray,
    IndexedVec,
    IndexedOwnedArray,
    IndexedOwnedVec,
    IndexedNestedArray,
    IndexedNestedVec,
}

impl OperandKind {
    fn result(self, payload: Payload) -> Payload {
        match self {
            Self::Plain => payload,
            Self::Vec => Payload::Vec,
            Self::StringClone
            | Self::StringConcat
            | Self::StringNamed
            | Self::IndexedOwnedArray
            | Self::IndexedOwnedVec => Payload::String,
            Self::IndexedArray
            | Self::IndexedVec
            | Self::IndexedNestedArray
            | Self::IndexedNestedVec => Payload::I32,
            Self::Array
            | Self::Call
            | Self::FormalShared
            | Self::FormalExclusive
            | Self::Lexical => Payload::Array(2),
        }
    }
    fn formal(self) -> Option<bool> {
        match self {
            Self::FormalShared | Self::Lexical => Some(false),
            Self::FormalExclusive => Some(true),
            _ => None,
        }
    }
    fn call(self) -> bool {
        matches!(self, Self::Call | Self::FormalShared | Self::FormalExclusive | Self::Lexical)
    }
}

pub(in crate::data_ownership_v1) fn indexed_fixture(
    vector: bool,
    owned: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build(
        Payload::I32,
        false,
        true,
        match (vector, owned) {
            (false, false) => OperandKind::IndexedArray,
            (true, false) => OperandKind::IndexedVec,
            (false, true) => OperandKind::IndexedOwnedArray,
            (true, true) => OperandKind::IndexedOwnedVec,
        },
    )
}

pub(in crate::data_ownership_v1) fn indexed_nested_fixture(
    vector: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build(
        Payload::I32,
        false,
        true,
        if vector { OperandKind::IndexedNestedVec } else { OperandKind::IndexedNestedArray },
    )
}

pub(in crate::data_ownership_v1) fn formal_fixture(
    exclusive: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build(
        Payload::String,
        true,
        true,
        if exclusive { OperandKind::FormalExclusive } else { OperandKind::FormalShared },
    )
}

pub(in crate::data_ownership_v1) fn vec_fixture(
    cloned: bool,
    local: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build(Payload::String, cloned, local, OperandKind::Vec)
}

pub(in crate::data_ownership_v1) fn lexical_fixture() -> (String, RawProjectSyntaxSnapshot) {
    build(Payload::String, true, true, OperandKind::Lexical)
}

pub(in crate::data_ownership_v1) fn string_fixture(mode: u8) -> (String, RawProjectSyntaxSnapshot) {
    build(
        Payload::String,
        true,
        true,
        match mode {
            0 => OperandKind::StringClone,
            1 => OperandKind::StringConcat,
            _ => OperandKind::StringNamed,
        },
    )
}

pub(in crate::data_ownership_v1) fn string_move_fixture() -> (String, RawProjectSyntaxSnapshot) {
    build(Payload::String, false, true, OperandKind::StringNamed)
}

fn build(
    payload: Payload,
    cloned: bool,
    local: bool,
    operand: OperandKind,
) -> (String, RawProjectSyntaxSnapshot) {
    let call = operand.call();
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
    let parameters = formal::parameters(&mut builder, payload, operand);
    builder.text("): ");
    let result_payload = operand.result(payload);
    let result_type = builder.payload_type(result_payload);
    builder.text(" ");
    let body_start = builder.text.len();
    let open_brace_span = builder.text("{");
    builder.text(" ");
    if matches!(operand, OperandKind::Lexical) {
        builder.lexical_declaration();
        builder.text(" ");
    }
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
        let initializer = builder.match_operand(cloned, operand);
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
        let value = builder.match_operand(cloned, operand);
        let semicolon_span = builder.text(";");
        RawStatementKind::Return { keyword_span, value, semicolon_span }
    };
    builder.statements.push(RawStatementSyntax { span: builder.span(statement_start), kind });
    if local {
        builder.text(" ");
        if matches!(operand, OperandKind::StringNamed) {
            builder.statement(&Statement::Local("after", "saved", false));
            builder.text(" ");
        }
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
                statements: (0..u32::try_from(builder.statements.len())
                    .expect("body statement count"))
                    .collect(),
            }],
            statements: std::mem::take(&mut builder.statements),
            expressions: std::mem::take(&mut builder.expressions),
        },
    };
    let mut functions = vec![function];
    if call {
        functions.push(builder.match_callee(operand.formal()));
    }
    finish(builder, declarations, functions)
}

fn finish(
    builder: Builder,
    declarations: Vec<RawDataDeclaration>,
    functions: Vec<RawFunctionSyntax>,
) -> (String, RawProjectSyntaxSnapshot) {
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
