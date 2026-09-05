use super::*;

#[path = "lexical_chained_call_fixture.rs"]
mod calls;

#[derive(Clone, Copy)]
pub(crate) enum Case {
    Basic,
    Pair,
    Call,
}

fn alias_type(f: &mut Builder, element: &Ty, exclusive: bool) -> u32 {
    let start = f.source.len();
    let keyword_span = f.text(if exclusive { "BorrowMut" } else { "Borrow" });
    let less_than_span = f.text("<");
    let argument = f.ty(element);
    let greater_than_span = f.text(">");
    let kind = if exclusive {
        RawTypeSyntaxKind::BorrowMut { keyword_span, less_than_span, argument, greater_than_span }
    } else {
        RawTypeSyntaxKind::Borrow { keyword_span, less_than_span, argument, greater_than_span }
    };
    let id = u32::try_from(f.types.len()).expect("type");
    f.types.push(RawTypeSyntax { span: at(start, f.source.len()), kind });
    id
}

fn alias(
    f: &mut Builder,
    element: &Ty,
    exclusive: bool,
    indices: [Option<i32>; 2],
    name: &str,
) -> u32 {
    let start = f.source.len();
    let keyword_span = f.text("const");
    f.text(" ");
    let name = f.name(name);
    f.text(": ");
    let type_syntax = alias_type(f, element, exclusive);
    f.text(" ");
    let equals_span = f.text("=");
    f.text(" ");
    let borrow_start = f.source.len();
    let borrow_keyword = f.text(if exclusive { "borrowMut" } else { "borrow" });
    let open_paren_span = f.text("(");
    let base = f.reference("items");
    let outer = index(f, base, indices[0], "firstIndex");
    let value = index(f, outer, indices[1], "secondIndex");
    let close_paren_span = f.text(")");
    let kind = if exclusive {
        RawExpressionKind::BorrowMut {
            keyword_span: borrow_keyword,
            open_paren_span,
            value,
            close_paren_span,
        }
    } else {
        RawExpressionKind::Borrow {
            keyword_span: borrow_keyword,
            open_paren_span,
            value,
            close_paren_span,
        }
    };
    let initializer = f.expression(borrow_start, kind);
    let semicolon_span = f.text(";");
    let id = u32::try_from(f.statements.len()).expect("statement");
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
    id
}

fn operation(f: &mut Builder, element: &Ty, owned: bool, exclusive: bool) -> u32 {
    let start = f.source.len();
    let kind = if exclusive {
        let target = f.reference("loan");
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
            if owned { f.clone_value(|f| f.reference("loan")) } else { f.reference("loan") };
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
    vector: bool,
    owned: bool,
    exclusive: bool,
    indices: [Option<i32>; 2],
) -> (String, RawProjectSyntaxSnapshot) {
    composed_fixture(vector, owned, exclusive, indices, Case::Basic)
}

pub(crate) fn composed_fixture(
    vector: bool,
    owned: bool,
    exclusive: bool,
    indices: [Option<i32>; 2],
    case: Case,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations, element) =
        initial(&if owned { Element::String } else { Element::I32 });
    let inner = Ty::Array(Box::new(element.clone()), 2);
    let container = if vector { Ty::Vec(Box::new(inner)) } else { Ty::Array(Box::new(inner), 2) };
    let mut nested = None;
    let mut caller =
        function(&mut f, "observe", &[("incoming", container.clone())], &container, |f| {
            f.local("items", &container, true, "incoming");
            let open_brace_span = f.text("{");
            f.text(" ");
            let block_statement = f.statements.len();
            f.statements.push(RawStatementSyntax {
                span: open_brace_span,
                kind: RawStatementKind::Block { block: 1 },
            });
            let first = alias(f, &element, exclusive, indices, "loan");
            let second = match case {
                Case::Basic => operation(f, &element, owned, exclusive),
                Case::Pair => alias(f, &element, exclusive, [Some(1), None], "other"),
                Case::Call => calls::statement(f, &element),
            };
            let close_brace_span = f.text("}");
            let span = at(open_brace_span.start as usize, close_brace_span.end as usize);
            f.statements[block_statement].span = span;
            nested = Some(RawBlockSyntax {
                span,
                open_brace_span,
                statements: vec![first, second],
                close_brace_span,
            });
            f.text(" return ");
            f.reference("items")
        });
    caller.body.blocks[0].statements =
        vec![0, 1, u32::try_from(caller.body.statements.len() - 1).expect("return")];
    caller.body.blocks.push(nested.expect("nested block"));
    let mut functions = vec![caller];
    for name in ["firstIndex", "secondIndex"] {
        functions.push(function(&mut f, name, &[], &Ty::Named("i32"), |f| {
            f.text("return ");
            literal(f, false)
        }));
    }
    if exclusive || matches!(case, Case::Call) {
        functions.push(function(&mut f, "replacement", &[], &element, |f| {
            f.text("return ");
            literal(f, owned)
        }));
    }
    if matches!(case, Case::Call) {
        functions.push(calls::callee(&mut f, &element, owned, exclusive));
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
