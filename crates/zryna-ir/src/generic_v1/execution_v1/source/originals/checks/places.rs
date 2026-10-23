//! Exact source static places retain declaration member and constant array identities.
use super::{
    Checker, Closed, Expr, Failure, UntrustedSpan, add, charge, check_depth, clone_type, copy_key,
    description, key, push, reserve,
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Projection {
    Field(usize),
    Element(u64),
}
pub(super) struct Place {
    pub root: usize,
    pub path: Vec<Projection>,
    pub through: Option<usize>,
    pub ty: Closed,
}
impl Checker<'_, '_, '_> {
    pub(super) fn binding(&mut self, name: &str, span: UntrustedSpan) -> Result<usize, Failure> {
        for index in (0..self.locals.len()).rev() {
            charge(self.spent, add(self.locals[index].name.len().min(name.len()), 1)?)?;
            if self.locals[index].name == name {
                if !self.locals[index].available {
                    return Err(self.error(
                        span,
                        "ZRYNA-M7007",
                        "original value has already moved",
                    ));
                }
                return Ok(index);
            }
        }
        Err(self.error(span, "ZRYNA-M7006", "original reference has no exact lexical binding"))
    }
    pub(super) fn static_place(&mut self, mut at: u32) -> Result<bool, Failure> {
        loop {
            charge(self.spent, 1)?;
            match &self.original.body.expressions[at as usize].kind {
                Expr::Reference { .. } => return Ok(true),
                Expr::FieldAccess { base, .. } => at = *base,
                Expr::Index { base, index, .. } => {
                    if !matches!(
                        self.original.body.expressions[*index as usize].kind,
                        Expr::I32Literal { .. }
                    ) {
                        return Ok(false);
                    }
                    if !self.static_place(*base)? {
                        return Ok(false);
                    }
                    let place = self.place(*base, 1)?;
                    if let Closed::Stored(bytes) = &place.ty {
                        if key::shape(bytes, self.spent)?.tag != 0x20 {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                    at = *base;
                }
                _ => return Ok(false),
            }
        }
    }
    pub(super) fn local_place(&mut self, index: usize) -> Result<Place, Failure> {
        let local = &self.locals[index];
        let ty = match &local.ty {
            Closed::Borrow(key, _) => Closed::Stored(copy_key(key, self.spent)?),
            _ => clone_type(&local.ty, self.spent)?,
        };
        if let Some(loan) = &local.loan {
            charge(self.spent, loan.path.len())?;
            let mut path = reserve(loan.path.len())?;
            path.extend_from_slice(&loan.path);
            Ok(Place { root: loan.root, path, through: Some(index), ty })
        } else {
            Ok(Place { root: index, path: Vec::new(), through: None, ty })
        }
    }
    pub(super) fn place(&mut self, at: u32, depth: usize) -> Result<Place, Failure> {
        check_depth(depth)?;
        charge(self.spent, 1)?;
        let source = &self.original.body.expressions[at as usize];
        match &source.kind {
            Expr::Reference { name } => {
                let index = self.binding(&name.text, name.span)?;
                self.local_place(index)
            }
            Expr::FieldAccess { base, field, .. } => {
                let mut place = self.place(*base, depth + 1)?;
                let Closed::Stored(bytes) = &place.ty else {
                    return Err(Failure::InternalFailure);
                };
                let shape = key::shape(bytes, self.spent)?;
                if !matches!(shape.tag, 0x10 | 0x12) {
                    return Err(self.error(
                        field.span,
                        "ZRYNA-M7006",
                        "field requires an exact struct",
                    ));
                }
                charge(self.spent, shape.children.len())?;
                let mut args = reserve(shape.children.len())?;
                for child in &shape.children {
                    args.push(copy_key(child, self.spent)?);
                }
                let desc = description::identity(
                    self.resolver.originals,
                    shape.module,
                    shape.index,
                    &args,
                    self.spent,
                )?;
                for (ordinal, (name, ty)) in desc.members.into_iter().enumerate() {
                    charge(self.spent, add(name.len().min(field.text.len()), 1)?)?;
                    if name == field.text {
                        place.ty = ty.ok_or(Failure::InternalFailure)?;
                        push(&mut place.path, Projection::Field(ordinal), self.spent)?;
                        return Ok(place);
                    }
                }
                Err(self.error(field.span, "ZRYNA-M7006", "field is absent from original struct"))
            }
            Expr::Index { base, index, .. } => {
                let mut place = self.place(*base, depth + 1)?;
                let operand = &self.original.body.expressions[*index as usize];
                let Expr::I32Literal { spelling } = &operand.kind else {
                    return Err(self.error(
                        operand.span,
                        "ZRYNA-M7006",
                        "loan place needs a constant index",
                    ));
                };
                charge(self.spent, spelling.len())?;
                let index = spelling.parse::<u64>().map_err(|_| {
                    self.error(operand.span, "ZRYNA-M7006", "static index must be nonnegative")
                })?;
                let Closed::Stored(bytes) = &place.ty else {
                    return Err(Failure::InternalFailure);
                };
                let shape = key::shape(bytes, self.spent)?;
                if shape.tag != 0x20 || index >= u64::from(shape.length) {
                    return Err(self.error(
                        operand.span,
                        "ZRYNA-M7006",
                        "static array index is out of bounds",
                    ));
                }
                place.ty = Closed::Stored(copy_key(shape.children[0], self.spent)?);
                push(&mut place.path, Projection::Element(index), self.spent)?;
                Ok(place)
            }
            _ => Err(self.error(
                source.span,
                "ZRYNA-M7007",
                "loan operation requires a source place",
            )),
        }
    }
}
