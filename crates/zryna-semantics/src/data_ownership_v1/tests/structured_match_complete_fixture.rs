use super::*;

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum InvalidMatch {
    Empty,
    Missing,
    Duplicate,
    Unknown,
    PayloadlessBinding,
    MissingPayloadBinding,
}

#[allow(clippy::too_many_lines)]
pub(in crate::data_ownership_v1) fn mixed_variant_fixture() -> (String, RawProjectSyntaxSnapshot) {
    mixed_variant_fixture_with_arms(
        &[
            ("empty", None, "fallback"),
            ("text", Some("text"), "fallback"),
            ("number", Some("number"), "number"),
        ],
        false,
    )
}

pub(in crate::data_ownership_v1) fn reordered_mixed_variant_fixture()
-> (String, RawProjectSyntaxSnapshot) {
    mixed_variant_fixture_with_arms(
        &[
            ("number", Some("number"), "number"),
            ("empty", None, "fallback"),
            ("text", Some("text"), "fallback"),
        ],
        true,
    )
}

pub(in crate::data_ownership_v1) fn invalid_mixed_variant_fixture(
    invalid: InvalidMatch,
) -> (String, RawProjectSyntaxSnapshot) {
    let arms = match invalid {
        InvalidMatch::Empty => vec![],
        InvalidMatch::Missing => {
            vec![("empty", None, "fallback"), ("text", Some("text"), "fallback")]
        }
        InvalidMatch::Duplicate => vec![
            ("empty", None, "fallback"),
            ("text", Some("text"), "fallback"),
            ("text", Some("again"), "fallback"),
        ],
        InvalidMatch::Unknown => vec![
            ("empty", None, "fallback"),
            ("text", Some("text"), "fallback"),
            ("absent", Some("unknown"), "fallback"),
        ],
        InvalidMatch::PayloadlessBinding => vec![
            ("empty", Some("invalid"), "fallback"),
            ("text", Some("text"), "fallback"),
            ("number", Some("number"), "number"),
        ],
        InvalidMatch::MissingPayloadBinding => vec![
            ("empty", None, "fallback"),
            ("text", None, "fallback"),
            ("number", Some("number"), "number"),
        ],
    };
    mixed_variant_fixture_with_arms(&arms, false)
}

