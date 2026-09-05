use super::*;
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializer, RawFieldInitializerKind};

pub(super) fn at(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: start.try_into().expect("offset"),
        end: end.try_into().expect("offset"),
    }
}

// Emit fixed fixture spellings together with their syntax spans; no semantic inference.
pub(super) struct Fixture {
    pub(super) source: String,
    pub(super) types: Vec<RawTypeSyntax>,
    pub(super) expressions: Vec<RawExpressionSyntax>,
    pub(super) statements: Vec<RawStatementSyntax>,
}

impl Fixture {
    pub(super) fn text(&mut self, text: &str) -> UntrustedSpan {
        let start = self.source.len();
        self.source.push_str(text);
        at(start, self.source.len())
    }

    pub(super) fn name(&mut self, text: &str) -> RawIdentifierSyntax {
        RawIdentifierSyntax { text: text.into(), span: self.text(text) }
    }

    pub(super) fn ty(&mut self, root: Option<ReplacementRoot>) -> u32 {
        let start = self.source.len();
        let kind = match root {
            None => RawTypeSyntaxKind::String { keyword_span: self.text("String") },
            Some(ReplacementRoot::Struct | ReplacementRoot::Enum) => RawTypeSyntaxKind::Named {
                name: self.name(if matches!(root, Some(ReplacementRoot::Struct)) {
                    "Parcel"
                } else {
                    "Choice"
                }),
            },
            Some(ReplacementRoot::Vec | ReplacementRoot::Array) => {
                let array = matches!(root, Some(ReplacementRoot::Array));
                let keyword_span = self.text(if array { "FixedArray" } else { "Vec" });
                let less_than_span = self.text("<");
                let argument = self.payload_type();
                if array {
                    let comma_span = self.text(",");
                    self.text(" ");
                    let length_span = self.text("1");
                    let greater_than_span = self.text(">");
                    RawTypeSyntaxKind::FixedArray {
                        keyword_span,
                        less_than_span,
                        element: argument,
                        comma_span,
                        length_span,
                        length: 1,
                        length_spelling: "1".into(),
                        greater_than_span,
                    }
                } else {
                    RawTypeSyntaxKind::Vec {
                        keyword_span,
                        less_than_span,
                        argument,
                        greater_than_span: self.text(">"),
                    }
                }
            }
        };
        let id = self.types.len().try_into().expect("type");
        self.types.push(RawTypeSyntax { span: at(start, self.source.len()), kind });
        id
    }

