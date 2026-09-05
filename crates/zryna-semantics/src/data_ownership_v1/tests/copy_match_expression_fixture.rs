use super::*;
use zryna_syntax::v4::{RawFieldInitializer, RawFieldInitializerKind, RawMatchArm};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Arm {
    Add,
    Call,
    Array,
    LargeArray(u32),
    Enum,
    Struct,
    WrongType,
    Missing,
    Mismatch,
    Nested,
    FreshScrutinee,
    Projection,
}

fn integer(f: &mut Builder, value: i32) -> u32 {
    let start = f.source.len();
    let spelling = value.to_string();
    f.text(&spelling);
    f.expression(start, RawExpressionKind::I32Literal { spelling })
}

fn operand(f: &mut Builder, payload: bool) -> u32 {
    if payload { f.reference("value") } else { integer(f, 0) }
}

fn array_operand(f: &mut Builder, payload: bool, negate: bool) -> u32 {
    if !negate {
        return operand(f, payload);
    }
    let start = f.source.len();
    let operator_span = f.text("-");
    let operand = operand(f, payload);
    f.expression(start, RawExpressionKind::Negation { operator_span, operand })
}

fn arm_value(f: &mut Builder, arm: Arm, payload: bool) -> u32 {
    let start = f.source.len();
    match arm {
        Arm::Add | Arm::FreshScrutinee => {
            let lhs = operand(f, payload);
            f.text(" ");
            let operator_span = f.text("+");
            f.text(" ");
            let rhs = integer(f, 1);
            f.expression(start, RawExpressionKind::Addition { lhs, operator_span, rhs })
        }
        Arm::Call => {
            let callee = f.name("identity");
            let open_paren_span = f.text("(");
            let argument = operand(f, payload);
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
        Arm::Array | Arm::LargeArray(_) => array_arm(f, arm, payload),
        Arm::Struct => struct_arm(f, payload),
        Arm::Enum => {
            let type_name = f.name("Maybe");
            let dot_span = f.text(".");
            let variant = f.name("some");
            let open_paren_span = f.text("(");
            let payload = Some(operand(f, payload));
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
        Arm::WrongType => {
            let lhs = operand(f, payload);
            f.text(" ");
            let operator_span = f.text("+");
            f.text(" ");
            let literal = f.source.len();
            f.text("true");
            let rhs = f.expression(literal, RawExpressionKind::BoolLiteral { value: true });
            f.expression(start, RawExpressionKind::Addition { lhs, operator_span, rhs })
        }
        Arm::Missing => {
            let lhs = f.reference("missing");
            f.text(" ");
            let operator_span = f.text("+");
            f.text(" ");
            let rhs = integer(f, 1);
            f.expression(start, RawExpressionKind::Addition { lhs, operator_span, rhs })
        }
        Arm::Mismatch => {
            f.text("true");
            f.expression(start, RawExpressionKind::BoolLiteral { value: true })
        }
        Arm::Nested => match_value(f, Arm::Add, true, false),
        Arm::Projection => {
            let base = f.reference("items");
            let open_bracket_span = f.text("[");
            let index = integer(f, 0);
            let close_bracket_span = f.text("]");
            let lhs = f.expression(
                start,
                RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
            );
            f.text(" ");
            let operator_span = f.text("+");
            f.text(" ");
            let rhs = operand(f, payload);
            f.expression(start, RawExpressionKind::Addition { lhs, operator_span, rhs })
        }
    }
}

fn array_arm(f: &mut Builder, arm: Arm, payload: bool) -> u32 {
    let start = f.source.len();
    let length = if let Arm::LargeArray(length) = arm { length } else { 2 };
    let type_syntax = f.ty(&Ty::Array(Box::new(Ty::Named("i32")), length));
    let open_paren_span = f.text("(");
    let open_bracket_span = f.text("[");
    let first = array_operand(f, payload, matches!(arm, Arm::LargeArray(_)));
    let mut elements = vec![first];
    for _ in 1..length {
        f.text(", ");
        elements.push(array_operand(f, false, matches!(arm, Arm::LargeArray(_))));
    }
    let close_bracket_span = f.text("]");
    let close_paren_span = f.text(")");
    f.expression(
        start,
        RawExpressionKind::FixedArrayConstruction {
            type_syntax,
            open_paren_span,
            open_bracket_span,
            elements,
            close_bracket_span,
            close_paren_span,
        },
    )
}

fn struct_arm(f: &mut Builder, payload: bool) -> u32 {
    let start = f.source.len();
    let type_name = f.name("Pair");
    let open_paren_span = f.text("(");
    let open_brace_span = f.text("{");
    f.text(" ");
    let field_start = f.source.len();
    let name = f.name("item");
    let colon_span = f.text(":");
    f.text(" ");
    let value = operand(f, payload);
    let fields = vec![RawFieldInitializer {
        span: at(field_start, f.source.len()),
        kind: RawFieldInitializerKind::Explicit { name, colon_span, value },
    }];
    f.text(" ");
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

fn match_value(f: &mut Builder, arm: Arm, exhaustive: bool, reverse: bool) -> u32 {
    let start = f.source.len();
    let keyword_span = f.text("match");
    let open_paren_span = f.text("(");
    let scrutinee = if matches!(arm, Arm::FreshScrutinee) {
        arm_value(f, Arm::Enum, false)
    } else {
        f.reference("source")
    };
    f.text(", ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let mut arms = Vec::new();
    let variants: &[bool] = if !exhaustive {
        &[true]
    } else if reverse {
        &[true, false]
    } else {
        &[false, true]
    };
    for (index, payload) in variants.iter().copied().enumerate() {
        if index != 0 {
            f.text(", ");
        }
        let arm_start = f.source.len();
        f.text("\"");
        let type_name = f.name("Maybe");
        let dot_span = f.text(".");
        let variant = f.name(if payload { "some" } else { "none" });
        f.text("\": (");
        let binding = payload.then(|| f.name("value"));
        f.text(") ");
        let arrow_span = f.text("=>");
        f.text(" ");
        let value = arm_value(f, arm, payload);
        arms.push(RawMatchArm {
            span: at(arm_start, f.source.len()),
            type_name,
            dot_span,
            variant,
            binding,
            arrow_span,
            value,
        });
    }
    f.text(" ");
    let close_brace_span = f.text("}");
    let close_paren_span = f.text(")");
    f.expression(
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

pub(crate) fn fixture(
    arm: Arm,
    exhaustive: bool,
    reverse: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    padded_fixture(arm, exhaustive, reverse, 0)
}

pub(crate) fn padded_fixture(
    arm: Arm,
    exhaustive: bool,
    reverse: bool,
    padding: usize,
) -> (String, RawProjectSyntaxSnapshot) {
    let mut raw = response_snapshot(ENUM_RESPONSE);
    let prefix = ENUM_SOURCE.find('\n').expect("enum declaration");
    let mut f = Builder {
        source: ENUM_SOURCE[..prefix].into(),
        types: vec![raw.files[0].type_syntax[0].clone()],
        expressions: Vec::new(),
        statements: Vec::new(),
    };
    if matches!(arm, Arm::Struct) {
        raw.files[0].data_declarations.push(struct_declaration(&mut f));
    }
    let result = match arm {
        Arm::Array => Ty::Array(Box::new(Ty::Named("i32")), 2),
        Arm::LargeArray(length) => Ty::Array(Box::new(Ty::Named("i32")), length),
        Arm::Enum => Ty::Named("Maybe"),
        Arm::Struct => Ty::Named("Pair"),
        _ => Ty::Named("i32"),
    };
    let mut parameters = vec![("source", Ty::Named("Maybe"))];
    if matches!(arm, Arm::Projection) {
        parameters.push(("items", Ty::Array(Box::new(Ty::Named("i32")), 2)));
    }
    for name in ["paddingOne", "paddingTwo"].into_iter().take(padding) {
        parameters.push((name, Ty::Named("i32")));
    }
    let caller = function(&mut f, "get", &parameters, &result, |f| {
        f.text("return ");
        match_value(f, arm, exhaustive, reverse)
    });
    let mut functions = vec![caller];
    if matches!(arm, Arm::Call) {
        functions.push(function(
            &mut f,
            "identity",
            &[("arg", Ty::Named("i32"))],
            &Ty::Named("i32"),
            |f| {
                f.text("return ");
                f.reference("arg")
            },
        ));
    }
    raw.files[0].functions = functions;
    raw.files[0].type_syntax = f.types;
    (f.source, raw)
}

fn struct_declaration(f: &mut Builder) -> RawDataDeclaration {
    f.text("\n");
    let start = f.source.len();
    let interface_span = f.text("interface");
    f.text(" ");
    let name = f.name("Pair");
    f.text(" ");
    let extends_span = f.text("extends");
    f.text(" ");
    let marker_span = f.text("ZrynaStruct");
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let field_start = f.source.len();
    let field_name = f.name("item");
    let colon_span = f.text(":");
    f.text(" ");
    let type_syntax = f.ty(&Ty::Named("i32"));
    let semicolon_span = f.text(";");
    let fields = vec![RawDataField {
        span: at(field_start, f.source.len()),
        name: field_name,
        colon_span,
        type_syntax,
        semicolon_span,
    }];
    f.text(" ");
    let close_brace_span = f.text("}");
    RawDataDeclaration {
        span: at(start, f.source.len()),
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
