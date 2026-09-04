use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind};

use super::super::layout_graph::semantic_type;
use super::super::{Decl, Errors, SemanticInput, Ty, span};

#[path = "constructor_decisions.rs"]
mod constructors;

pub(super) struct ExpressionDecisions<'a, 'f, 'e> {
    pub(super) input: SemanticInput<'a>,
    pub(super) file: &'a syntax::SourceUnit,
    pub(super) function: &'f syntax::RawFunctionSyntax,
    pub(super) module: usize,
    pub(super) declarations: &'a [Decl],
    pub(super) graph: &'a raw_layout::Graph,
    pub(super) node_types: &'a [Option<Ty>],
    pub(super) layouts: &'a layout::VerifiedLayouts,
    pub(super) errors: &'e mut Errors<'a>,
}

pub(super) struct StructDecision {
    pub(super) children: Vec<(u32, u32)>,
}

pub(super) struct ArrayDecision<'f> {
    pub(super) elements: &'f [u32],
    pub(super) element: Ty,
}

pub(super) struct EnumDecision {
    pub(super) ordinal: usize,
    pub(super) payload_input: Option<(u32, Ty)>,
}

pub(super) struct ExpressionDecision<'f> {
    pub(super) at: Span,
    pub(super) kind: ExpressionKind<'f>,
}

pub(super) enum ExpressionKind<'f> {
    Bool(bool),
    I32(i32),
    String(&'f [u8]),
    Reference(&'f syntax::RawIdentifierSyntax),
    Projection(u32),
    StringClone(u32),
    AggregateClone(u32),
    Struct(StructDecision),
    Array(ArrayDecision<'f>),
    Enum(EnumDecision),
}

impl<'f> ExpressionDecisions<'_, 'f, '_> {
    pub(super) fn child_type(&mut self, type_syntax: u32) -> Option<Ty> {
        semantic_type(
            self.file,
            type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )
    }

    pub(super) fn classify(&mut self, id: u32, expected: Ty) -> Option<ExpressionDecision<'f>> {
        let expression =
            usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))?;
        let at = span(self.input.sources(), expression.span);
        let kind = match &expression.kind {
            RawExpressionKind::BoolLiteral { value } if expected.category == TypeCategory::Bool => {
                ExpressionKind::Bool(*value)
            }
            RawExpressionKind::I32Literal { spelling }
                if expected.category == TypeCategory::I32 =>
            {
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3016",
                        at,
                        "aggregate leaf integer is outside i32",
                        "use one in-range i32 leaf",
                    );
                    None
                })?;
                ExpressionKind::I32(value)
            }
            RawExpressionKind::StringLiteral { spelling }
                if expected.category == TypeCategory::String =>
            {
                let bytes = spelling.get(1..spelling.len().checked_sub(1)?)?.as_bytes();
                ExpressionKind::String(bytes)
            }
            RawExpressionKind::Reference { name } => ExpressionKind::Reference(name),
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. } => {
                ExpressionKind::Projection(id)
            }
            RawExpressionKind::Clone { value, .. } if expected.category == TypeCategory::String => {
                ExpressionKind::StringClone(*value)
            }
            RawExpressionKind::Clone { value, .. } => ExpressionKind::AggregateClone(*value),
            RawExpressionKind::StructConstruction { type_name, fields, .. }
                if expected.category == TypeCategory::Struct =>
            {
                ExpressionKind::Struct(self.struct_decision(type_name, fields, expected, at)?)
            }
            RawExpressionKind::FixedArrayConstruction { type_syntax, elements, .. }
                if expected.category == TypeCategory::FixedArray =>
            {
                ExpressionKind::Array(self.array_decision(*type_syntax, elements, expected, at)?)
            }
            RawExpressionKind::EnumConstruction { type_name, variant, payload, .. }
                if expected.category == TypeCategory::Enum =>
            {
                ExpressionKind::Enum(
                    self.enum_decision(type_name, variant, *payload, expected, at)?,
                )
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "expression is outside private owned Struct/Enum/FixedArray lowering",
                    "use literals, whole-value moves, and exact Struct/Enum/FixedArray constructors",
                );
                return None;
            }
        };
        Some(ExpressionDecision { at, kind })
    }
}
