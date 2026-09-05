use super::nested_mixed_construction::root_replacement::{
    ReplacementCase, ReplacementRoot, replacement_fixture,
};
use super::*;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawExpressionKind;

#[derive(Clone, Debug)]
pub(in crate::data_ownership_v1) enum Element {
    I32,
    Bool,
    String,
    Struct,
    Enum,
    Array,
    Vec,
}
#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Operation {
    Read,
    Clone,
    Replace,
    ReplaceClone,
    ReplaceSelfClone,
    Push,
    PushClone,
    PushIndexedClone,
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
    statements: Vec<RawStatementSyntax>,
}

fn at(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: start.try_into().expect("start"),
        end: end.try_into().expect("end"),
    }
}

impl Builder {
    fn mutation(&mut self, operation: Operation, index: Option<i32>) {
        let start = self.source.len();
        let kind = if matches!(
            operation,
            Operation::Push | Operation::PushClone | Operation::PushIndexedClone
        ) {
            let keyword_span = self.text("push");
            let open_paren_span = self.text("(");
            let vector = self.reference("items");
            let comma_span = self.text(",");
            self.text(" ");
            let value = if matches!(operation, Operation::PushIndexedClone) {
                self.clone_value(|f| f.indexed(index))
            } else if matches!(operation, Operation::PushClone) {
                self.clone_value(|f| f.reference("next"))
            } else {
                self.reference("next")
            };
            let close_paren_span = self.text(")");
            let expression = self.expression(
                start,
                RawExpressionKind::VecPush {
                    keyword_span,
                    open_paren_span,
                    vector,
                    comma_span,
                    value,
                    close_paren_span,
                },
            );
            RawStatementKind::ExpressionStatement { expression, semicolon_span: self.text(";") }
        } else {
            let target = self.indexed(index);
            self.text(" ");
            let equals_span = self.text("=");
            self.text(" ");
            let value = if matches!(operation, Operation::ReplaceSelfClone) {
                self.clone_value(|f| f.indexed(index))
            } else if matches!(operation, Operation::ReplaceClone) {
                self.clone_value(|f| f.reference("next"))
            } else {
                self.reference("next")
            };
            RawStatementKind::Assignment {
                target,
                equals_span,
                value,
                semicolon_span: self.text(";"),
            }
        };
        self.statements.push(RawStatementSyntax { span: at(start, self.source.len()), kind });
        self.text(" ");
    }
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
            Ty::Vec(element) | Ty::Array(element) => {
                let keyword_span =
                    self.text(if matches!(ty, Ty::Vec(_)) { "Vec" } else { "FixedArray" });
                let less_than_span = self.text("<");
                let argument = self.ty(element);
                if matches!(ty, Ty::Vec(_)) {
                    RawTypeSyntaxKind::Vec {
                        keyword_span,
                        less_than_span,
                        argument,
                        greater_than_span: self.text(">"),
                    }
                } else {
                    let comma_span = self.text(",");
                    self.text(" ");
                    let length_span = self.text("1");
                    RawTypeSyntaxKind::FixedArray {
                        keyword_span,
                        less_than_span,
                        element: argument,
                        comma_span,
                        length_span,
                        length_spelling: "1".into(),
                        length: 1,
                        greater_than_span: self.text(">"),
                    }
                }
            }
        };
        let id = self.types.len().try_into().expect("type");
        self.types.push(RawTypeSyntax { span: at(start, self.source.len()), kind });
        id
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
    fn indexed(&mut self, index: Option<i32>) -> u32 {
        let start = self.source.len();
        let base = self.reference("items");
        let open_bracket_span = self.text("[");
        let index = if let Some(index) = index {
            let start = self.source.len();
            let spelling = index.to_string();
            self.text(&spelling);
            self.expression(start, RawExpressionKind::I32Literal { spelling })
        } else {
            self.reference("index")
        };
        let close_bracket_span = self.text("]");
        self.expression(
            start,
            RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
        )
    }
    fn clone_value(&mut self, value: impl FnOnce(&mut Self) -> u32) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("clone");
        let open_paren_span = self.text("(");
        let value = value(self);
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span },
        )
    }
    fn parameter(&mut self, name: &str, ty: &Ty) -> RawParameterSyntax {
        let start = self.source.len();
        let name = self.name(name);
        self.text(": ");
        let type_syntax = self.ty(ty);
        RawParameterSyntax { span: at(start, self.source.len()), name, type_syntax }
    }
    fn local(&mut self, name: &str, ty: &Ty, mutable: bool, source: &str) {
        let start = self.source.len();
        let keyword_span = self.text(if mutable { "let" } else { "const" });
        self.text(" ");
        let name = self.name(name);
        self.text(": ");
        let type_syntax = self.ty(ty);
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let initializer = self.reference(source);
        let semicolon_span = self.text(";");
        self.statements.push(RawStatementSyntax {
            span: at(start, self.source.len()),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span,
                mutable,
                name,
                type_syntax,
                equals_span,
                initializer,
                semicolon_span,
            },
        });
        self.text(" ");
    }
}

