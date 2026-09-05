use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Shape {
    ArrayVec,
    VecVec,
    Alternating,
}

impl Shape {
    pub(crate) fn containers(self) -> &'static [bool] {
        match self {
            Self::ArrayVec => &[false, true],
            Self::VecVec => &[true, true],
            Self::Alternating => &[false, true, false, true],
        }
    }
}

fn construction(f: &mut Builder, ty: &Ty, depth: usize, empty: Option<usize>) -> u32 {
    let (element, count, vector) = match ty {
        Ty::Array(element, length) => (element, *length, false),
        Ty::Vec(element) => (element, u32::from(empty != Some(depth)), true),
        Ty::String => return literal(f, true),
        Ty::Named("i32") => return literal(f, false),
        _ => unreachable!("fixture leaf"),
    };
    let start = f.source.len();
    let type_syntax = f.ty(ty);
    let open_paren_span = f.text("(");
    let open_bracket_span = f.text("[");
    let elements = (0..count)
        .map(|ordinal| {
            if ordinal != 0 {
                f.text(", ");
            }
            construction(f, element, depth + 1, empty)
        })
        .collect();
    let close_bracket_span = f.text("]");
    let close_paren_span = f.text(")");
    f.expression(
        start,
        if vector {
            RawExpressionKind::VecConstruction {
                type_syntax,
                open_paren_span,
                open_bracket_span,
                elements,
                close_bracket_span,
                close_paren_span,
            }
        } else {
            RawExpressionKind::FixedArrayConstruction {
                type_syntax,
                open_paren_span,
                open_bracket_span,
                elements,
                close_bracket_span,
                close_paren_span,
            }
        },
    )
}

fn indexed(f: &mut Builder, shape: Shape, bad: Option<(usize, i32)>) -> u32 {
    let mut value = f.reference("items");
    for ordinal in 0..shape.containers().len() {
        value = index(
            f,
            value,
            bad.filter(|(at, _)| *at == ordinal).map(|(_, value)| value),
            &format!("index{ordinal}"),
        );
    }
    value
}

fn statement(
    f: &mut Builder,
    element: &Ty,
    shape: Shape,
    owned: bool,
    replace: bool,
    bad: Option<(usize, i32)>,
) -> u32 {
    let start = f.source.len();
    let kind = if replace {
        let target = indexed(f, shape, bad);
        f.text(" ");
        let equals_span = f.text("=");
        f.text(" ");
        let value = call(f, "replacement");
        RawStatementKind::Assignment { target, equals_span, value, semicolon_span: f.text(";") }
    } else {
        let keyword_span = f.text("const");
        f.text(" ");
        let name = f.name("seen");
        f.text(": ");
        let type_syntax = f.ty(element);
        f.text(" ");
        let equals_span = f.text("=");
        f.text(" ");
        let initializer =
            if owned { f.clone_value(|f| indexed(f, shape, bad)) } else { indexed(f, shape, bad) };
        RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span: f.text(";"),
        }
    };
    let id = u32::try_from(f.statements.len()).expect("statement");
    f.statements.push(RawStatementSyntax { span: at(start, f.source.len()), kind });
    f.text(" ");
    id
}

pub(crate) fn fixture(
    shape: Shape,
    owned: bool,
    replace: bool,
    empty: Option<usize>,
    bad: Option<(usize, i32)>,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations, element) =
        initial(&if owned { Element::String } else { Element::I32 });
    let container = shape.containers().iter().rev().fold(element.clone(), |ty, vector| {
        if *vector { Ty::Vec(Box::new(ty)) } else { Ty::Array(Box::new(ty), 2) }
    });
    let mut nested = None;
    let mut caller = function(&mut f, "observe", &[], &container, |f| {
        let start = f.source.len();
        let keyword_span = f.text("let");
        f.text(" ");
        let name = f.name("items");
        f.text(": ");
        let type_syntax = f.ty(&container);
        f.text(" ");
        let equals_span = f.text("=");
        f.text(" ");
        let initializer = call(f, "makeContainer");
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
        let open_brace_span = f.text("{");
        f.text(" ");
        let block_statement = f.statements.len();
        f.statements.push(RawStatementSyntax {
            span: open_brace_span,
            kind: RawStatementKind::Block { block: 1 },
        });
        let operation = statement(f, &element, shape, owned, replace, bad);
        let close_brace_span = f.text("}");
        let span = at(open_brace_span.start as usize, close_brace_span.end as usize);
        f.statements[block_statement].span = span;
        nested = Some(RawBlockSyntax {
            span,
            open_brace_span,
            statements: vec![operation],
            close_brace_span,
        });
        f.text(" return ");
        f.reference("items")
    });
    caller.body.blocks[0].statements =
        vec![0, 1, u32::try_from(caller.body.statements.len() - 1).expect("return")];
    caller.body.blocks.push(nested.expect("nested scope"));
    let mut functions = vec![
        caller,
        function(&mut f, "makeContainer", &[], &container, |f| {
            f.text("return ");
            construction(f, &container, 0, empty)
        }),
    ];
    for ordinal in 0..shape.containers().len() {
        functions.push(function(&mut f, &format!("index{ordinal}"), &[], &Ty::Named("i32"), |f| {
            f.text("return ");
            let start = f.source.len();
            f.text("0");
            f.expression(start, RawExpressionKind::I32Literal { spelling: "0".into() })
        }));
    }
    if replace {
        functions.push(function(&mut f, "replacement", &[], &element, |f| {
            f.text("return ");
            literal(f, owned)
        }));
    }
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".into(),
            imports: Vec::new(),
            type_syntax: f.types,
            data_declarations: declarations,
            functions,
        }],
        diagnostics: Vec::new(),
    };
    (f.source, raw)
}
