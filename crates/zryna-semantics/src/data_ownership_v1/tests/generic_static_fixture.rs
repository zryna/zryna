use super::nested_mixed_construction::root_replacement::{
    ReplacementCase, ReplacementRoot, replacement_fixture,
};
use super::*;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawExpressionKind;

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Shape {
    Struct,
    Array,
    ArrayStruct,
    CopyStruct,
    CopyArray,
    CopyNestedArray,
}
#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Case {
    Clone,
    Move,
    Sibling,
    Partial,
    Replace,
    Repeat,
    SelfMove,
    SelfClone,
    CopyReplace,
}
#[derive(Clone, Copy)]
enum Ty {
    Root,
    Element,
    String,
    Vector,
    Parcel,
    Integer,
    CopyArray,
}

struct Builder {
    source: String,
    types: Vec<RawTypeSyntax>,
    expressions: Vec<RawExpressionSyntax>,
    statements: Vec<RawStatementSyntax>,
    shape: Shape,
}

fn at(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: start.try_into().expect("start"),
        end: end.try_into().expect("end"),
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
    fn ty(&mut self, ty: Ty) -> u32 {
        let start = self.source.len();
        let kind = match ty {
            Ty::Root if matches!(self.shape, Shape::Struct | Shape::CopyStruct) => {
                return self.ty(Ty::Parcel);
            }
            Ty::Element => {
                return self.ty(match self.shape {
                    Shape::ArrayStruct => Ty::Parcel,
                    Shape::CopyStruct | Shape::CopyArray => Ty::Integer,
                    Shape::CopyNestedArray => Ty::CopyArray,
                    _ => Ty::Vector,
                });
            }
            Ty::Integer => RawTypeSyntaxKind::Named { name: self.name("i32") },
            Ty::String => RawTypeSyntaxKind::String { keyword_span: self.text("String") },
            Ty::Parcel => RawTypeSyntaxKind::Named { name: self.name("Parcel") },
            Ty::Vector => {
                let keyword_span = self.text("Vec");
                let less_than_span = self.text("<");
                let argument = self.ty(Ty::String);
                RawTypeSyntaxKind::Vec {
                    keyword_span,
                    less_than_span,
                    argument,
                    greater_than_span: self.text(">"),
                }
            }
            Ty::Root | Ty::CopyArray => {
                let keyword_span = self.text("FixedArray");
                let less_than_span = self.text("<");
                let element =
                    self.ty(if matches!(ty, Ty::CopyArray) { Ty::Integer } else { Ty::Element });
                let comma_span = self.text(",");
                self.text(" ");
                let length_span = self.text("2");
                RawTypeSyntaxKind::FixedArray {
                    keyword_span,
                    less_than_span,
                    element,
                    comma_span,
                    length_span,
                    length_spelling: "2".into(),
                    length: 2,
                    greater_than_span: self.text(">"),
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
    fn reference(&mut self, name: &str) -> u32 {
        let start = self.source.len();
        let name = self.name(name);
        self.expression(start, RawExpressionKind::Reference { name })
    }
    fn projection(&mut self, sibling: bool) -> u32 {
        let start = self.source.len();
        let base = self.reference("item");
        let kind = if matches!(self.shape, Shape::Struct | Shape::CopyStruct) {
            let dot_span = self.text(".");
            RawExpressionKind::FieldAccess { base, dot_span, field: self.name("value") }
        } else {
            let open_bracket_span = self.text("[");
            let index_start = self.source.len();
            let spelling = if sibling { "1" } else { "0" };
            self.text(spelling);
            let index = self.expression(
                index_start,
                RawExpressionKind::I32Literal { spelling: spelling.into() },
            );
            RawExpressionKind::Index {
                base,
                open_bracket_span,
                index,
                close_bracket_span: self.text("]"),
            }
        };
        self.expression(start, kind)
    }
    fn clone_value(&mut self, source: impl FnOnce(&mut Self) -> u32) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("clone");
        let open_paren_span = self.text("(");
        let value = source(self);
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span },
        )
    }
    fn parameter(&mut self, name: &str, ty: Ty) -> RawParameterSyntax {
        let start = self.source.len();
        let name = self.name(name);
        self.text(": ");
        let type_syntax = self.ty(ty);
        RawParameterSyntax { span: at(start, self.source.len()), name, type_syntax }
    }
    fn local(&mut self, name: &str, ty: Ty, value: impl FnOnce(&mut Self) -> u32) {
        let start = self.source.len();
        let mutable = name == "item";
        let keyword_span = self.text(if mutable { "let" } else { "const" });
        self.text(" ");
        let name = self.name(name);
        self.text(": ");
        let type_syntax = self.ty(ty);
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let initializer = value(self);
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
    fn replace(&mut self, case: Case) {
        let start = self.source.len();
        let target = self.projection(false);
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let value = match case {
            Case::CopyReplace => self.reference("next"),
            Case::SelfMove => self.projection(false),
            Case::SelfClone => self.clone_value(|f| f.projection(false)),
            _ => self.clone_value(|f| f.reference("next")),
        };
        let semicolon_span = self.text(";");
        self.statements.push(RawStatementSyntax {
            span: at(start, self.source.len()),
            kind: RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
        });
        self.text(" ");
    }
    fn returned(&mut self, case: Case) {
        let start = self.source.len();
        let keyword_span = self.text("return");
        self.text(" ");
        let value = match case {
            Case::Clone => self.clone_value(|f| f.projection(false)),
            Case::Move => self.projection(false),
            Case::Sibling => self.clone_value(|f| f.projection(true)),
            Case::Partial => self.clone_value(|f| f.reference("item")),
            _ => self.reference("item"),
        };
        let semicolon_span = self.text(";");
        self.statements.push(RawStatementSyntax {
            span: at(start, self.source.len()),
            kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
        });
        self.text(" ");
    }
}

fn initial(shape: Shape) -> (Builder, Vec<RawDataDeclaration>) {
    if !matches!(shape, Shape::CopyStruct | Shape::CopyArray | Shape::CopyNestedArray) {
        let (source, raw) =
            replacement_fixture(ReplacementRoot::Struct, ReplacementCase::Constructor);
        let file = &raw.files[0];
        let end = file.data_declarations[0].span.end as usize;
        return (
            Builder {
                source: source[..end].into(),
                types: file.type_syntax[..2].to_vec(),
                expressions: Vec::new(),
                statements: Vec::new(),
                shape,
            },
            file.data_declarations.clone(),
        );
    }
    let mut f = Builder {
        source: String::new(),
        types: Vec::new(),
        expressions: Vec::new(),
        statements: Vec::new(),
        shape,
    };
    let interface_span = f.text("interface");
    f.text(" ");
    let name = f.name("Parcel");
    f.text(" ");
    let extends_span = f.text("extends");
    f.text(" ");
    let marker_span = f.text("ZrynaStruct");
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let start = f.source.len();
    let field_name = f.name("value");
    let colon_span = f.text(":");
    f.text(" ");
    let type_syntax = f.ty(Ty::Integer);
    let semicolon_span = f.text(";");
    let fields = vec![RawDataField {
        span: at(start, f.source.len()),
        name: field_name,
        colon_span,
        semicolon_span,
        type_syntax,
    }];
    f.text(" ");
    let close_brace_span = f.text("}");
    let declaration = RawDataDeclaration {
        span: at(0, f.source.len()),
        export_span: None,
        kind: RawDataDeclarationKind::Struct {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            close_brace_span,
            fields,
        },
    };
    (f, vec![declaration])
}

pub(in crate::data_ownership_v1) fn fixture(
    shape: Shape,
    case: Case,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations) = initial(shape);
    f.text("\n");
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("project");
    f.text("(");
    let mut parameters = vec![f.parameter("incoming", Ty::Root)];
    let replacement = matches!(
        case,
        Case::Replace | Case::Repeat | Case::SelfMove | Case::SelfClone | Case::CopyReplace
    );
    if replacement {
        f.text(", ");
        parameters.push(f.parameter("next", Ty::Element));
    }
    if matches!(shape, Shape::CopyStruct | Shape::CopyArray | Shape::CopyNestedArray) {
        f.text(", ");
        parameters.push(f.parameter("owned", Ty::String));
    }
    f.text("): ");
    let result_type =
        f.ty(if replacement || matches!(case, Case::Partial) { Ty::Root } else { Ty::Element });
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    f.local("item", Ty::Root, |f| f.reference("incoming"));
    if matches!(case, Case::Sibling | Case::Partial) {
        f.local("taken", Ty::Element, |f| f.projection(false));
    }
    if replacement {
        f.replace(case);
        if matches!(case, Case::Repeat) {
            f.replace(case);
        }
    }
    f.returned(case);
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
                    .map(|i| u32::try_from(i).expect("statement"))
                    .collect(),
                close_brace_span,
            }],
            statements: f.statements,
            expressions: f.expressions,
        },
    };
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
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}
