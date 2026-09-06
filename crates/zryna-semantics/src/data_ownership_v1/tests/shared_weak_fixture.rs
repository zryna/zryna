use super::*;
use zryna_syntax::v4::RawEnumVariant;

#[path = "shared_weak_composition_fixture.rs"]
pub(in crate::data_ownership_v1) mod composition_fixture;

fn unary(f: &mut Builder, spelling: &str, value: impl FnOnce(&mut Builder) -> u32) -> u32 {
    let start = f.source.len();
    let keyword_span = f.text(spelling);
    let open_paren_span = f.text("(");
    let value = value(f);
    let close_paren_span = f.text(")");
    let kind = match spelling {
        "shared" => {
            RawExpressionKind::Shared { keyword_span, open_paren_span, value, close_paren_span }
        }
        "clone" => {
            RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span }
        }
        "downgrade" => {
            RawExpressionKind::Downgrade { keyword_span, open_paren_span, value, close_paren_span }
        }
        _ => unreachable!("fixture unary operation"),
    };
    f.expression(start, kind)
}

pub(super) fn local(
    f: &mut Builder,
    name: &str,
    ty: &Ty,
    initializer: impl FnOnce(&mut Builder) -> u32,
) {
    let start = f.source.len();
    let keyword_span = f.text("const");
    f.text(" ");
    let name = f.name(name);
    f.text(": ");
    let type_syntax = f.ty(ty);
    f.text(" ");
    let equals_span = f.text("=");
    f.text(" ");
    let initializer = initializer(f);
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(start, f.source.len()),
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
    f.text(" ");
}

#[derive(Clone, Copy)]
pub(super) enum NominalPayload {
    Struct,
    Enum,
    MultiVariantEnum,
    RecursiveEnum,
}

fn enum_variant(f: &mut Builder, name: &str, payload: Option<&Ty>) -> RawEnumVariant {
    let start = f.source.len();
    let name = f.name(name);
    let colon_span = f.text(":");
    f.text(" ");
    let (payload_type, none_span) = if let Some(payload) = payload {
        (Some(f.ty(payload)), None)
    } else {
        (None, Some(f.text("ZrynaNone")))
    };
    let semicolon_span = f.text(";");
    RawEnumVariant {
        span: at(start, semicolon_span.end as usize),
        name,
        colon_span,
        payload_type,
        none_span,
        semicolon_span,
    }
}

pub(super) fn nominal_payload(f: &mut Builder, shape: NominalPayload) -> RawDataDeclaration {
    let start = f.source.len();
    let interface_span = f.text("interface");
    f.text(" ");
    let name = f.name("Payload");
    f.text(" ");
    let extends_span = f.text("extends");
    f.text(" ");
    let is_enum = !matches!(shape, NominalPayload::Struct);
    let marker_span = f.text(if is_enum { "ZrynaEnum" } else { "ZrynaStruct" });
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let member_start = f.source.len();
    let (member, colon_span, string, semicolon_span, variants) = if is_enum {
        let mut variants = Vec::new();
        if matches!(shape, NominalPayload::MultiVariantEnum | NominalPayload::RecursiveEnum) {
            variants.push(enum_variant(f, "Empty", None));
            f.text(" ");
        }
        let payload = if matches!(shape, NominalPayload::RecursiveEnum) {
            Ty::Shared(Box::new(Ty::Named("Payload")))
        } else {
            Ty::String
        };
        variants.push(enum_variant(f, "Value", Some(&payload)));
        if matches!(shape, NominalPayload::MultiVariantEnum | NominalPayload::RecursiveEnum) {
            f.text(" ");
            variants.push(enum_variant(f, "Text", Some(&Ty::String)));
        }
        (None, None, None, None, Some(variants))
    } else {
        let member = f.name("value");
        let colon_span = f.text(":");
        f.text(" ");
        let string = f.ty(&Ty::String);
        let semicolon_span = f.text(";");
        (Some(member), Some(colon_span), Some(string), Some(semicolon_span), None)
    };
    f.text(" ");
    let close_brace_span = f.text("}");
    f.text("\n");
    let kind = if let Some(variants) = variants {
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
            fields: vec![RawDataField {
                span: at(member_start, semicolon_span.expect("struct terminator").end as usize),
                name: member.expect("struct member"),
                colon_span: colon_span.expect("struct colon"),
                type_syntax: string.expect("struct type"),
                semicolon_span: semicolon_span.expect("struct terminator"),
            }],
        }
    };
    RawDataDeclaration { span: at(start, close_brace_span.end as usize), export_span: None, kind }
}

