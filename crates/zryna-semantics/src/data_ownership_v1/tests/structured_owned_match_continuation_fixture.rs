use super::*;
use zryna_syntax::v4::{RawDataDeclaration, RawDataDeclarationKind, RawEnumVariant, RawMatchArm};

impl Builder {
    fn nominal_type(&mut self, text: &str) -> u32 {
        let name = self.name(text);
        let id = u32::try_from(self.types.len()).expect("type count");
        self.types.push(RawTypeSyntax { span: name.span, kind: RawTypeSyntaxKind::Named { name } });
        id
    }

    fn one_arm_match(&mut self) -> u32 {
        let start = self.text.len();
        let keyword_span = self.text("match");
        let open_paren_span = self.text("(");
        let scrutinee = self.reference("source");
        self.text(", ");
        let open_brace_span = self.text("{");
        self.text(" ");
        let arm_start = self.text.len();
        self.text("\"");
        let type_name = self.name("Choice");
        let dot_span = self.text(".");
        let variant = self.name("item");
        self.text("\": (");
        let binding = Some(self.name("payload"));
        self.text(") ");
        let arrow_span = self.text("=>");
        self.text(" ");
        let value = self.reference("payload");
        let arm = RawMatchArm {
            span: self.span(arm_start),
            type_name,
            dot_span,
            variant,
            binding,
            arrow_span,
            value,
        };
        self.text(" ");
        let close_brace_span = self.text("}");
        let close_paren_span = self.text(")");
        let id = u32::try_from(self.expressions.len()).expect("expression count");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(start),
            kind: RawExpressionKind::Match {
                keyword_span,
                open_paren_span,
                scrutinee,
                close_paren_span,
                open_brace_span,
                arms: vec![arm],
                close_brace_span,
            },
        });
        id
    }
}

#[allow(clippy::too_many_lines)]
pub(in crate::data_ownership_v1::tests) fn fixture(
    payload: Payload,
) -> (String, RawProjectSyntaxSnapshot) {
    let mut builder = Builder::default();
    let mut declarations = builder.payload_declaration(payload).into_iter().collect::<Vec<_>>();
    if !declarations.is_empty() {
        builder.text("\n");
    }
    builder.owned_payload = Some(payload);
    let declaration_start = builder.text.len();
    let interface_span = builder.text("interface");
    builder.text(" ");
    let enum_name = builder.name("Choice");
    builder.text(" ");
    let extends_span = builder.text("extends");
    builder.text(" ");
    let marker_span = builder.text("ZrynaEnum");
    builder.text(" ");
    let open_brace_span = builder.text("{");
    builder.text(" ");
    let variant_start = builder.text.len();
    let variant_name = builder.name("item");
    let colon_span = builder.text(":");
    builder.text(" ");
    let payload_type = builder.payload_type(payload);
    let semicolon_span = builder.text(";");
    let variant = RawEnumVariant {
        span: builder.span(variant_start),
        name: variant_name,
        colon_span,
        payload_type: Some(payload_type),
        none_span: None,
        semicolon_span,
    };
    builder.text(" ");
    let close_brace_span = builder.text("}");
    declarations.push(RawDataDeclaration {
        span: builder.span(declaration_start),
        export_span: None,
        kind: RawDataDeclarationKind::Enum {
            interface_span,
            name: enum_name,
            extends_span,
            marker_span,
            open_brace_span,
            variants: vec![variant],
            close_brace_span,
        },
    });
    builder.text("\n");
    let function_start = builder.text.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("extract");
    builder.text("(");
    let parameter_start = builder.text.len();
    let parameter_name = builder.name("source");
    builder.text(": ");
    let parameter_type = builder.nominal_type("Choice");
    let parameters = vec![RawParameterSyntax {
        span: builder.span(parameter_start),
        name: parameter_name,
        type_syntax: parameter_type,
    }];
    builder.text("): ");
    let result_type = builder.payload_type(payload);
    builder.text(" ");
    let body_start = builder.text.len();
    let open_brace_span = builder.text("{");
    builder.blocks.push(RawBlockSyntax {
        span: open_brace_span,
        open_brace_span,
        close_brace_span: open_brace_span,
        statements: Vec::new(),
    });
    builder.text(" ");
    let local_start = builder.text.len();
    let keyword_span = builder.text("const");
    builder.text(" ");
    let local_name = builder.name("output");
    builder.text(": ");
    let local_type = builder.payload_type(payload);
    builder.text(" ");
    let equals_span = builder.text("=");
    builder.text(" ");
    let initializer = builder.one_arm_match();
    let semicolon_span = builder.text(";");
    let local_id = u32::try_from(builder.statements.len()).expect("statement count");
    builder.statements.push(RawStatementSyntax {
        span: builder.span(local_start),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name: local_name,
            type_syntax: local_type,
            equals_span,
            initializer,
            semicolon_span,
        },
    });
    builder.text(" ");
    let block_id = builder.statement(&Statement::Block(Vec::new()));
    builder.text(" ");
    let return_id = builder.statement(&Statement::Return("output"));
    builder.text(" ");
    let root_close = builder.text("}");
    builder.blocks[0] = RawBlockSyntax {
        span: builder.span(body_start),
        open_brace_span,
        close_brace_span: root_close,
        statements: vec![local_id, block_id, return_id],
    };
    let function = RawFunctionSyntax {
        span: builder.span(function_start),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: builder.span(body_start),
            root_block: 0,
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
