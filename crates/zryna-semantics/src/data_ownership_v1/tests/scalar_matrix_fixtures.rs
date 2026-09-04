use super::*;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializer, RawFieldInitializerKind};

#[derive(Clone, Copy)]
pub(super) enum Binary {
    Add,
    Sub,
    Mul,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy)]
pub(super) enum Syntax {
    I32(&'static str),
    Bool(bool),
    Neg { token: usize, operand: u32 },
    Binary { token: usize, width: usize, kind: Binary, lhs: u32, rhs: u32 },
}

#[derive(Clone, Copy)]
pub(super) struct Node {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) kind: Syntax,
}

#[derive(Clone, Copy)]
pub(super) struct Expression {
    pub(super) text: &'static str,
    pub(super) nodes: &'static [Node],
}

pub(super) const ARITHMETIC: Expression = Expression {
    text: "7 - 3 * 2 + -1",
    nodes: &[
        Node { start: 0, end: 1, kind: Syntax::I32("7") },
        Node { start: 4, end: 5, kind: Syntax::I32("3") },
        Node { start: 8, end: 9, kind: Syntax::I32("2") },
        Node {
            start: 4,
            end: 9,
            kind: Syntax::Binary { token: 6, width: 1, kind: Binary::Mul, lhs: 1, rhs: 2 },
        },
        Node {
            start: 0,
            end: 9,
            kind: Syntax::Binary { token: 2, width: 1, kind: Binary::Sub, lhs: 0, rhs: 3 },
        },
        Node { start: 13, end: 14, kind: Syntax::I32("1") },
        Node { start: 12, end: 14, kind: Syntax::Neg { token: 12, operand: 5 } },
        Node {
            start: 0,
            end: 14,
            kind: Syntax::Binary { token: 10, width: 1, kind: Binary::Add, lhs: 4, rhs: 6 },
        },
    ],
};

pub(super) const BOOL_EQ: Expression = Expression {
    text: "true === false",
    nodes: &[
        Node { start: 0, end: 4, kind: Syntax::Bool(true) },
        Node { start: 9, end: 14, kind: Syntax::Bool(false) },
        Node {
            start: 0,
            end: 14,
            kind: Syntax::Binary { token: 5, width: 3, kind: Binary::Eq, lhs: 0, rhs: 1 },
        },
    ],
};

fn at(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("offset"),
        end: u32::try_from(end).expect("offset"),
    }
}

fn ident(text: &str, start: usize) -> RawIdentifierSyntax {
    RawIdentifierSyntax { text: text.into(), span: at(start, start + text.len()) }
}

// This maps a fixed DTO record, not source parsing, typing or operation evaluation.
fn node(raw: Node, start: usize, base: u32) -> RawExpressionSyntax {
    let kind = match raw.kind {
        Syntax::I32(spelling) => RawExpressionKind::I32Literal { spelling: spelling.into() },
        Syntax::Bool(value) => RawExpressionKind::BoolLiteral { value },
        Syntax::Neg { token, operand } => RawExpressionKind::Negation {
            operator_span: at(start + token, start + token + 1),
            operand: base + operand,
        },
        Syntax::Binary { token, width, kind, lhs, rhs } => {
            let operator_span = at(start + token, start + token + width);
            let (lhs, rhs) = (base + lhs, base + rhs);
            match kind {
                Binary::Add => RawExpressionKind::Addition { operator_span, lhs, rhs },
                Binary::Sub => RawExpressionKind::Subtraction { operator_span, lhs, rhs },
                Binary::Mul => RawExpressionKind::Multiplication { operator_span, lhs, rhs },
                Binary::Eq => RawExpressionKind::Equal { operator_span, lhs, rhs },
                Binary::Ne => RawExpressionKind::NotEqual { operator_span, lhs, rhs },
                Binary::Lt => RawExpressionKind::LessThan { operator_span, lhs, rhs },
                Binary::Le => RawExpressionKind::LessEqual { operator_span, lhs, rhs },
                Binary::Gt => RawExpressionKind::GreaterThan { operator_span, lhs, rhs },
                Binary::Ge => RawExpressionKind::GreaterEqual { operator_span, lhs, rhs },
            }
        }
    };
    RawExpressionSyntax { span: at(start + raw.start, start + raw.end), kind }
}

