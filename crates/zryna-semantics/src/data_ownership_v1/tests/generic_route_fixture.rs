use super::super::*;
use super::{Builder, Ty, at};
use zryna_syntax::v4::RawExpressionKind;

impl Builder {
    fn literal(&mut self, string: bool) -> u32 {
        let start = self.source.len();
        let spelling = if string { "\"text\"" } else { "0" };
        self.text(spelling);
        self.expression(
            start,
            if string {
                RawExpressionKind::StringLiteral { spelling: spelling.into() }
            } else {
                RawExpressionKind::I32Literal { spelling: spelling.into() }
            },
        )
    }

    fn construction(&mut self, ty: &Ty) -> u32 {
        if matches!(ty, Ty::String) {
            return self.literal(true);
        }
        let (Ty::Vec(element) | Ty::Array(element)) = ty else { panic!("container") };
        let start = self.source.len();
        let type_syntax = self.ty(ty);
        let open_paren_span = self.text("(");
        let open_bracket_span = self.text("[");
        let value = self.construction(element);
        let close_bracket_span = self.text("]");
        let close_paren_span = self.text(")");
        self.expression(
            start,
            if matches!(ty, Ty::Array(_)) {
                RawExpressionKind::FixedArrayConstruction {
                    type_syntax,
                    open_paren_span,
                    open_bracket_span,
                    elements: vec![value],
                    close_bracket_span,
                    close_paren_span,
                }
            } else {
                RawExpressionKind::VecConstruction {
                    type_syntax,
                    open_paren_span,
                    open_bracket_span,
                    elements: vec![value],
                    close_bracket_span,
                    close_paren_span,
                }
            },
        )
    }

    fn mixed_local(&mut self) -> RawStatementSyntax {
        let ty = Ty::Array(Box::new(Ty::Vec(Box::new(Ty::String))));
        let start = self.source.len();
        let keyword_span = self.text("const");
        self.text(" ");
        let name = self.name("items");
        self.text(": ");
        let type_syntax = self.ty(&ty);
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let initializer = self.construction(&ty);
        let semicolon_span = self.text(";");
        let statement = RawStatementSyntax {
            span: at(start, self.source.len()),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span,
                mutable: false,
                name,
                type_syntax,
                equals_span,
                initializer,
                semicolon_span,
            },
        };
        self.text(" ");
        statement
    }

    fn inline_call(&mut self, ty: &Ty) -> u32 {
        let start = self.source.len();
        let callee = self.name("consume");
        let open_paren_span = self.text("(");
        let first = self.construction(ty);
        let mut arguments = vec![first];
        if !matches!(ty, Ty::String) {
            self.text(", ");
            arguments.push(self.literal(true));
        }
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::Call { callee, open_paren_span, arguments, close_paren_span },
        )
    }

    fn route_function(
        &mut self,
        local: bool,
        string: bool,
        callee: bool,
        ty: &Ty,
    ) -> RawFunctionSyntax {
        self.text("\n");
        let start = self.source.len();
        let function_span = self.text("function");
        self.text(" ");
        let name = self.name(if callee { "consume" } else { "caller" });
        self.text("(");
        let parameters = if callee {
            let mut parameters = vec![self.parameter("items", ty)];
            if !matches!(ty, Ty::String) {
                self.text(", ");
                parameters.push(self.parameter("text", &Ty::String));
            }
            parameters
        } else {
            Vec::new()
        };
        self.text("): ");
        let result_type = self.ty(&if string { Ty::String } else { Ty::Named("i32") });
        self.text(" ");
        let open_brace_span = self.text("{");
        self.text(" ");
        let mut statements = Vec::new();
        if local {
            statements.push(self.mixed_local());
        }
        let return_start = self.source.len();
        let keyword_span = self.text("return");
        self.text(" ");
        let value = if local || callee { self.literal(string) } else { self.inline_call(ty) };
        let semicolon_span = self.text(";");
        statements.push(RawStatementSyntax {
            span: at(return_start, self.source.len()),
            kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
        });
        self.text(" ");
        let close_brace_span = self.text("}");
        let body_span = at(open_brace_span.start as usize, self.source.len());
        RawFunctionSyntax {
            span: at(start, self.source.len()),
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
                    statements: (0..statements.len())
                        .map(|id| u32::try_from(id).expect("statement"))
                        .collect(),
                    close_brace_span,
                }],
                statements,
                expressions: std::mem::take(&mut self.expressions),
            },
        }
    }
}

pub(in super::super) fn fixture(local: bool, string: bool) -> (String, RawProjectSyntaxSnapshot) {
    fixture_for(local, string, &Ty::Vec(Box::new(Ty::Vec(Box::new(Ty::String)))))
}

pub(in super::super) fn single_string_fixture() -> (String, RawProjectSyntaxSnapshot) {
    fixture_for(false, false, &Ty::String)
}

fn fixture_for(local: bool, string: bool, ty: &Ty) -> (String, RawProjectSyntaxSnapshot) {
    let mut f = Builder { source: String::new(), types: Vec::new(), expressions: Vec::new() };
    let mut functions = vec![f.route_function(local, string, false, ty)];
    if !local {
        functions.push(f.route_function(false, string, true, ty));
    }
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".into(),
            imports: Vec::new(),
            type_syntax: f.types,
            data_declarations: Vec::new(),
            functions,
        }],
        diagnostics: Vec::new(),
    };
    (f.source, raw)
}
