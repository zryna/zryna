use super::*;

#[derive(Clone, Copy)]
pub(crate) enum Base {
    Call,
    Construction,
}

fn vector(f: &mut Builder, ty: &Ty, owned: bool, length: u32) -> u32 {
    let start = f.source.len();
    let type_syntax = f.ty(ty);
    let open_paren_span = f.text("(");
    let open_bracket_span = f.text("[");
    let elements = (0..length)
        .map(|i| {
            if i != 0 {
                f.text(", ");
            }
            literal(f, owned)
        })
        .collect();
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

fn access(
    f: &mut Builder,
    ty: &Ty,
    base: Base,
    owned: bool,
    length: u32,
    selected: Option<i32>,
) -> u32 {
    let base = match base {
        Base::Call => call(f, "makeVec"),
        Base::Construction => vector(f, ty, owned, length),
    };
    index(f, base, selected, "firstIndex")
}

pub(crate) fn fixture(
    owned: bool,
    base: Base,
    length: u32,
    selected: Option<i32>,
    clone: bool,
    replace: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations, element) =
        initial(&if owned { Element::String } else { Element::I32 });
    let ty = Ty::Vec(Box::new(element.clone()));
    let mut functions = vec![function(&mut f, "observe", &[], &element, |f| {
        if replace {
            let start = f.source.len();
            let target = access(f, &ty, base, owned, length, selected);
            f.text(" ");
            let equals_span = f.text("=");
            f.text(" ");
            let value = literal(f, owned);
            let semicolon_span = f.text(";");
            f.statements.push(RawStatementSyntax {
                span: at(start, f.source.len()),
                kind: RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
            });
            f.text(" return ");
            literal(f, owned)
        } else {
            f.text("return ");
            if clone {
                f.clone_value(|f| access(f, &ty, base, owned, length, selected))
            } else {
                access(f, &ty, base, owned, length, selected)
            }
        }
    })];
    if matches!(base, Base::Call) {
        functions.push(function(&mut f, "makeVec", &[], &ty, |f| {
            f.text("return ");
            vector(f, &ty, owned, length)
        }));
    }
    functions.push(function(&mut f, "firstIndex", &[], &Ty::Named("i32"), |f| {
        f.text("return ");
        let start = f.source.len();
        f.text("0");
        f.expression(start, RawExpressionKind::I32Literal { spelling: "0".into() })
    }));
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
