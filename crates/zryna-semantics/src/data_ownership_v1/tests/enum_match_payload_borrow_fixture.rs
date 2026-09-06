use super::*;
use zryna_syntax::v4::RawMatchArm;

#[path = "enum_match_payload_borrow_fixture/calls.rs"]
mod calls;

#[derive(Clone, Copy)]
pub(super) enum MatchBorrowCase {
    Shared,
    Exclusive,
    Inactive,
    CallShared,
    CallExclusive,
    CallInactive,
    CallWrongAccess,
}

impl MatchBorrowCase {
    fn is_call(self) -> bool {
        matches!(
            self,
            Self::CallShared | Self::CallExclusive | Self::CallInactive | Self::CallWrongAccess
        )
    }

    fn borrow_exclusive(self) -> bool {
        matches!(self, Self::Exclusive | Self::CallExclusive)
    }

    fn callee_exclusive(self) -> bool {
        matches!(self, Self::CallExclusive | Self::CallWrongAccess)
    }
}

impl Builder {
    fn inline_borrow(&mut self, target: impl FnOnce(&mut Self) -> u32, exclusive: bool) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text(if exclusive { "borrowMut" } else { "borrow" });
        let open_paren_span = self.text("(");
        let value = target(self);
        let close_paren_span = self.text(")");
        self.expression(
            start,
            if exclusive {
                RawExpressionKind::BorrowMut {
                    keyword_span,
                    open_paren_span,
                    value,
                    close_paren_span,
                }
            } else {
                RawExpressionKind::Borrow { keyword_span, open_paren_span, value, close_paren_span }
            },
        )
    }

    fn inline_borrow_clone(
        &mut self,
        target: impl FnOnce(&mut Self) -> u32,
        exclusive: bool,
    ) -> u32 {
        self.clone_value(|builder| builder.inline_borrow(target, exclusive))
    }

    fn inline_borrow_call(
        &mut self,
        target: impl FnOnce(&mut Self) -> u32,
        exclusive: bool,
    ) -> u32 {
        let start = self.source.len();
        let callee = self.name("relay");
        let open_paren_span = self.text("(");
        let argument = self.inline_borrow(target, exclusive);
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::Call {
                callee,
                open_paren_span,
                arguments: vec![argument],
                close_paren_span,
            },
        )
    }

    fn source_variant(&mut self, variant: &str) -> u32 {
        let start = self.source.len();
        let base = self.reference("source");
        let dot_span = self.text(".");
        let field = self.name(variant);
        self.expression(start, RawExpressionKind::FieldAccess { base, dot_span, field })
    }
}

fn match_expression(f: &mut Builder, case: MatchBorrowCase) -> u32 {
    let match_start = f.source.len();
    let keyword_span = f.text("match");
    let open_paren_span = f.text("(");
    let scrutinee = f.reference("source");
    f.text(", ");
    let open_brace_span = f.text("{");

    f.text(" ");
    let none_start = f.source.len();
    f.text("\"");
    let none_type = f.name("Choice");
    let none_dot = f.text(".");
    let none_variant = f.name("none");
    f.text("\": () ");
    let none_arrow = f.text("=>");
    f.text(" ");
    let none_value = if matches!(case, MatchBorrowCase::CallInactive) {
        f.inline_borrow_call(|builder| builder.source_variant("some"), false)
    } else if matches!(case, MatchBorrowCase::Inactive) {
        f.inline_borrow_clone(|builder| builder.source_variant("some"), false)
    } else {
        f.clone_value(|builder| builder.reference("fallback"))
    };
    let none_arm = RawMatchArm {
        span: at(none_start, f.source.len()),
        type_name: none_type,
        dot_span: none_dot,
        variant: none_variant,
        binding: None,
        arrow_span: none_arrow,
        value: none_value,
    };
    f.text(", ");
    let some_start = f.source.len();
    f.text("\"");
    let some_type = f.name("Choice");
    let some_dot = f.text(".");
    let some_variant = f.name("some");
    f.text("\": (");
    let binding = Some(f.name("payload"));
    f.text(") ");
    let some_arrow = f.text("=>");
    f.text(" ");
    let exclusive = case.borrow_exclusive();
    let some_value = if case.is_call() {
        f.inline_borrow_call(|builder| builder.reference("payload"), exclusive)
    } else {
        f.inline_borrow_clone(|builder| builder.reference("payload"), exclusive)
    };
    let some_arm = RawMatchArm {
        span: at(some_start, f.source.len()),
        type_name: some_type,
        dot_span: some_dot,
        variant: some_variant,
        binding,
        arrow_span: some_arrow,
        value: some_value,
    };
    f.text(" }");
    let close_brace_span = at(f.source.len() - 1, f.source.len());
    let close_paren_span = f.text(")");
    f.expression(
        match_start,
        RawExpressionKind::Match {
            keyword_span,
            open_paren_span,
            scrutinee,
            close_paren_span,
            open_brace_span,
            arms: vec![none_arm, some_arm],
            close_brace_span,
        },
    )
}

pub(super) fn match_borrow_fixture(case: MatchBorrowCase) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations, choice) = initial(&Element::Enum);
    let payload = Ty::Vec(Box::new(Ty::String));
    f.text("\n");
    let function_start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("observe");
    f.text("(");
    let mut parameters = vec![f.parameter("source", &choice)];
    f.text(", ");
    parameters.push(f.parameter("fallback", &payload));
    f.text("): ");
    let result_type = f.ty(&payload);
    f.text(" ");
    let body_start = f.source.len();
    let open_brace_span = f.text("{");
    f.text(" ");
    let return_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = match_expression(&mut f, case);
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
    let body_span = at(body_start, f.source.len());
    let function = RawFunctionSyntax {
        span: at(function_start, f.source.len()),
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
                statements: vec![0],
                close_brace_span,
            }],
            statements: std::mem::take(&mut f.statements),
            expressions: std::mem::take(&mut f.expressions),
        },
    };
    let mut functions = vec![function];
    if case.is_call() {
        functions.push(calls::callee(&mut f, &payload, case.callee_exclusive()));
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
