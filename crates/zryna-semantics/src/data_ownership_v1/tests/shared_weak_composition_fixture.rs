use super::*;
use zryna_syntax::v4::{RawEnumVariant, RawFieldInitializer, RawFieldInitializerKind};

fn field(f: &mut Builder, base_name: &str, field_name: &str) -> u32 {
    let start = f.source.len();
    let base = f.reference(base_name);
    let dot_span = f.text(".");
    let field = f.name(field_name);
    f.expression(start, RawExpressionKind::FieldAccess { base, dot_span, field })
}

fn index(f: &mut Builder, base_name: &str) -> u32 {
    let start = f.source.len();
    let base = f.reference(base_name);
    let open_bracket_span = f.text("[");
    let literal_start = f.source.len();
    f.text("0");
    let index = f.expression(literal_start, RawExpressionKind::I32Literal { spelling: "0".into() });
    let close_bracket_span = f.text("]");
    f.expression(
        start,
        RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
    )
}

fn struct_value(f: &mut Builder, strong_source: &str, weak_source: &str) -> u32 {
    let start = f.source.len();
    let type_name = f.name("Bundle");
    let open_paren_span = f.text("(");
    let open_brace_span = f.text("{");
    let strong_start = f.source.len();
    let strong_name = f.name("strong");
    let colon_span = f.text(":");
    f.text(" ");
    let strong = f.reference(strong_source);
    let strong_end = f.source.len();
    f.text(", ");
    let weak_start = f.source.len();
    let weak_name = f.name("weak");
    let weak_colon = f.text(":");
    f.text(" ");
    let weak = f.reference(weak_source);
    let weak_end = f.source.len();
    let close_brace_span = f.text("}");
    let close_paren_span = f.text(")");
    f.expression(
        start,
        RawExpressionKind::StructConstruction {
            type_name,
            open_paren_span,
            open_brace_span,
            fields: vec![
                RawFieldInitializer {
                    span: at(strong_start, strong_end),
                    kind: RawFieldInitializerKind::Explicit {
                        name: strong_name,
                        colon_span,
                        value: strong,
                    },
                },
                RawFieldInitializer {
                    span: at(weak_start, weak_end),
                    kind: RawFieldInitializerKind::Explicit {
                        name: weak_name,
                        colon_span: weak_colon,
                        value: weak,
                    },
                },
            ],
            close_brace_span,
            close_paren_span,
        },
    )
}

fn container_value(f: &mut Builder, ty: &Ty, vector: bool, source: &str) -> u32 {
    let start = f.source.len();
    let type_syntax = f.ty(ty);
    let open_paren_span = f.text("(");
    let open_bracket_span = f.text("[");
    let value = f.reference(source);
    let close_bracket_span = f.text("]");
    let close_paren_span = f.text(")");
    let common = (type_syntax, open_paren_span, open_bracket_span, vec![value]);
    let kind = if vector {
        RawExpressionKind::VecConstruction {
            type_syntax: common.0,
            open_paren_span: common.1,
            open_bracket_span: common.2,
            elements: common.3,
            close_bracket_span,
            close_paren_span,
        }
    } else {
        RawExpressionKind::FixedArrayConstruction {
            type_syntax: common.0,
            open_paren_span: common.1,
            open_bracket_span: common.2,
            elements: common.3,
            close_bracket_span,
            close_paren_span,
        }
    };
    f.expression(start, kind)
}

