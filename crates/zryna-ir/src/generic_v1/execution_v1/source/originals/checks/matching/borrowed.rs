//! Borrowed scrutinees are observed; payload loans retain exact original variant paths.
use super::super::loans::Loan;
use super::super::places::Projection;
use super::super::{
    Checker, Closed, Expr, Failure, Local, add, charge, check_depth, copy_key, push, reserve,
    symbolic_fact,
};
impl<'s> Checker<'_, 's, '_> {
    pub(super) fn match_source(
        &mut self,
        at: u32,
        depth: usize,
    ) -> Result<(Closed, Option<usize>), Failure> {
        check_depth(depth + 1)?;
        charge(self.spent, 1)?;
        let source = &self.original.body.expressions[at as usize];
        match &source.kind {
            Expr::Reference { name } => {
                let index = self.binding(&name.text, name.span)?;
                if matches!(self.locals[index].ty, Closed::Borrow(..)) {
                    let place = self.local_place(index)?;
                    self.access(&place, false, false, name.span)?;
                    return Ok((place.ty, Some(index)));
                }
            }
            Expr::Borrow { value, keyword_span, .. }
            | Expr::BorrowMut { value, keyword_span, .. } => {
                let exclusive = matches!(source.kind, Expr::BorrowMut { .. });
                let place = self.place(*value, depth + 2)?;
                self.access(&place, exclusive, false, *keyword_span)?;
                let Closed::Stored(key) = &place.ty else {
                    return Err(self.error(
                        *keyword_span,
                        "ZRYNA-M7006",
                        "borrowed match requires a stored referent",
                    ));
                };
                symbolic_fact(self.resolver.originals, self.owner, &place.ty, self.spent)?;
                let ty = Closed::Borrow(copy_key(key, self.spent)?, exclusive);
                self.match_local_credit()?;
                let index = self.locals.len();
                // Empty identifiers cannot occur in authenticated source. Hidden parents
                // are appended directly so nested temporary loans need no invented name.
                push(
                    &mut self.locals,
                    Local {
                        name: "",
                        ty,
                        available: true,
                        moved_at: None,
                        holes: Vec::new(),
                        mutable: false,
                        loan: Some(Loan {
                            root: place.root,
                            path: place.path,
                            parent: place.through,
                            indexed: place.indexed,
                        }),
                    },
                    self.spent,
                )?;
                return Ok((place.ty, Some(index)));
            }
            _ => {}
        }
        Ok((self.expression(at, super::Use::Move, depth + 1)?, None))
    }
    fn match_local_credit(&mut self) -> Result<(), Failure> {
        // A new row can relocate the entire retained lexical prefix.
        let cost = add(self.locals.len(), 1)?
            .checked_mul(size_of::<Local<'_>>())
            .ok_or_else(|| super::super::exhausted("original match local credit overflow"))?;
        charge(self.spent, cost)
    }
    pub(super) fn match_payload(
        &mut self,
        name: &'s str,
        payload: &Closed,
        parent: usize,
        ordinal: usize,
    ) -> Result<(), Failure> {
        let Closed::Borrow(_, exclusive) = self.locals[parent].ty else {
            return Err(Failure::InternalFailure);
        };
        let place = self.local_place(parent)?;
        let Closed::Stored(bytes) = payload else { return Err(Failure::InternalFailure) };
        let ty = Closed::Borrow(copy_key(bytes, self.spent)?, exclusive);
        let count = add(place.path.len(), 1)?;
        charge(
            self.spent,
            count
                .checked_mul(size_of::<Projection>() + 1)
                .ok_or_else(|| super::super::exhausted("original match path credit overflow"))?,
        )?;
        let mut path = reserve(count)?;
        path.extend_from_slice(&place.path);
        if !place.indexed {
            path.push(Projection::EnumPayload(ordinal));
        }
        self.match_local_credit()?;
        self.bind(name, ty)?;
        let index = self.locals.len() - 1;
        self.locals[index].loan =
            Some(Loan { root: place.root, path, parent: Some(parent), indexed: place.indexed });
        Ok(())
    }
}
