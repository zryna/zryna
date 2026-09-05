use super::generic_vec_fixture::{Element, Operation, fixture as vector_fixture};
use super::*;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawExpressionKind;

#[path = "generic_route_fixture.rs"]
mod route;
pub(super) use route::fixture as route_fixture;
pub(super) use route::single_string_fixture;

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Direct,
    Nested,
    WrongType,
    RepeatedOwner,
}

#[derive(Clone)]
enum Ty {
    Named(&'static str),
    String,
    Vec(Box<Self>),
    Array(Box<Self>),
}

struct Builder {
    source: String,
    types: Vec<RawTypeSyntax>,
    expressions: Vec<RawExpressionSyntax>,
}

fn at(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: start.try_into().expect("fixture offset"),
        end: end.try_into().expect("fixture offset"),
    }
}

impl Builder {
    fn text(&mut self, text: &str) -> UntrustedSpan {
        let start = self.source.len();
        self.source.push_str(text);
        at(start, self.source.len())
    }

    fn name(&mut self, text: &str) -> RawIdentifierSyntax {
        RawIdentifierSyntax { text: text.into(), span: self.text(text) }
    }

    fn ty(&mut self, ty: &Ty) -> u32 {
        let start = self.source.len();
        let kind = match ty {
            Ty::Named(name) => RawTypeSyntaxKind::Named { name: self.name(name) },
            Ty::String => RawTypeSyntaxKind::String { keyword_span: self.text("String") },
            Ty::Vec(element) => RawTypeSyntaxKind::Vec {
                keyword_span: self.text("Vec"),
                less_than_span: self.text("<"),
                argument: self.ty(element),
                greater_than_span: self.text(">"),
            },
            Ty::Array(element) => RawTypeSyntaxKind::FixedArray {
                keyword_span: self.text("FixedArray"),
                less_than_span: self.text("<"),
                element: self.ty(element),
                comma_span: self.text(","),
                length_span: self.text("1"),
                length_spelling: "1".into(),
                length: 1,
                greater_than_span: self.text(">"),
            },
        };
        let id = self.types.len().try_into().expect("type");
        self.types.push(RawTypeSyntax { span: at(start, self.source.len()), kind });
        id
    }

    fn parameter(&mut self, name: &str, ty: &Ty) -> RawParameterSyntax {
        let start = self.source.len();
        let name = self.name(name);
        self.text(": ");
        let type_syntax = self.ty(ty);
        RawParameterSyntax { span: at(start, self.source.len()), name, type_syntax }
    }

    fn expression(&mut self, start: usize, kind: RawExpressionKind) -> u32 {
        let id = self.expressions.len().try_into().expect("expression");
        self.expressions.push(RawExpressionSyntax { span: at(start, self.source.len()), kind });
        id
    }

    fn reference(&mut self, text: &str) -> u32 {
        let start = self.source.len();
        let name = self.name(text);
        self.expression(start, RawExpressionKind::Reference { name })
    }

    fn call(&mut self, case: Case) -> u32 {
        let start = self.source.len();
        let callee = self.name("choose");
        let open_paren_span = self.text("(");
        let first = self.reference(if matches!(case, Case::WrongType) { "count" } else { "left" });
        self.text(", ");
        let second = self.reference("count");
        self.text(", ");
        let third =
            self.reference(if matches!(case, Case::RepeatedOwner) { "left" } else { "right" });
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::Call {
                callee,
                open_paren_span,
                arguments: vec![first, second, third],
                close_paren_span,
            },
        )
    }

    fn returned_value(&mut self, ty: &Ty, case: Case, caller: bool) -> u32 {
        if !caller {
            return self.reference("left");
        }
        if !matches!(case, Case::Nested) {
            return self.call(case);
        }
        let start = self.source.len();
        let type_syntax = self.ty(&Ty::Vec(Box::new(ty.clone())));
        let open_paren_span = self.text("(");
        let open_bracket_span = self.text("[");
        let value = self.call(case);
        let close_bracket_span = self.text("]");
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::VecConstruction {
                type_syntax,
                open_paren_span,
                open_bracket_span,
                elements: vec![value],
                close_bracket_span,
                close_paren_span,
            },
        )
    }

    fn function(&mut self, ty: &Ty, case: Case, caller: bool) -> RawFunctionSyntax {
        self.expressions.clear();
        self.text("\n");
        let start = self.source.len();
        let function_span = self.text("function");
        self.text(" ");
        let name = self.name(if caller { "caller" } else { "choose" });
        self.text("(");
        let mut parameters = vec![self.parameter("left", ty)];
        self.text(", ");
        parameters.push(self.parameter("count", &Ty::Named("i32")));
        self.text(", ");
        parameters.push(self.parameter("right", ty));
        if caller {
            self.text(", ");
            parameters.push(self.parameter("keep", ty));
        }
        self.text("): ");
        let result_type = self.ty(&if caller && matches!(case, Case::Nested) {
            Ty::Vec(Box::new(ty.clone()))
        } else {
            ty.clone()
        });
        self.text(" ");
        let open_brace_span = self.text("{");
        self.text(" ");
        let return_start = self.source.len();
        let keyword_span = self.text("return");
        self.text(" ");
        let value = self.returned_value(ty, case, caller);
        let semicolon_span = self.text(";");
        let statement = RawStatementSyntax {
            span: at(return_start, self.source.len()),
            kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
        };
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
                    statements: vec![0],
                    close_brace_span,
                }],
                statements: vec![statement],
                expressions: std::mem::take(&mut self.expressions),
            },
        }
    }
}

pub(super) fn fixture(element: &Element, case: Case) -> (String, RawProjectSyntaxSnapshot) {
    let (source, mut raw) = vector_fixture(element, Operation::Clone, None);
    let file = &mut raw.files[0];
    let end = file.data_declarations.last().map_or(0, |declaration| declaration.span.end);
    let ty = match element {
        Element::Struct => Ty::Named("Parcel"),
        Element::Enum => Ty::Named("Choice"),
        Element::String => Ty::String,
        Element::Vec => Ty::Vec(Box::new(Ty::String)),
        _ => panic!("owned call category"),
    };
    let mut builder = Builder {
        source: source[..end as usize].to_owned(),
        types: file.type_syntax.iter().filter(|ty| ty.span.end <= end).cloned().collect(),
        expressions: Vec::new(),
    };
    file.functions = vec![builder.function(&ty, case, true), builder.function(&ty, case, false)];
    file.type_syntax = builder.types;
    (builder.source, raw)
}
