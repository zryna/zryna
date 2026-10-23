//! Exact original operand order and symbolic source call substitution.

use super::{Builder, Value};
use crate::generic_v1::{Failure, raw, reject, reserve, source_types::Closed};
use zryna_syntax::v5::RawExpressionKind as Expr;
mod calls;

impl Builder<'_, '_> {
    pub(super) fn expression(&mut self, index: u32, depth: usize) -> Result<Value, Failure> {
        let span = self.original.body.expressions[index as usize].span;
        self.expression_inner(index, depth).map_err(|failure| self.locate(failure, span))
    }
    fn expression_inner(&mut self, index: u32, depth: usize) -> Result<Value, Failure> {
        if depth > 128 {
            return Err(crate::generic_v1::budget("source expression depth exceeds 128"));
        }
        let source = &self.original.body.expressions[index as usize];
        match &source.kind {
            Expr::Reference { name } => self.reference_expression(name, source.span),
            Expr::StringLiteral { spelling } => {
                let text = spelling
                    .get(1..spelling.len() - 1)
                    .ok_or_else(|| reject("invalid string spelling"))?;
                // Symbolic checking proves ownership without retaining executable bytes.
                let bytes = if self.symbolic { &[][..] } else { text.as_bytes() };
                self.budget.literal(bytes.len())?;
                let mut literal = reserve(bytes.len())?;
                literal.extend_from_slice(bytes);
                self.ext(Closed::Stored(vec![2]), source.span, super::Owned::StringLiteral(literal))
            }
            Expr::Borrow { value, .. } | Expr::BorrowMut { value, .. } => {
                let value = self.reference_operand(*value)?;
                let Closed::Stored(key) = &value.ty else {
                    return Err(reject("loan requires a stored owner"));
                };
                let exclusive = matches!(source.kind, Expr::BorrowMut { .. });
                self.type_facts.key_copy(&value.ty, 1, &mut self.budget)?;
                self.ext(
                    Closed::Borrow(key.clone(), exclusive),
                    source.span,
                    super::Owned::Borrow { value: value.id, exclusive },
                )
            }
            Expr::Clone { value, .. } => {
                let value = self.reference_operand(*value)?;
                if super::clones::admitted(&value.ty) {
                    return self.clone_enum(value, source.span);
                }
                if value.ty != Closed::Stored(vec![2]) {
                    return Err(reject(
                        "this owned opcode requires exact String; opaque T cannot acquire Clone",
                    ));
                }
                self.ext(value.ty.clone(), source.span, super::Owned::CloneString(value.id))
            }
            Expr::BoolLiteral { value } => {
                self.emit(Closed::Stored(vec![0]), source.span, raw::Operation::BoolLiteral(*value))
            }
            Expr::I32Literal { spelling } => {
                let value = spelling
                    .parse::<i32>()
                    .map_err(|_| reject("source i32 literal is outside the exact scalar range"))?;
                self.emit(Closed::Stored(vec![1]), source.span, raw::Operation::I32Literal(value))
            }
            Expr::Addition { lhs, rhs, .. } => {
                let left = self.expression(*lhs, depth + 1)?;
                let right = self.expression(*rhs, depth + 1)?;
                if left.ty != Closed::Stored(vec![1]) || right.ty != Closed::Stored(vec![1]) {
                    return Err(reject(
                        "source addition requires original i32 operands; opaque T cannot specialize it",
                    ));
                }
                self.emit(
                    left.ty,
                    source.span,
                    raw::Operation::I32Add { left: left.id, right: right.id },
                )
            }
            Expr::Call { callee, type_arguments, arguments, .. } => {
                self.call(callee, type_arguments.as_ref(), arguments, source.span, depth)
            }
            Expr::EnumConstruction { type_name, variant, type_arguments, payload, .. } => self
                .construct_enum(
                    type_name,
                    variant,
                    type_arguments.as_ref(),
                    *payload,
                    source.span,
                    depth,
                ),
            Expr::StructConstruction { type_name, type_arguments, fields, .. } => self
                .construct_struct(type_name, type_arguments.as_ref(), fields, source.span, depth),
            Expr::FieldAccess { base, field, .. } => {
                self.copy_struct_field(*base, field, source.span, depth)
            }
            Expr::Match { scrutinee, arms, .. } => {
                self.match_expression(*scrutinee, arms, source.span, depth + 1)
            }
            _ => Err(reject(
                "source expression requires a successor lane beyond immutable Copy replay",
            )),
        }
    }
    fn reference_expression(
        &mut self,
        name: &zryna_syntax::v4::RawIdentifierSyntax,
        span: zryna_source::UntrustedSpan,
    ) -> Result<Value, Failure> {
        let value = self.peek(&name.text)?;
        if self.affine[value.id as usize] {
            self.type_facts.key_copy(&value.ty, 1, &mut self.budget)?;
            self.ext(value.ty.clone(), span, super::Owned::Move(value.id))
        } else if self
            .locals
            .iter()
            .rev()
            .find(|local| local.name == name.text)
            .is_some_and(|local| local.mutable)
        {
            self.emit(value.ty, span, raw::Operation::Copy { value: value.id })
        } else {
            Ok(value)
        }
    }
    fn construct_enum(
        &mut self,
        type_name: &zryna_syntax::v4::RawIdentifierSyntax,
        variant: &zryna_syntax::v4::RawIdentifierSyntax,
        type_arguments: Option<&zryna_syntax::v5::RawTypeArgumentList>,
        payload: Option<u32>,
        span: zryna_source::UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let mut arguments = reserve(type_arguments.map_or(0, |list| list.arguments.len()))?;
        for occurrence in type_arguments.map_or(&[][..], |list| &list.arguments) {
            let Closed::Stored(key) = self.resolve(*occurrence)? else {
                return Err(reject("enum argument is not a stored type"));
            };
            arguments.push(key);
        }
        let description = self.type_facts.enum_description(
            &self.resolver,
            &type_name.text,
            &arguments,
            self.symbolic,
            &mut self.budget,
        )?;
        let ordinal = self
            .type_facts
            .variant(&description.variants, &variant.text, &mut self.budget)?
            .ok_or_else(|| reject("source constructor names an unknown enum variant"))?;
        let value = payload.map(|payload| self.expression(payload, depth + 1)).transpose()?;
        if !self.type_facts.equal_payload(
            value.as_ref().map(|value| &value.ty),
            description.variants[ordinal].1.as_ref(),
            &mut self.budget,
        )? {
            return Err(reject(
                "source enum constructor payload differs from exact original active variant",
            ));
        }
        let ty = if self.symbolic {
            0
        } else {
            self.type_facts.v4_copy(&description.ty, 1, &mut self.budget)?;
            let raw::Type::Stored(id) =
                self.type_facts.type_id(self.program, description.ty.clone(), &mut self.budget)?
            else {
                return Err(Failure::InternalFailure);
            };
            id
        };
        self.emit(
            description.ty,
            span,
            raw::Operation::ClosedEnumConstruct {
                ty,
                ordinal: u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
                payload: value.map(|value| value.id),
            },
        )
    }
}