fn declaration(f: &mut Builder) -> RawDataDeclaration {
    let start = f.source.len();
    let interface_span = f.text("interface");
    f.text(" ");
    let name = f.name("Bundle");
    f.text(" extends ");
    let extends_span = at(name.span.end as usize + 1, f.source.len() - 1);
    let marker_span = f.text("ZrynaStruct");
    f.text(" {");
    let open_brace_span = at(f.source.len() - 1, f.source.len());
    f.text(" ");
    let mut fields = Vec::new();
    for (field_name, ty) in
        [("strong", Ty::Shared(Box::new(Ty::String))), ("weak", Ty::Weak(Box::new(Ty::String)))]
    {
        let field_start = f.source.len();
        let name = f.name(field_name);
        let colon_span = f.text(":");
        f.text(" ");
        let type_syntax = f.ty(&ty);
        let semicolon_span = f.text(";");
        fields.push(RawDataField {
            span: at(field_start, f.source.len()),
            name,
            colon_span,
            type_syntax,
            semicolon_span,
        });
        f.text(" ");
    }
    let close_brace_span = f.text("}");
    f.text("\n");
    RawDataDeclaration {
        span: at(start, close_brace_span.end as usize),
        export_span: None,
        kind: RawDataDeclarationKind::Struct {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            close_brace_span,
            fields,
        },
    }
}

fn enum_declaration(f: &mut Builder) -> RawDataDeclaration {
    let start = f.source.len();
    let interface_span = f.text("interface");
    f.text(" ");
    let name = f.name("Envelope");
    f.text(" extends ");
    let extends_span = at(name.span.end as usize + 1, f.source.len() - 1);
    let marker_span = f.text("ZrynaEnum");
    f.text(" {");
    let open_brace_span = at(f.source.len() - 1, f.source.len());
    f.text(" ");
    let item_start = f.source.len();
    let item_name = f.name("Some");
    let colon_span = f.text(":");
    f.text(" ");
    let payload_type = f.ty(&Ty::Named("Bundle"));
    let semicolon_span = f.text(";");
    let item = RawEnumVariant {
        span: at(item_start, f.source.len()),
        name: item_name,
        colon_span,
        payload_type: Some(payload_type),
        none_span: None,
        semicolon_span,
    };
    f.text(" ");
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
            variants: vec![item],
        },
    }
}

fn enum_value(f: &mut Builder, source: &str) -> u32 {
    let start = f.source.len();
    let type_name = f.name("Envelope");
    let dot_span = f.text(".");
    let variant = f.name("Some");
    let open_paren_span = f.text("(");
    let payload = f.reference(source);
    let close_paren_span = f.text(")");
    f.expression(
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

fn call(f: &mut Builder, source: &str) -> u32 {
    let start = f.source.len();
    let callee = f.name("relay");
    let open_paren_span = f.text("(");
    let argument = f.reference(source);
    let close_paren_span = f.text(")");
    f.expression(
        start,
        RawExpressionKind::Call {
            callee,
            open_paren_span,
            arguments: vec![argument],
            close_paren_span,
        },
    )
}

fn relay_function(f: &mut Builder) -> RawFunctionSyntax {
    let envelope = Ty::Named("Envelope");
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("relay");
    f.text("(");
    let parameters = vec![f.parameter("incoming", &envelope)];
    f.text("): ");
    let result_type = f.ty(&envelope);
    f.text(" {");
    let open_brace_span = at(f.source.len() - 1, f.source.len());
    f.text(" return ");
    let keyword_span = at(f.source.len() - 7, f.source.len() - 1);
    let value = f.reference("incoming");
    let semicolon_span = f.text(";");
    let statement = RawStatementSyntax {
        span: at(keyword_span.start as usize, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    };
    f.text(" }");
    let close_brace_span = at(f.source.len() - 1, f.source.len());
    let body_span = at(open_brace_span.start as usize, f.source.len());
    let expressions = std::mem::take(&mut f.expressions);
    RawFunctionSyntax {
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
                statements: vec![0],
                close_brace_span,
            }],
            statements: vec![statement],
            expressions,
        },
    }
}

fn mutable_local(
    f: &mut Builder,
    name_text: &str,
    ty: &Ty,
    initializer: impl FnOnce(&mut Builder) -> u32,
) {
    let start = f.source.len();
    let keyword_span = f.text("let");
    f.text(" ");
    let name = f.name(name_text);
    f.text(": ");
    let type_syntax = f.ty(ty);
    f.text(" = ");
    let equals_span = at(f.source.len() - 2, f.source.len() - 1);
    let initializer = initializer(f);
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(start, f.source.len()),
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
    f.text(" ");
}

