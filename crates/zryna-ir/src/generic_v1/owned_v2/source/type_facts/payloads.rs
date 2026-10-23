//! Allocation-free preflight of every original nominal variant substitution.
use super::super::Budget;
use super::domain::{Shape, add, charged_argument};
use crate::generic_v1::{Failure, budget, reject, source::Originals};
use zryna_syntax::v5::{RawDataDeclaration, RawTypeSyntaxKind};

pub(super) struct Frame<'a, 'b> {
    pub originals: &'a Originals<'b>,
    pub module: u32,
    pub declaration: &'b RawDataDeclaration,
    pub arguments: &'a [&'a [u8]],
}
impl Frame<'_, '_> {
    pub(super) fn shape(
        &self,
        occurrence: u32,
        depth: usize,
        credit: &mut Budget,
    ) -> Result<Shape, Failure> {
        credit.nominal(1)?;
        if depth > 64 {
            return Err(budget("nominal substituted key exceeds inherited byte/depth ceiling"));
        }
        let kind = &self.originals.units[self.module as usize]
            .type_syntax
            .get(occurrence as usize)
            .ok_or(Failure::InternalFailure)?
            .kind;
        match kind {
            RawTypeSyntaxKind::Named { name } => {
                if let Some(slot) =
                    self.declaration.type_parameters.as_ref().and_then(|list| {
                        list.parameters.iter().position(|p| p.name.text == name.text)
                    })
                {
                    return charged_argument(self.arguments[slot], credit);
                }
                match name.text.as_str() {
                    "bool" | "i32" => {
                        credit.nominal(1)?;
                        Ok(Shape::leaf(1, false))
                    }
                    _ => Err(reject("nominal payload requires an admitted original stored type")),
                }
            }
            RawTypeSyntaxKind::String { .. } => {
                credit.nominal(1)?;
                Ok(Shape::leaf(1, true))
            }
            RawTypeSyntaxKind::Application { name, type_arguments } => {
                if self
                    .declaration
                    .type_parameters
                    .as_ref()
                    .is_some_and(|list| list.parameters.iter().any(|p| p.name.text == name.text))
                {
                    return Err(reject("opaque parameter cannot be a nominal payload family head"));
                }
                let count = match name.text.as_str() {
                    "Option" => 1,
                    "Result" => 2,
                    _ => {
                        return Err(reject(
                            "nominal payload/argument requires scalar/String/Option/Result",
                        ));
                    }
                };
                if type_arguments.arguments.len() != count {
                    return Err(reject("nominal payload has inexact compiler-family arity"));
                }
                let mut children = [Shape::leaf(0, false); 2];
                for (child, occurrence) in children.iter_mut().zip(&type_arguments.arguments) {
                    *child = self.shape(*occurrence, add(depth, 1)?, credit)?;
                }
                Shape::family(&children[..count])
            }
            _ => Err(reject("nominal payload/argument requires scalar/String/Option/Result")),
        }
    }
}
