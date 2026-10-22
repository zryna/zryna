//! Read-only symbolic type shapes and source-bound views.

use zryna_source::UntrustedSpan;

use super::super::{DeclarationIdentity, TypeParameterIdentity};
use super::model::Ty;
use super::{BodyTypeContext, model, resources, substitution};

/// The head shape of a symbolic source type, independent of closed layout or ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeShape {
    /// Boolean scalar.
    Bool,
    /// Signed 32-bit scalar.
    I32,
    /// Owned string type.
    String,
    /// Unit result type.
    Unit,
    /// Original declaration-owned opaque parameter.
    Parameter(TypeParameterIdentity),
    /// Original nominal data declaration.
    Nominal(DeclarationIdentity),
    /// Compiler-owned Option family.
    Option,
    /// Compiler-owned Result family.
    Result,
    /// Owned vector.
    Vec,
    /// Shared handle.
    Shared,
    /// Weak handle.
    Weak,
    /// Shared access type, without a loan certificate.
    Borrow,
    /// Exclusive access type, without a loan certificate.
    BorrowMut,
    /// Fixed array with its original length.
    FixedArray(u32),
    /// Original ordinary function value.
    Function(DeclarationIdentity),
}

/// A borrowed symbolic view. The origin and substitution environment cannot be constructed.
#[derive(Clone, Copy, Debug)]
pub struct TypeView<'v, 'c, 's> {
    pub(super) context: &'v BodyTypeContext<'c, 's>,
    pub(super) function: DeclarationIdentity,
    pub(super) ty: Ty,
}

impl<'v, 'c, 's> TypeView<'v, 'c, 's> {
    pub(in super::super) fn original_span(self) -> Option<UntrustedSpan> {
        match self.ty.origin {
            model::Origin::Source { module, occurrence } => Some(
                self.context.declarations.syntax().files()[module as usize].type_syntax
                    [occurrence as usize]
                    .span,
            ),
            model::Origin::Expression { function, expression } => Some(
                self.context.declarations.syntax().files()[function.module().index() as usize]
                    .functions[function.source_index() as usize]
                    .body
                    .expressions[expression as usize]
                    .span,
            ),
            _ => None,
        }
    }

    pub(in super::super) fn application_head_span(self) -> Option<UntrustedSpan> {
        match self.ty.origin {
            model::Origin::Source { module, occurrence } => {
                let node = &self.context.declarations.syntax().files()[module as usize].type_syntax
                    [occurrence as usize];
                match &node.kind {
                    zryna_syntax::v5::RawTypeSyntaxKind::Application { name, .. } => {
                        Some(name.span)
                    }
                    _ => Some(node.span),
                }
            }
            model::Origin::Expression { function, expression } => Some(
                resources::raw_function(self.context.declarations, function).body.expressions
                    [expression as usize]
                    .span,
            ),
            _ => None,
        }
    }
    fn head(self) -> model::Head {
        substitution::head(&self.context.tables, self.function, self.ty)
            .expect("retained checked environment")
            .expect("complete checked symbolic type")
    }

    /// Returns exact family/owner identity without unfolding nominal fields.
    #[must_use]
    pub fn shape(self) -> TypeShape {
        use model::{Kind, Scalar};
        match self.head().kind {
            Kind::Scalar(Scalar::Bool) => TypeShape::Bool,
            Kind::Scalar(Scalar::I32) => TypeShape::I32,
            Kind::Scalar(Scalar::String) => TypeShape::String,
            Kind::Scalar(Scalar::Unit) => TypeShape::Unit,
            Kind::Parameter(parameter) => TypeShape::Parameter(parameter),
            Kind::Nominal(owner) => TypeShape::Nominal(owner),
            Kind::Option => TypeShape::Option,
            Kind::Result => TypeShape::Result,
            Kind::Vec => TypeShape::Vec,
            Kind::Shared => TypeShape::Shared,
            Kind::Weak => TypeShape::Weak,
            Kind::Borrow => TypeShape::Borrow,
            Kind::BorrowMut => TypeShape::BorrowMut,
            Kind::FixedArray(length) => TypeShape::FixedArray(length),
            Kind::Function(owner) => TypeShape::Function(owner),
        }
    }

    /// Iterates at most two original ordered argument/element views.
    pub fn children(self) -> impl Iterator<Item = TypeView<'v, 'c, 's>> {
        self.head().children.into_iter().flatten().map(move |ty| TypeView { ty, ..self })
    }
}