#[allow(clippy::too_many_lines)]
fn mixed_variant_fixture_with_arms(
    arm_specs: &[(&str, Option<&str>, &str)],
    constructed_scrutinee: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let mut builder = Builder::default();
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
    let mut variants = Vec::new();
    for (name, payload) in
        [("empty", None), ("text", Some(Payload::String)), ("number", Some(Payload::I32))]
    {
        builder.text(" ");
        let start = builder.text.len();
        let name = builder.name(name);
        let colon_span = builder.text(":");
        builder.text(" ");
        let (payload_type, none_span) = match payload {
            Some(payload) => (Some(builder.payload_type(payload)), None),
            None => (None, Some(builder.text("ZrynaNone"))),
        };
        let semicolon_span = builder.text(";");
        variants.push(RawEnumVariant {
            span: builder.span(start),
            name,
            colon_span,
            payload_type,
            none_span,
            semicolon_span,
        });
    }
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let declaration = RawDataDeclaration {
        span: builder.span(declaration_start),
        export_span: None,
        kind: RawDataDeclarationKind::Enum {
            interface_span,
            name: enum_name,
            extends_span,
            marker_span,
            open_brace_span,
            variants,
            close_brace_span,
        },
    };

    builder.text("\n");
    let function_start = builder.text.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("select");
    builder.text("(");
    let source_start = builder.text.len();
    let source_name = builder.name("source");
    builder.text(": ");
    let source_type = builder.named_type("Choice");
    let source_parameter = RawParameterSyntax {
        span: builder.span(source_start),
        name: source_name,
        type_syntax: source_type,
    };
    builder.text(", ");
    let fallback_start = builder.text.len();
    let fallback_name = builder.name("fallback");
    builder.text(": ");
    let fallback_type = builder.named_type("i32");
    let fallback_parameter = RawParameterSyntax {
        span: builder.span(fallback_start),
        name: fallback_name,
        type_syntax: fallback_type,
    };
    builder.text("): ");
    let result_type = builder.named_type("i32");
    builder.text(" ");
    let body_start = builder.text.len();
    let open_brace_span = builder.text("{");
    builder.text(" ");
    let return_start = builder.text.len();
    let keyword_span = builder.text("return");
    builder.text(" ");
    let match_start = builder.text.len();
    let match_keyword_span = builder.text("match");
    let open_paren_span = builder.text("(");
    let scrutinee = if constructed_scrutinee {
        constructed_choice(&mut builder)
    } else {
        builder.reference("source")
    };
    builder.text(", ");
    let match_open_brace_span = builder.text("{");
    let mut arms = Vec::new();
    for &(variant_name, binding_name, value_name) in arm_specs {
        if !arms.is_empty() {
            builder.text(",");
        }
        builder.text(" ");
        let arm_start = builder.text.len();
        builder.text("\"");
        let type_name = builder.name("Choice");
        let dot_span = builder.text(".");
        let variant = builder.name(variant_name);
        builder.text("\": (");
        let binding = binding_name.map(|binding| builder.name(binding));
        builder.text(") ");
        let arrow_span = builder.text("=>");
        builder.text(" ");
        let value = builder.reference(value_name);
        arms.push(RawMatchArm {
            span: builder.span(arm_start),
            type_name,
            dot_span,
            variant,
            binding,
            arrow_span,
            value,
        });
    }
    builder.text(" ");
    let match_close_brace_span = builder.text("}");
    let close_paren_span = builder.text(")");
    let value = u32::try_from(builder.expressions.len()).expect("match expression count");
    builder.expressions.push(RawExpressionSyntax {
        span: builder.span(match_start),
        kind: RawExpressionKind::Match {
            keyword_span: match_keyword_span,
            open_paren_span,
            scrutinee,
            close_paren_span,
            open_brace_span: match_open_brace_span,
            arms,
            close_brace_span: match_close_brace_span,
        },
    });
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: builder.span(return_start),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let body_span = builder.span(body_start);
    let function = RawFunctionSyntax {
        span: builder.span(function_start),
        export_span: None,
        function_span,
        name,
        parameters: vec![source_parameter, fallback_parameter],
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                close_brace_span,
                statements: vec![0],
            }],
            statements: builder.statements,
            expressions: builder.expressions,
        },
    };
    builder.statements = Vec::new();
    builder.expressions = Vec::new();
    finish(builder, vec![declaration], vec![function])
}

fn constructed_choice(builder: &mut Builder) -> u32 {
    let start = builder.text.len();
    let type_name = builder.name("Choice");
    let dot_span = builder.text(".");
    let variant = builder.name("text");
    let open_paren_span = builder.text("(");
    let literal_start = builder.text.len();
    let spelling = "\"once\"";
    builder.text(spelling);
    let payload = u32::try_from(builder.expressions.len()).expect("constructed payload");
    builder.expressions.push(RawExpressionSyntax {
        span: builder.span(literal_start),
        kind: RawExpressionKind::StringLiteral { spelling: spelling.into() },
    });
    let close_paren_span = builder.text(")");
    let id = u32::try_from(builder.expressions.len()).expect("constructed choice");
    builder.expressions.push(RawExpressionSyntax {
        span: builder.span(start),
        kind: RawExpressionKind::EnumConstruction {
            type_name,
            dot_span,
            variant,
            open_paren_span,
            payload: Some(payload),
            close_paren_span,
        },
    });
    id
}

pub(in crate::data_ownership_v1) fn nested_variant_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let mut builder = Builder::default();
    let inner = uniform_enum(&mut builder, "Inner", ["left", "right"], |builder| builder.ty(true));
    builder.text("\n");
    let outer = uniform_enum(&mut builder, "Outer", ["primary", "secondary"], |builder| {
        builder.named_type("Inner")
    });
    builder.text("\n");

    let function_start = builder.text.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("flatten");
    builder.text("(");
    let parameter_start = builder.text.len();
    let parameter_name = builder.name("source");
    builder.text(": ");
    let parameter_type = builder.named_type("Outer");
    let parameter = RawParameterSyntax {
        span: builder.span(parameter_start),
        name: parameter_name,
        type_syntax: parameter_type,
    };
    builder.text("): ");
    let result_type = builder.ty(true);
    builder.text(" ");
    let body_start = builder.text.len();
    let open_brace_span = builder.text("{");
    builder.text(" ");
    let return_start = builder.text.len();
    let keyword_span = builder.text("return");
    builder.text(" ");
    let value = nested_outer_match(&mut builder);
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: builder.span(return_start),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let body_span = builder.span(body_start);
    let function = RawFunctionSyntax {
        span: builder.span(function_start),
        export_span: None,
        function_span,
        name,
        parameters: vec![parameter],
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                close_brace_span,
                statements: vec![0],
            }],
            statements: builder.statements,
            expressions: builder.expressions,
        },
    };
    builder.statements = Vec::new();
    builder.expressions = Vec::new();
    finish(builder, vec![inner, outer], vec![function])
}