#[derive(Clone, Copy)]
pub(crate) enum Case {
    Positive,
    NestedShared,
    ScalarBool,
    ScalarI32,
    NominalEnum,
    MultiVariantEnum,
    RecursiveEnum,
    NominalStruct,
    StringArrayZero,
    StringArrayOne,
    StringVec,
    TemporaryOperands,
    MissingHandle,
    MovedReuse,
    WrongCloneType,
}

pub(crate) fn fixture() -> (String, RawProjectSyntaxSnapshot) {
    fixture_case(Case::Positive)
}

#[allow(clippy::too_many_lines)]
pub(crate) fn fixture_case(case: Case) -> (String, RawProjectSyntaxSnapshot) {
    let mut f = Builder {
        source: String::new(),
        types: Vec::new(),
        expressions: Vec::new(),
        statements: Vec::new(),
    };
    let data_declarations = match case {
        Case::NominalEnum => vec![nominal_payload(&mut f, NominalPayload::Enum)],
        Case::MultiVariantEnum => {
            vec![nominal_payload(&mut f, NominalPayload::MultiVariantEnum)]
        }
        Case::RecursiveEnum => vec![nominal_payload(&mut f, NominalPayload::RecursiveEnum)],
        Case::NominalStruct => vec![nominal_payload(&mut f, NominalPayload::Struct)],
        _ => Vec::new(),
    };
    let parameter = match case {
        Case::ScalarBool => Ty::Named("bool"),
        Case::ScalarI32 => Ty::Named("i32"),
        Case::StringArrayZero => Ty::Array(Box::new(Ty::String), 0),
        Case::StringArrayOne => Ty::Array(Box::new(Ty::String), 1),
        Case::StringVec => Ty::Vec(Box::new(Ty::String)),
        Case::NominalEnum | Case::MultiVariantEnum | Case::RecursiveEnum | Case::NominalStruct => {
            Ty::Named("Payload")
        }
        _ => Ty::String,
    };
    let payload = if matches!(case, Case::NestedShared) {
        Ty::Shared(Box::new(parameter.clone()))
    } else {
        parameter.clone()
    };
    let shared = Ty::Shared(Box::new(payload.clone()));
    let weak = Ty::Weak(Box::new(payload.clone()));
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("handles");
    f.text("(");
    let parameters = vec![f.parameter("payload", &parameter)];
    f.text("): ");
    let result_type = f.ty(&shared);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    if matches!(case, Case::NestedShared) {
        local(&mut f, "inner", &payload, |f| unary(f, "shared", |f| f.reference("payload")));
        local(&mut f, "owner", &shared, |f| unary(f, "shared", |f| f.reference("inner")));
    } else {
        local(&mut f, "owner", &shared, |f| unary(f, "shared", |f| f.reference("payload")));
    }
    match case {
        Case::Positive
        | Case::NestedShared
        | Case::ScalarBool
        | Case::ScalarI32
        | Case::NominalEnum
        | Case::MultiVariantEnum
        | Case::RecursiveEnum
        | Case::NominalStruct
        | Case::StringArrayZero
        | Case::StringArrayOne
        | Case::StringVec => {
            local(&mut f, "copy", &shared, |f| unary(f, "clone", |f| f.reference("owner")));
            local(&mut f, "weak", &weak, |f| unary(f, "downgrade", |f| f.reference("copy")));
            local(&mut f, "weakCopy", &weak, |f| unary(f, "clone", |f| f.reference("weak")));
        }
        Case::TemporaryOperands => {
            local(&mut f, "copy", &shared, |f| {
                unary(f, "clone", |f| unary(f, "clone", |f| f.reference("owner")))
            });
            local(&mut f, "weak", &weak, |f| {
                unary(f, "downgrade", |f| unary(f, "clone", |f| f.reference("owner")))
            });
            local(&mut f, "weakCopy", &weak, |f| {
                unary(f, "clone", |f| unary(f, "downgrade", |f| f.reference("owner")))
            });
        }
        Case::MissingHandle => {
            local(&mut f, "copy", &shared, |f| unary(f, "clone", |f| f.reference("ghost")));
        }
        Case::MovedReuse => {
            local(&mut f, "moved", &shared, |f| f.reference("owner"));
            local(&mut f, "copy", &shared, |f| unary(f, "clone", |f| f.reference("owner")));
        }
        Case::WrongCloneType => {
            local(&mut f, "weak", &weak, |f| unary(f, "downgrade", |f| f.reference("owner")));
            local(&mut f, "copy", &shared, |f| unary(f, "clone", |f| f.reference("weak")));
        }
    }
    let return_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = f.reference(if matches!(case, Case::MovedReuse) { "moved" } else { "owner" });
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
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
                statements: (0..statements.len())
                    .map(|id| u32::try_from(id).expect("statement"))
                    .collect(),
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
                data_declarations,
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}