pub(crate) fn fixture() -> (String, RawProjectSyntaxSnapshot) {
    build(None)
}

pub(crate) fn clone_rejection_fixture() -> (String, RawProjectSyntaxSnapshot) {
    build(Some("bundle"))
}

pub(crate) fn missing_clone_source_fixture() -> (String, RawProjectSyntaxSnapshot) {
    build(Some("ghostx"))
}

fn build(structural_clone_source: Option<&str>) -> (String, RawProjectSyntaxSnapshot) {
    let mut f = Builder {
        source: String::new(),
        types: Vec::new(),
        expressions: Vec::new(),
        statements: Vec::new(),
    };
    let declarations = vec![declaration(&mut f), enum_declaration(&mut f)];
    let relay = relay_function(&mut f);
    let string = Ty::String;
    let shared = Ty::Shared(Box::new(string.clone()));
    let weak = Ty::Weak(Box::new(string.clone()));
    let bundle = Ty::Named("Bundle");
    let envelope = Ty::Named("Envelope");
    let weak_array = Ty::Array(Box::new(weak.clone()), 1);
    let shared_vec = Ty::Vec(Box::new(shared.clone()));
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("compose");
    f.text("(");
    let mut parameters = vec![f.parameter("payload", &string)];
    f.text(", ");
    parameters.push(f.parameter("spare", &weak));
    f.text(", ");
    parameters.push(f.parameter("replacement", &shared));
    f.text("): ");
    let result_type = f.ty(&shared);
    f.text(" {");
    let open_brace_span = at(f.source.len() - 1, f.source.len());
    f.text(" ");
    local(&mut f, "owner", &shared, |f| unary(f, "shared", |f| f.reference("payload")));
    local(&mut f, "copy", &shared, |f| unary(f, "clone", |f| f.reference("owner")));
    local(&mut f, "weak", &weak, |f| unary(f, "downgrade", |f| f.reference("owner")));
    mutable_local(&mut f, "bundle", &bundle, |f| struct_value(f, "copy", "spare"));
    if let Some(source) = structural_clone_source {
        local(&mut f, "bundleCopy", &bundle, |f| unary(f, "clone", |f| f.reference(source)));
    }
    local(&mut f, "fieldCopy", &shared, |f| unary(f, "clone", |f| field(f, "bundle", "strong")));
    let assignment_start = f.source.len();
    let target = field(&mut f, "bundle", "strong");
    f.text(" = ");
    let equals_span = at(f.source.len() - 2, f.source.len() - 1);
    let value = f.reference("replacement");
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(assignment_start, f.source.len()),
        kind: RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
    });
    f.text(" ");
    local(&mut f, "envelope", &envelope, |f| enum_value(f, "bundle"));
    local(&mut f, "called", &envelope, |f| call(f, "envelope"));
    local(&mut f, "weakItems", &weak_array, |f| container_value(f, &weak_array, false, "weak"));
    local(&mut f, "weakCopy", &weak, |f| unary(f, "clone", |f| index(f, "weakItems")));
    local(&mut f, "strongItems", &shared_vec, |f| {
        container_value(f, &shared_vec, true, "fieldCopy")
    });
    let return_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = f.reference("owner");
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" }");
    let close_brace_span = at(f.source.len() - 1, f.source.len());
    let body_span = at(open_brace_span.start as usize, f.source.len());
    let statements = std::mem::take(&mut f.statements);
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
                statements: (0..statements.len()).map(|id| id as u32).collect(),
                close_brace_span,
            }],
            statements,
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
                functions: vec![relay, function],
            }],
            diagnostics: Vec::new(),
        },
    )
}
