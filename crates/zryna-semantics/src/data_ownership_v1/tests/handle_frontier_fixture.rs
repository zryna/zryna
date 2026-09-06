use super::*;
use zryna_syntax::v4::{RawEnumVariant, RawFieldInitializer, RawFieldInitializerKind};

fn declarations(f: &mut Builder) -> Vec<RawDataDeclaration> {
    let mut result = Vec::new();
    for is_enum in [false, true] {
        let start = f.source.len();
        let interface_span = f.text("interface");
        f.text(" ");
        let name = f.name(if is_enum { "Entry" } else { "Payload" });
        f.text(" ");
        let extends_span = f.text("extends");
        f.text(" ");
        let marker_span = f.text(if is_enum { "ZrynaEnum" } else { "ZrynaStruct" });
        f.text(" ");
        let open_brace_span = f.text("{");
        let mut fields = Vec::new();
        let mut variants = Vec::new();
        let items = if is_enum {
            vec![("Empty", None), ("Data", Some(Ty::Named("Payload")))]
        } else {
            vec![
                ("strong", Some(Ty::Shared(Box::new(Ty::String)))),
                ("weak", Some(Ty::Weak(Box::new(Ty::String)))),
                ("text", Some(Ty::String)),
            ]
        };
        for (label, ty) in items {
            f.text(" ");
            let start = f.source.len();
            let name = f.name(label);
            let colon_span = f.text(":");
            f.text(" ");
            let (type_syntax, none_span) = if let Some(ty) = ty {
                (Some(f.ty(&ty)), None)
            } else {
                (None, Some(f.text("ZrynaNone")))
            };
            let semicolon_span = f.text(";");
            if is_enum {
                variants.push(RawEnumVariant {
                    span: at(start, f.source.len()),
                    name,
                    colon_span,
                    payload_type: type_syntax,
                    none_span,
                    semicolon_span,
                });
            } else {
                fields.push(RawDataField {
                    span: at(start, f.source.len()),
                    name,
                    colon_span,
                    type_syntax: type_syntax.expect("field type"),
                    semicolon_span,
                });
            }
        }
        f.text(" ");
        let close_brace_span = f.text("}");
        let kind = if is_enum {
            RawDataDeclarationKind::Enum {
                interface_span,
                name,
                extends_span,
                marker_span,
                open_brace_span,
                close_brace_span,
                variants,
            }
        } else {
            RawDataDeclarationKind::Struct {
                interface_span,
                name,
                extends_span,
                marker_span,
                open_brace_span,
                close_brace_span,
                fields,
            }
        };
        result.push(RawDataDeclaration {
            span: at(start, f.source.len()),
            export_span: None,
            kind,
        });
        f.text("\n");
    }
    result
}

fn entry(f: &mut Builder, data: bool) -> u32 {
    let start = f.source.len();
    let type_name = f.name("Entry");
    let dot_span = f.text(".");
    let variant = f.name(if data { "Data" } else { "Empty" });
    let open_paren_span = f.text("(");
    let payload = data.then(|| payload(f));
    let close_paren_span = f.text(")");
    f.expression(
        start,
        RawExpressionKind::EnumConstruction {
            type_name,
            dot_span,
            variant,
            open_paren_span,
            payload,
            close_paren_span,
        },
    )
}

fn payload(f: &mut Builder) -> u32 {
    let start = f.source.len();
    let type_name = f.name("Payload");
    let open_paren_span = f.text("(");
    let open_brace_span = f.text("{");
    let mut fields = Vec::new();
    for label in ["strong", "weak", "text"] {
        if !fields.is_empty() {
            f.text(", ");
        }
        let start = f.source.len();
        let name = f.name(label);
        let colon_span = f.text(":");
        f.text(" ");
        let value = f.clone_value(|f| f.reference(label));
        fields.push(RawFieldInitializer {
            span: at(start, f.source.len()),
            kind: RawFieldInitializerKind::Explicit { name, colon_span, value },
        });
    }
    let close_brace_span = f.text("}");
    let close_paren_span = f.text(")");
    f.expression(
        start,
        RawExpressionKind::StructConstruction {
            type_name,
            open_paren_span,
            open_brace_span,
            fields,
            close_brace_span,
            close_paren_span,
        },
    )
}

fn vector(f: &mut Builder, ty: &Ty, populated: bool) -> u32 {
    let start = f.source.len();
    let type_syntax = f.ty(ty);
    let open_paren_span = f.text("(");
    let open_bracket_span = f.text("[");
    let mut elements = Vec::new();
    if populated {
        for data in [true, false, true] {
            if !elements.is_empty() {
                f.text(", ");
            }
            elements.push(entry(f, data));
        }
    }
    let close_bracket_span = f.text("]");
    let close_paren_span = f.text(")");
    f.expression(
        start,
        RawExpressionKind::VecConstruction {
            type_syntax,
            open_paren_span,
            open_bracket_span,
            elements,
            close_bracket_span,
            close_paren_span,
        },
    )
}

pub(in crate::data_ownership_v1) fn fixture(populated: bool) -> (String, RawProjectSyntaxSnapshot) {
    let mut f =
        Builder { source: String::new(), types: vec![], expressions: vec![], statements: vec![] };
    let declarations = declarations(&mut f);
    let vector_type = Ty::Vec(Box::new(Ty::Named("Entry")));
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("replace");
    f.text("(");
    let mut parameters = Vec::new();
    for (name, ty) in [
        ("old", vector_type.clone()),
        ("strong", Ty::Shared(Box::new(Ty::String))),
        ("weak", Ty::Weak(Box::new(Ty::String))),
        ("text", Ty::String),
    ] {
        if !parameters.is_empty() {
            f.text(", ");
        }
        parameters.push(f.parameter(name, &ty));
    }
    f.text("): ");
    let result_type = f.ty(&vector_type);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    f.local("target", &vector_type, true, "old");
    shared_weak_fixture::local(&mut f, "source", &vector_type, |f| {
        vector(f, &vector_type, populated)
    });
    let statement_start = f.source.len();
    let target = f.reference("target");
    f.text(" ");
    let equals_span = f.text("=");
    f.text(" ");
    let value = f.clone_value(|f| f.reference("source"));
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(statement_start, f.source.len()),
        kind: RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
    });
    f.text(" ");
    let statement_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = f.reference("target");
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(statement_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start as usize, close_brace_span.end as usize);
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
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                close_brace_span,
                statements: (0..u32::try_from(f.statements.len()).expect("bounded statements"))
                    .collect(),
            }],
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
                imports: vec![],
                type_syntax: f.types,
                data_declarations: declarations,
                functions: vec![function],
            }],
            diagnostics: vec![],
        },
    )
}
