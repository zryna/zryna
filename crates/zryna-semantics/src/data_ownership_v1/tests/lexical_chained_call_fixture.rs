use super::*;

pub(super) fn statement(f: &mut Builder, element: &Ty) -> u32 {
    let start = f.source.len();
    let keyword_span = f.text("const");
    f.text(" ");
    let name = f.name("seen");
    f.text(": ");
    let type_syntax = f.ty(element);
    f.text(" ");
    let equals_span = f.text("=");
    f.text(" ");
    let call_start = f.source.len();
    let callee = f.name("consume");
    let open_paren_span = f.text("(");
    let alias = f.reference("loan");
    f.text(", ");
    let rhs = call(f, "replacement");
    let close_paren_span = f.text(")");
    let initializer = f.expression(
        call_start,
        RawExpressionKind::Call {
            callee,
            open_paren_span,
            arguments: vec![alias, rhs],
            close_paren_span,
        },
    );
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

fn body(f: &mut Builder, owned: bool, exclusive: bool) {
    if exclusive {
        let start = f.source.len();
        let target = f.reference("loan");
        f.text(" ");
        let equals_span = f.text("=");
        f.text(" ");
        let value = f.reference("rhs");
        let semicolon_span = f.text(";");
        f.statements.push(RawStatementSyntax {
            span: at(start, f.source.len()),
            kind: RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
        });
        f.text(" ");
    }
    let start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = if owned { f.clone_value(|f| f.reference("loan")) } else { f.reference("loan") };
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
}

pub(super) fn callee(
    f: &mut Builder,
    element: &Ty,
    owned: bool,
    exclusive: bool,
) -> RawFunctionSyntax {
    f.text("\n");
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("consume");
    f.text("(");
    let parameter_start = f.source.len();
    let parameter_name = f.name("loan");
    f.text(": ");
    let type_syntax = alias_type(f, element, exclusive);
    let first = RawParameterSyntax {
        span: at(parameter_start, f.source.len()),
        name: parameter_name,
        type_syntax,
    };
    f.text(", ");
    let second = f.parameter("rhs", element);
    f.text("): ");
    let result_type = f.ty(element);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    body(f, owned, exclusive);
    f.text(" ");
    let close_brace_span = f.text("}");
    let span = at(open_brace_span.start as usize, f.source.len());
    let statements = std::mem::take(&mut f.statements);
    RawFunctionSyntax {
        span: at(start, f.source.len()),
        export_span: None,
        function_span,
        name,
        parameters: vec![first, second],
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
