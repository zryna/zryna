use super::*;

#[derive(Clone, Copy)]
pub(crate) enum Source {
    Whole,
    Static,
    Dynamic,
}

pub(crate) fn fixture(
    owned: bool,
    length: u32,
    checked_index: Option<i32>,
    replace: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    sourced_fixture(owned, length, checked_index, replace, Source::Whole)
}

pub(crate) fn sourced_fixture(
    owned: bool,
    length: u32,
    checked_index: Option<i32>,
    replace: bool,
    source: Source,
) -> (String, RawProjectSyntaxSnapshot) {
    let projected = !matches!(source, Source::Whole);
    let element = if owned { Element::String } else { Element::I32 };
    let (mut f, declarations, element) = initial(&element);
    let container = Ty::Array(Box::new(element.clone()), length);
    let root =
        if projected { Ty::Array(Box::new(container.clone()), 2) } else { container.clone() };
    let caller = function(
        &mut f,
        "observe",
        &[("incoming", root.clone())],
        if replace { &root } else { &element },
        |f| {
            f.local("items", &root, true, "incoming");
            let access = |f: &mut Builder| {
                let base = f.clone_value(|f| {
                    let root = f.reference("items");
                    if projected {
                        index(
                            f,
                            root,
                            if matches!(source, Source::Dynamic) { None } else { Some(0) },
                            "sourceIndex",
                        )
                    } else {
                        root
                    }
                });
                index(f, base, checked_index, "firstIndex")
            };
            if replace {
                let start = f.source.len();
                let target = access(f);
                f.text(" ");
                let equals_span = f.text("=");
                f.text(" ");
                let value = call(f, "replacement");
                let semicolon_span = f.text(";");
                f.statements.push(RawStatementSyntax {
                    span: at(start, f.source.len()),
                    kind: RawStatementKind::Assignment {
                        target,
                        equals_span,
                        value,
                        semicolon_span,
                    },
                });
                f.text(" return ");
                f.reference("items")
            } else {
                f.text("return ");
                if owned { f.clone_value(access) } else { access(f) }
            }
        },
    );
    let mut functions = vec![
        caller,
        function(&mut f, "firstIndex", &[], &Ty::Named("i32"), |f| {
            f.text("return ");
            integer_value(f, 0)
        }),
    ];
    if matches!(source, Source::Dynamic) {
        functions.push(function(&mut f, "sourceIndex", &[], &Ty::Named("i32"), |f| {
            f.text("return ");
            integer_value(f, 0)
        }));
    }
    if replace {
        functions.push(function(&mut f, "replacement", &[], &element, |f| {
            f.text("return ");
            literal(f, owned)
        }));
    }
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
                functions,
            }],
            diagnostics: Vec::new(),
        },
    )
}

fn integer_value(f: &mut Builder, number: i32) -> u32 {
    let start = f.source.len();
    let spelling = number.to_string();
    f.text(&spelling);
    f.expression(start, RawExpressionKind::I32Literal { spelling })
}
