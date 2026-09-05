use super::*;

#[path = "ordinary_static_prefix_fixture.rs"]
pub(crate) mod ordinary_static_prefix_fixture;

#[path = "checked_chain_fixture.rs"]
pub(crate) mod checked_chain_fixture;

#[path = "fresh_vec_fixture.rs"]
pub(crate) mod fresh_vec_fixture;
#[path = "lexical_chained_fixture.rs"]
pub(crate) mod lexical_chained_fixture;

#[derive(Clone, Copy)]
pub(crate) enum Shape {
    Fresh,
    Chained,
}

fn call(f: &mut Builder, name: &str) -> u32 {
    let start = f.source.len();
    let callee = f.name(name);
    let open_paren_span = f.text("(");
    let close_paren_span = f.text(")");
    f.expression(
        start,
        RawExpressionKind::Call {
            callee,
            open_paren_span,
            arguments: Vec::new(),
            close_paren_span,
        },
    )
}

fn index(f: &mut Builder, base: u32, value: Option<i32>, name: &str) -> u32 {
    let start = f.expressions[base as usize].span.start as usize;
    let open_bracket_span = f.text("[");
    let index = if let Some(value) = value {
        let start = f.source.len();
        let spelling = value.to_string();
        f.text(&spelling);
        f.expression(start, RawExpressionKind::I32Literal { spelling })
    } else {
        call(f, name)
    };
    let close_bracket_span = f.text("]");
    f.expression(
        start,
        RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
    )
}

fn access(f: &mut Builder, shape: Shape, indices: [Option<i32>; 2]) -> u32 {
    let base = match shape {
        Shape::Fresh => call(f, "makeArray"),
        Shape::Chained => f.reference("items"),
    };
    let first = index(f, base, indices[0], "firstIndex");
    match shape {
        Shape::Fresh => first,
        Shape::Chained => index(f, first, indices[1], "secondIndex"),
    }
}

fn function(
    f: &mut Builder,
    name: &str,
    parameters: &[(&str, Ty)],
    result: &Ty,
    body: impl FnOnce(&mut Builder) -> u32,
) -> RawFunctionSyntax {
    f.text("\n");
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name(name);
    f.text("(");
    let parameters = parameters
        .iter()
        .enumerate()
        .map(|(i, (name, ty))| {
            if i != 0 {
                f.text(", ");
            }
            f.parameter(name, ty)
        })
        .collect();
    f.text("): ");
    let result_type = f.ty(result);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let value = body(f);
    // Reserve the return spelling before its expression in each body callback.
    let return_start = f.source[..f.expressions[value as usize].span.start as usize]
        .rfind("return ")
        .expect("return keyword");
    let keyword_span = at(return_start, return_start + 6);
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
    let span = at(open_brace_span.start as usize, f.source.len());
    let statements = std::mem::take(&mut f.statements);
    RawFunctionSyntax {
        span: at(start, f.source.len()),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span,
                open_brace_span,
                statements: (0..statements.len())
                    .map(|i| u32::try_from(i).expect("statement"))
                    .collect(),
                close_brace_span,
            }],
            statements,
            expressions: std::mem::take(&mut f.expressions),
        },
    }
}

fn literal(f: &mut Builder, owned: bool) -> u32 {
    let start = f.source.len();
    let spelling = if owned { "\"value\"" } else { "7" }.to_owned();
    f.text(&spelling);
    f.expression(
        start,
        if owned {
            RawExpressionKind::StringLiteral { spelling }
        } else {
            RawExpressionKind::I32Literal { spelling }
        },
    )
}

fn array_value(f: &mut Builder, container: &Ty, length: u32, owned: bool) -> u32 {
    f.text("return ");
    let start = f.source.len();
    let type_syntax = f.ty(container);
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

pub(crate) fn fixture(
    owned: bool,
    shape: Shape,
    replace: bool,
    lengths: [u32; 2],
    indices: [Option<i32>; 2],
) -> (String, RawProjectSyntaxSnapshot) {
    let element = if owned { Element::String } else { Element::I32 };
    let (mut f, declarations, element) = initial(&element);
    let inner = Ty::Array(Box::new(element.clone()), lengths[1]);
    let container = Ty::Array(
        Box::new(match shape {
            Shape::Fresh => element.clone(),
            Shape::Chained => inner,
        }),
        lengths[0],
    );
    let parameters = match shape {
        Shape::Fresh => Vec::new(),
        Shape::Chained => vec![("incoming", container.clone())],
    };
    let mut functions = vec![function(
        &mut f,
        "observe",
        &parameters,
        if replace { &container } else { &element },
        |f| {
            if matches!(shape, Shape::Chained) {
                f.local("items", &container, true, "incoming");
            }
            if replace {
                let start = f.source.len();
                let target = access(f, shape, indices);
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
                if owned {
                    f.clone_value(|f| access(f, shape, indices))
                } else {
                    access(f, shape, indices)
                }
            }
        },
    )];
    if matches!(shape, Shape::Fresh) {
        functions.push(function(&mut f, "makeArray", &[], &container, |f| {
            array_value(f, &container, lengths[0], owned)
        }));
    }
    for name in ["firstIndex", "secondIndex"] {
        functions.push(function(&mut f, name, &[], &Ty::Named("i32"), |f| {
            f.text("return ");
            literal(f, false)
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
