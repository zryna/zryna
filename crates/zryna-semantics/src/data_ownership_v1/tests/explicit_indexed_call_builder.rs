use super::*;

pub(super) fn call(f: &mut Builder<'_>) -> u32 {
    let start = f.position();
    let callee = f.name("access");
    let open_paren_span = f.text("(");
    let first = f.reference("loan");
    f.text(", ");
    let second = f.reference("next");
    let close_paren_span = f.text(")");
    f.expression(
        start,
        RawExpressionKind::Call {
            callee,
            open_paren_span,
            arguments: vec![first, second],
            close_paren_span,
        },
    )
}

fn parameter(f: &mut Builder<'_>, element: u32, exclusive: Option<bool>) -> RawParameterSyntax {
    let start = f.position();
    let name = f.name(if exclusive.is_some() { "loan" } else { "rhs" });
    f.text(": ");
    let type_syntax = if let Some(exclusive) = exclusive {
        let type_start = f.position();
        let keyword_span = f.text(if exclusive { "BorrowMut" } else { "Borrow" });
        let less_than_span = f.text("<");
        let argument = f.ty(element);
        let greater_than_span = f.text(">");
        let kind = if exclusive {
            RawTypeSyntaxKind::BorrowMut {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            }
        } else {
            RawTypeSyntaxKind::Borrow { keyword_span, less_than_span, argument, greater_than_span }
        };
        let id = u32::try_from(f.types.len()).expect("borrow parameter type");
        f.types.push(RawTypeSyntax { span: at(type_start, f.position()), kind });
        id
    } else {
        f.ty(element)
    };
    RawParameterSyntax { span: at(start, f.position()), name, type_syntax }
}

pub(super) fn callee(
    f: &mut Builder<'_>,
    element: u32,
    exclusive: bool,
    copy: bool,
) -> RawFunctionSyntax {
    f.text("\n");
    let start = f.position();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("access");
    f.text("(");
    let first = parameter(f, element, Some(exclusive));
    f.text(", ");
    let second = parameter(f, element, None);
    f.text("): ");
    let result_type = f.ty(element);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let mut statements = Vec::new();
    if exclusive {
        let start = f.position();
        let target = f.reference("loan");
        f.text(" ");
        let equals_span = f.text("=");
        f.text(" ");
        let value = f.reference("rhs");
        let semicolon_span = f.text(";");
        statements.push(f.statement(
            start,
            RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
        ));
    }
    let returned = f.position();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = if copy { f.reference("loan") } else { f.cloned("loan") };
    let semicolon_span = f.text(";");
    statements.push(
        f.statement(returned, RawStatementKind::Return { keyword_span, value, semicolon_span }),
    );
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start, close_brace_span.end);
    f.body.span = body_span;
    f.body.root_block = 0;
    f.body.blocks.push(RawBlockSyntax {
        span: body_span,
        open_brace_span,
        statements,
        close_brace_span,
    });
    RawFunctionSyntax {
        span: at(start, f.position()),
        export_span: None,
        function_span,
        name,
        parameters: vec![first, second],
        result_type,
        body: f.body.clone(),
    }
}
