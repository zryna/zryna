//! Exact symbolic operands and calls; unfinished source forms cannot issue success.
use super::{
    Checker, Closed, Expr, Failure, UntrustedSpan, Use, charge, check_depth, copy_key, key, reject,
    resolve, symbolic_fact,
};
impl Checker<'_, '_, '_> {
    pub(super) fn expression(
        &mut self,
        at: u32,
        usage: Use,
        depth: usize,
    ) -> Result<Closed, Failure> {
        check_depth(depth)?;
        charge(self.spent, 1)?;
        let source =
            self.original.body.expressions.get(at as usize).ok_or(Failure::InternalFailure)?;
        match &source.kind {
            Expr::Reference { name } => self.local(&name.text, name.span, usage),
            Expr::FieldAccess { .. } | Expr::Index { .. } if self.static_place(at)? => {
                let place = self.place(at, depth + 1)?;
                self.read_place(place, source.span, usage)
            }
            Expr::BoolLiteral { .. } => scalar(0, self.spent),
            Expr::I32Literal { spelling } => {
                charge(self.spent, spelling.len())?;
                spelling.parse::<i32>().map_err(|_| {
                    self.error(source.span, "ZRYNA-M7006", "original i32 literal is out of range")
                })?;
                scalar(1, self.spent)
            }
            Expr::StringLiteral { .. } => scalar(2, self.spent),
            Expr::Clone { value, keyword_span, .. } => self.cloning(*value, *keyword_span, depth),
            Expr::Negation { operand, .. } => {
                let value = self.expression(*operand, Use::Move, depth + 1)?;
                let expected = scalar(1, self.spent)?;
                self.expect(&value, &expected, source.span)?;
                Ok(expected)
            }
            Expr::Addition { lhs, rhs, .. }
            | Expr::Subtraction { lhs, rhs, .. }
            | Expr::Multiplication { lhs, rhs, .. }
            | Expr::LessThan { lhs, rhs, .. }
            | Expr::LessEqual { lhs, rhs, .. }
            | Expr::GreaterThan { lhs, rhs, .. }
            | Expr::GreaterEqual { lhs, rhs, .. } => {
                let left = self.expression(*lhs, Use::Move, depth + 1)?;
                let right = self.expression(*rhs, Use::Move, depth + 1)?;
                let expected = scalar(1, self.spent)?;
                self.expect(&left, &expected, source.span)?;
                self.expect(&right, &expected, source.span)?;
                scalar(
                    u8::from(matches!(
                        source.kind,
                        Expr::Addition { .. }
                            | Expr::Subtraction { .. }
                            | Expr::Multiplication { .. }
                    )),
                    self.spent,
                )
            }
            Expr::Equal { lhs, rhs, .. } | Expr::NotEqual { lhs, rhs, .. } => {
                let left = self.expression(*lhs, Use::Move, depth + 1)?;
                let right = self.expression(*rhs, Use::Move, depth + 1)?;
                self.expect(&left, &right, source.span)?;
                charge(self.spent, 1)?;
                if !matches!(&left, Closed::Stored(key) if matches!(key.as_slice(), [0 | 1])) {
                    return Err(self.error(
                        source.span,
                        "ZRYNA-M7006",
                        "original equality requires a scalar type",
                    ));
                }
                scalar(0, self.spent)
            }
            Expr::Call { callee, type_arguments, arguments, .. } => {
                self.call(callee, type_arguments.as_ref(), arguments, source.span, depth)
            }
            Expr::StructConstruction { type_name, type_arguments, fields, .. } => {
                self.structure(type_name, type_arguments.as_ref(), fields, source.span, depth)
            }
            Expr::EnumConstruction { type_name, variant, type_arguments, payload, .. } => self
                .enumeration(
                    type_name,
                    variant,
                    type_arguments.as_ref(),
                    *payload,
                    source.span,
                    depth,
                ),
            Expr::FixedArrayConstruction { type_syntax, elements, .. } => {
                self.sequence(*type_syntax, elements, true, source.span, depth)
            }
            Expr::VecConstruction { type_syntax, elements, .. } => {
                self.sequence(*type_syntax, elements, false, source.span, depth)
            }
            Expr::Shared { value, .. } => self.sharing(*value, source.span, depth),
            Expr::Downgrade { value, keyword_span, .. } => {
                self.downgrading(*value, *keyword_span, depth)
            }
            Expr::FieldAccess { base, field, .. } => {
                self.field(*base, &field.text, usage, source.span, depth)
            }
            Expr::Index { base, index, .. } => self.index(*base, *index, usage, source.span, depth),
            Expr::VecPush { vector, value, keyword_span, .. } => {
                self.pushing(*vector, *value, *keyword_span, depth)
            }
            Expr::Match { scrutinee, arms, .. } => {
                self.matching(*scrutinee, arms, source.span, depth)
            }
            _ => Err(reject("original loans/count operations are unfinished")),
        }
    }
    fn cloning(
        &mut self,
        value: u32,
        keyword_span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let operand =
            self.original.body.expressions.get(value as usize).ok_or(Failure::InternalFailure)?;
        let value = if let Expr::Reference { name } = &operand.kind {
            check_depth(depth + 1)?;
            charge(self.spent, 1)?;
            self.local(&name.text, keyword_span, Use::Observe)?
        } else {
            self.expression(value, Use::Observe, depth + 1)?
        };
        if !symbolic_fact(self.resolver.originals, self.owner, &value, self.spent)?.clone {
            return Err(self.error(
                keyword_span,
                "ZRYNA-M7002",
                "original type does not provide Clone",
            ));
        }
        Ok(value)
    }
    fn sequence(
        &mut self,
        at: u32,
        elements: &[u32],
        fixed: bool,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let ty = resolve(&self.resolver, at, self.spent)?;
        let Closed::Stored(key) = &ty else {
            return Err(reject("original sequence requires stored type"));
        };
        let shape = key::shape(key, self.spent)?;
        let valid = if fixed {
            shape.tag == 0x20
                && usize::try_from(shape.length).map_err(|_| Failure::InternalFailure)?
                    == elements.len()
        } else {
            shape.tag == 0x21
        };
        if !valid {
            return Err(self.error(
                span,
                "ZRYNA-M7006",
                "original sequence constructor has wrong type/length",
            ));
        }
        let expected = Closed::Stored(copy_key(shape.children[0], self.spent)?);
        for element in elements {
            let actual = self.expression(*element, Use::Move, depth + 1)?;
            self.expect(&actual, &expected, span)?;
        }
        symbolic_fact(self.resolver.originals, self.owner, &ty, self.spent)?;
        Ok(ty)
    }
}
pub(super) fn scalar(tag: u8, spent: &mut usize) -> Result<Closed, Failure> {
    Ok(Closed::Stored(copy_key(&[tag], spent)?))
}