fn initial(element: &Element) -> (Builder, Vec<RawDataDeclaration>, Ty) {
    let root = match element {
        Element::Struct => Some(ReplacementRoot::Struct),
        Element::Enum => Some(ReplacementRoot::Enum),
        _ => None,
    };
    let (source, types, declarations) = if let Some(root) = root {
        let (source, raw) = replacement_fixture(root, ReplacementCase::Constructor);
        let file = &raw.files[0];
        let end = file.data_declarations.last().expect("declaration").span.end;
        (
            source[..end as usize].to_owned(),
            file.type_syntax.iter().filter(|ty| ty.span.end <= end).cloned().collect(),
            file.data_declarations.clone(),
        )
    } else {
        (String::new(), Vec::new(), Vec::new())
    };
    let ty = match element {
        Element::I32 => Ty::Named("i32"),
        Element::Bool => Ty::Named("bool"),
        Element::String => Ty::String,
        Element::Struct => Ty::Named("Parcel"),
        Element::Enum => Ty::Named("Choice"),
        Element::Array => Ty::Array(Box::new(Ty::Vec(Box::new(Ty::String)))),
        Element::Vec => Ty::Vec(Box::new(Ty::Vec(Box::new(Ty::String)))),
    };
    (Builder { source, types, expressions: Vec::new(), statements: Vec::new() }, declarations, ty)
}

pub(in crate::data_ownership_v1) fn fixture(
    element: &Element,
    operation: Operation,
    index: Option<i32>,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations, element_type) = initial(element);
    let vector = Ty::Vec(Box::new(element_type.clone()));
    let replacement = !matches!(operation, Operation::Read | Operation::Clone);
    f.text("\n");
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("observe");
    f.text("(");
    let mut parameters = vec![f.parameter("incoming", &vector)];
    f.text(", ");
    parameters.push(f.parameter("index", &Ty::Named("i32")));
    if replacement {
        f.text(", ");
        parameters.push(f.parameter("replacement", &element_type));
    }
    f.text("): ");
    let result_type = f.ty(if replacement { &vector } else { &element_type });
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    f.local("items", &vector, true, "incoming");
    if replacement {
        f.local("next", &element_type, false, "replacement");
        f.mutation(operation, index);
    }
    let return_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = if replacement {
        f.reference("items")
    } else if matches!(operation, Operation::Clone) {
        f.clone_value(|f| f.indexed(index))
    } else {
        f.indexed(index)
    };
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start as usize, f.source.len());
    let function = RawFunctionSyntax {
        span: at(start, f.source.len()),
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
                statements: (0..f.statements.len())
                    .map(|id| u32::try_from(id).expect("statement"))
                    .collect(),
                close_brace_span,
            }],
            statements: f.statements,
            expressions: f.expressions,
        },
    };
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".into(),
            imports: Vec::new(),
            type_syntax: f.types,
            data_declarations: declarations,
            functions: vec![function],
        }],
        diagnostics: Vec::new(),
    };
    (f.source, raw)
}