fn uniform_enum(
    builder: &mut Builder,
    text: &str,
    names: [&str; 2],
    mut payload: impl FnMut(&mut Builder) -> u32,
) -> RawDataDeclaration {
    let start = builder.text.len();
    let interface_span = builder.text("interface");
    builder.text(" ");
    let name = builder.name(text);
    builder.text(" ");
    let extends_span = builder.text("extends");
    builder.text(" ");
    let marker_span = builder.text("ZrynaEnum");
    builder.text(" ");
    let open_brace_span = builder.text("{");
    let mut variants = Vec::new();
    for text in names {
        builder.text(" ");
        let variant_start = builder.text.len();
        let name = builder.name(text);
        let colon_span = builder.text(":");
        builder.text(" ");
        let payload_type = Some(payload(builder));
        let semicolon_span = builder.text(";");
        variants.push(RawEnumVariant {
            span: builder.span(variant_start),
            name,
            colon_span,
            payload_type,
            none_span: None,
            semicolon_span,
        });
    }
    builder.text(" ");
    let close_brace_span = builder.text("}");
    RawDataDeclaration {
        span: builder.span(start),
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

fn nested_outer_match(builder: &mut Builder) -> u32 {
    let start = builder.text.len();
    let keyword_span = builder.text("match");
    let open_paren_span = builder.text("(");
    let scrutinee = builder.reference("source");
    builder.text(", ");
    let open_brace_span = builder.text("{");
    let mut arms = Vec::new();
    for (variant_name, binding) in [("primary", "firstInner"), ("secondary", "secondInner")] {
        if !arms.is_empty() {
            builder.text(",");
        }
        builder.text(" ");
        let arm_start = builder.text.len();
        builder.text("\"");
        let type_name = builder.name("Outer");
        let dot_span = builder.text(".");
        let variant = builder.name(variant_name);
        builder.text("\": (");
        let binding_name = builder.name(binding);
        builder.text(") ");
        let arrow_span = builder.text("=>");
        builder.text(" ");
        let value = nested_inner_match(builder, binding);
        arms.push(RawMatchArm {
            span: builder.span(arm_start),
            type_name,
            dot_span,
            variant,
            binding: Some(binding_name),
            arrow_span,
            value,
        });
    }
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let close_paren_span = builder.text(")");
    let id = u32::try_from(builder.expressions.len()).expect("outer match expression count");
    builder.expressions.push(RawExpressionSyntax {
        span: builder.span(start),
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

fn nested_inner_match(builder: &mut Builder, prefix: &str) -> u32 {
    let start = builder.text.len();
    let keyword_span = builder.text("match");
    let open_paren_span = builder.text("(");
    let scrutinee = builder.reference(prefix);
    builder.text(", ");
    let open_brace_span = builder.text("{");
    let mut arms = Vec::new();
    for variant_name in ["left", "right"] {
        if !arms.is_empty() {
            builder.text(",");
        }
        builder.text(" ");
        let arm_start = builder.text.len();
        builder.text("\"");
        let type_name = builder.name("Inner");
        let dot_span = builder.text(".");
        let variant = builder.name(variant_name);
        builder.text("\": (");
        let binding_text = format!("{prefix}{variant_name}");
        let binding = builder.name(&binding_text);
        builder.text(") ");
        let arrow_span = builder.text("=>");
        builder.text(" ");
        let value = builder.reference(&binding_text);
        arms.push(RawMatchArm {
            span: builder.span(arm_start),
            type_name,
            dot_span,
            variant,
            binding: Some(binding),
            arrow_span,
            value,
        });
    }
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let close_paren_span = builder.text(")");
    let id = u32::try_from(builder.expressions.len()).expect("inner match expression count");
    builder.expressions.push(RawExpressionSyntax {
        span: builder.span(start),
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