fn insert(
    source: &mut String,
    raw: RawProjectSyntaxSnapshot,
    start: usize,
    text: &str,
) -> RawProjectSyntaxSnapshot {
    source.insert_str(start, text);
    shift_snapshot(
        raw,
        u32::try_from(start).expect("offset"),
        u32::try_from(text.len()).expect("length"),
    )
}

pub(super) fn fixture(count: Expression, flag: Expression) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = mixed_construction::mixed_fixture(false);
    let end = usize::try_from(raw.files[0].data_declarations[0].span.end).expect("end") - 1;
    raw = insert(&mut source, raw, end, " count: i32; flag: bool;");
    let (count_decl, flag_decl) = (end + 1, end + 13);
    assert_eq!(&source[count_decl..count_decl + 11], "count: i32;");
    assert_eq!(&source[flag_decl..flag_decl + 11], "flag: bool;");
    assert_eq!(raw.files[0].type_syntax.len(), 5);
    raw.files[0].type_syntax.extend([
        RawTypeSyntax {
            span: at(count_decl + 7, count_decl + 10),
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".into(),
                    span: at(count_decl + 7, count_decl + 10),
                },
            },
        },
        RawTypeSyntax {
            span: at(flag_decl + 6, flag_decl + 10),
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "bool".into(),
                    span: at(flag_decl + 6, flag_decl + 10),
                },
            },
        },
    ]);
    let RawDataDeclarationKind::Struct { fields, .. } = &mut raw.files[0].data_declarations[0].kind
    else {
        panic!("Parcel")
    };
    for (name, start, colon, ty) in [("count", count_decl, 5, 5), ("flag", flag_decl, 4, 6)] {
        fields.push(RawDataField {
            span: at(start, start + 11),
            name: ident(name, start),
            colon_span: at(start + colon, start + colon + 1),
            type_syntax: ty,
            semicolon_span: at(start + 10, start + 11),
        });
    }
    let RawExpressionKind::StructConstruction { close_brace_span, .. } =
        raw.files[0].functions[0].body.expressions[2].kind
    else {
        panic!("Parcel initializer")
    };
    let start = usize::try_from(close_brace_span.start).expect("offset");
    let text = format!(", count: {}, flag: {}", count.text, flag.text);
    raw = insert(&mut source, raw, start, &text);
    let count_start = start + 9;
    let flag_name = count_start + count.text.len() + 2;
    let flag_start = flag_name + 6;
    let body = &mut raw.files[0].functions[0].body;
    let mut structure = body.expressions.pop().expect("Parcel");
    assert_eq!(body.expressions.len(), 2);
    body.expressions.extend(count.nodes.iter().map(|record| node(*record, count_start, 2)));
    let count_result = u32::try_from(body.expressions.len() - 1).expect("count result");
    let flag_base = count_result + 1;
    body.expressions.extend(flag.nodes.iter().map(|record| node(*record, flag_start, flag_base)));
    let flag_result = u32::try_from(body.expressions.len() - 1).expect("flag result");
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("Parcel fields")
    };
    for (name, begin, end, colon, value) in [
        ("count", start + 2, count_start + count.text.len(), 5, count_result),
        ("flag", flag_name, flag_start + flag.text.len(), 4, flag_result),
    ] {
        fields.push(RawFieldInitializer {
            span: at(begin, end),
            kind: RawFieldInitializerKind::Explicit {
                name: ident(name, begin),
                colon_span: at(begin + colon, begin + colon + 1),
                value,
            },
        });
    }
    body.expressions.push(structure);
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("return")
    };
    *value = flag_result + 1;
    (source, raw)
}