    fn payload_type(&mut self) -> u32 {
        let start = self.source.len();
        let keyword_span = self.text("Vec");
        let less_than_span = self.text("<");
        let argument = self.ty(None);
        let greater_than_span = self.text(">");
        let id = self.types.len().try_into().expect("type");
        self.types.push(RawTypeSyntax {
            span: at(start, self.source.len()),
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            },
        });
        id
    }

    fn expression(&mut self, start: usize, kind: RawExpressionKind) -> u32 {
        let id = self.expressions.len().try_into().expect("expression");
        self.expressions.push(RawExpressionSyntax { span: at(start, self.source.len()), kind });
        id
    }

    pub(super) fn reference(&mut self, text: &str) -> u32 {
        let start = self.source.len();
        let name = self.name(text);
        self.expression(start, RawExpressionKind::Reference { name })
    }

    fn payload(&mut self, invalid_later: bool) -> u32 {
        let start = self.source.len();
        let type_syntax = self.payload_type();
        let open_paren_span = self.text("(");
        let open_bracket_span = self.text("[");
        let literal = self.source.len();
        self.text("\"a\"");
        let mut elements =
            vec![self.expression(
                literal,
                RawExpressionKind::StringLiteral { spelling: "\"a\"".into() },
            )];
        if invalid_later {
            self.text(", ");
            elements.push(self.reference("lost"));
        }
        let close_bracket_span = self.text("]");
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::VecConstruction {
                type_syntax,
                open_paren_span,
                open_bracket_span,
                elements,
                close_bracket_span,
                close_paren_span,
            },
        )
    }

    fn constructor(&mut self, root: ReplacementRoot, invalid_later: bool, empty_enum: bool) -> u32 {
        let start = self.source.len();
        match root {
            ReplacementRoot::Struct => {
                let type_name = self.name("Parcel");
                let open_paren_span = self.text("(");
                let open_brace_span = self.text("{");
                self.text(" ");
                let field_start = self.source.len();
                let name = self.name("value");
                let colon_span = self.text(":");
                self.text(" ");
                let value = self.payload(invalid_later);
                let fields = vec![RawFieldInitializer {
                    span: at(field_start, self.source.len()),
                    kind: RawFieldInitializerKind::Explicit { name, colon_span, value },
                }];
                self.text(" ");
                let close_brace_span = self.text("}");
                let close_paren_span = self.text(")");
                self.expression(
                    start,
                    RawExpressionKind::StructConstruction {
                        type_name,
                        open_paren_span,
                        open_brace_span,
                        fields,
                        close_brace_span,
                        close_paren_span,
                    },
                )
            }
            ReplacementRoot::Enum => {
                let type_name = self.name("Choice");
                let dot_span = self.text(".");
                let variant = self.name(if empty_enum { "none" } else { "some" });
                let open_paren_span = self.text("(");
                let payload = if empty_enum { None } else { Some(self.payload(invalid_later)) };
                let close_paren_span = self.text(")");
                self.expression(
                    start,
                    RawExpressionKind::EnumConstruction {
                        type_name,
                        dot_span,
                        variant,
                        open_paren_span,
                        payload,
                        close_paren_span,
                    },
                )
            }
            ReplacementRoot::Array | ReplacementRoot::Vec => {
                let type_syntax = self.ty(Some(root));
                let open_paren_span = self.text("(");
                let open_bracket_span = self.text("[");
                let elements = vec![self.payload(invalid_later)];
                let close_bracket_span = self.text("]");
                let close_paren_span = self.text(")");
                let kind = if matches!(root, ReplacementRoot::Array) {
                    RawExpressionKind::FixedArrayConstruction {
                        type_syntax,
                        open_paren_span,
                        open_bracket_span,
                        elements,
                        close_bracket_span,
                        close_paren_span,
                    }
                } else {
                    RawExpressionKind::VecConstruction {
                        type_syntax,
                        open_paren_span,
                        open_bracket_span,
                        elements,
                        close_bracket_span,
                        close_paren_span,
                    }
                };
                self.expression(start, kind)
            }
        }
    }

    pub(super) fn local(&mut self, root: ReplacementRoot, name: &str, mutable: bool, moved: bool) {
        let start = self.source.len();
        let keyword_span = self.text(if mutable { "let" } else { "const" });
        self.text(" ");
        let name = self.name(name);
        self.text(": ");
        let type_syntax = self.ty(Some(root));
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let initializer =
            if moved { self.reference("item") } else { self.constructor(root, false, false) };
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

    pub(super) fn assignment(
        &mut self,
        root: ReplacementRoot,
        case: ReplacementCase,
        empty_enum: bool,
    ) {
        let start = self.source.len();
        let target = self.reference("item");
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let value = match case {
            ReplacementCase::Move => self.reference("next"),
            ReplacementCase::SelfDirect => self.reference("item"),
            ReplacementCase::WrongType => self.payload(false),
            ReplacementCase::SelfNested => {
                let value_start = self.source.len();
                let keyword_span = self.text("Vec");
                let less_than_span = self.text("<");
                let argument = self.ty(Some(root));
                let greater_than_span = self.text(">");
                let type_syntax = self.types.len().try_into().expect("type");
                self.types.push(RawTypeSyntax {
                    span: at(value_start, self.source.len()),
                    kind: RawTypeSyntaxKind::Vec {
                        keyword_span,
                        less_than_span,
                        argument,
                        greater_than_span,
                    },
                });
                let open_paren_span = self.text("(");
                let open_bracket_span = self.text("[");
                let elements = vec![self.reference("item")];
                let close_bracket_span = self.text("]");
                let close_paren_span = self.text(")");
                self.expression(
                    value_start,
                    RawExpressionKind::VecConstruction {
                        type_syntax,
                        open_paren_span,
                        open_bracket_span,
                        elements,
                        close_bracket_span,
                        close_paren_span,
                    },
                )
            }
            ReplacementCase::SelfCall => {
                let value_start = self.source.len();
                let callee = self.name("identity");
                let open_paren_span = self.text("(");
                let arguments = vec![self.reference("item")];
                let close_paren_span = self.text(")");
                self.expression(
                    value_start,
                    RawExpressionKind::Call {
                        callee,
                        open_paren_span,
                        arguments,
                        close_paren_span,
                    },
                )
            }
            _ => self.constructor(root, matches!(case, ReplacementCase::InvalidLater), empty_enum),
        };
        let semicolon_span = self.text(";");
        self.statements.push(RawStatementSyntax {
            span: at(start, self.source.len()),
            kind: RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
        });
        self.text(" ");
    }
}
