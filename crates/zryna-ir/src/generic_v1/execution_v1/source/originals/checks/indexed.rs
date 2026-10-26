//! Indexed referents retain exact types and conservative container conflict identities.
use super::places::{Place, Projection};
use super::{
    Checker, Closed, Expr, Failure, Local, UntrustedSpan, Use, add, charge, check_depth,
    clone_type, copy_key, expressions, key, push,
};
impl Checker<'_, '_, '_> {
    pub(super) fn place_expression(&mut self, mut at: u32) -> Result<bool, Failure> {
        loop {
            charge(self.spent, 1)?;
            match self.original.body.expressions[at as usize].kind {
                Expr::Reference { .. } => return Ok(true),
                Expr::FieldAccess { base, .. } | Expr::Index { base, .. } => at = base,
                _ => return Ok(false),
            }
        }
    }
    fn retain_access(&mut self, place: Place, exclusive: bool) -> Result<(), Failure> {
        let cells = add(self.retained_accesses.len(), 1)?;
        charge(
            self.spent,
            cells.checked_mul(size_of::<(Place, bool)>()).ok_or(Failure::InternalFailure)?,
        )?;
        push(&mut self.retained_accesses, (place, exclusive), self.spent)
    }
    fn release_access(&mut self) -> Result<Place, Failure> {
        charge(self.spent, 1)?;
        self.retained_accesses.pop().map(|(place, _)| place).ok_or(Failure::InternalFailure)
    }
    pub(super) fn indexed_place(
        &mut self,
        base: u32,
        index: u32,
        depth: usize,
        span: UntrustedSpan,
    ) -> Result<Place, Failure> {
        let place = self.place(base, depth + 1)?;
        let Closed::Stored(bytes) = &place.ty else { return Err(Failure::InternalFailure) };
        let shape = key::shape(bytes, self.spent)?;
        if !matches!(shape.tag, 0x20 | 0x21) {
            return Err(self.error(span, "ZRYNA-M7006", "index requires an array or Vec referent"));
        }
        let fixed = shape.tag == 0x20;
        let length = u64::from(shape.length);
        let element = Closed::Stored(copy_key(shape.children[0], self.spent)?);
        let constant = match &self.original.body.expressions[index as usize].kind {
            Expr::I32Literal { spelling } => {
                charge(self.spent, spelling.len())?;
                spelling.parse::<u64>().ok().filter(|value| *value < length)
            }
            _ => None,
        };
        if fixed && !place.indexed && constant.is_some() {
            // Exact static children retain their independent availability and aliases.
            let actual = self.expression(index, Use::Move, depth + 1)?;
            let expected = expressions::scalar(1, self.spent)?;
            self.expect(&actual, &expected, self.original.body.expressions[index as usize].span)?;
            let mut place = place;
            push(
                &mut place.path,
                Projection::Element(constant.ok_or(Failure::InternalFailure)?),
                self.spent,
            )?;
            place.ty = element;
            return Ok(place);
        }
        // The base is prepared before the index; index effects cannot invalidate it.
        // Preserve the inherited missing-base diagnostic at the actual base expression.
        // Alias diagnostics belong to the consuming/borrowing operation's primary token.
        let base_span = self.original.body.expressions[base as usize].span;
        self.complete_original(&place, base_span)?;
        self.retain_access(place, false)?;
        let actual = self.expression(index, Use::Move, depth + 1)?;
        let expected = expressions::scalar(1, self.spent)?;
        self.expect(&actual, &expected, self.original.body.expressions[index as usize].span)?;
        let mut place = self.release_access()?;
        self.complete_original(&place, base_span)?;
        // A dynamic array index or any Vec element aliases its entire container.
        // Further nested projections retain this enclosing conflict identity.
        place.indexed = true;
        place.ty = element;
        Ok(place)
    }
    pub(super) fn assign_indexed(
        &mut self,
        place: Place,
        value: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<(), Failure> {
        self.access(&place, true, false, span)?;
        let expected = clone_type(&place.ty, self.spent)?;
        self.retain_access(place, true)?;
        let actual = self.expression(value, Use::Move, depth + 1)?;
        self.expect(&actual, &expected, self.original.body.expressions[value as usize].span)?;
        let place = self.release_access()?;
        self.access(&place, true, false, span)
        // Replacement preserves completeness. Dynamic elements never create or fill holes.
    }
    pub(super) fn push_indexed(
        &mut self,
        place: Place,
        value: u32,
        element: &Closed,
        keyword: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        self.retain_access(place, true)?;
        let actual = self.expression(value, Use::Move, depth + 1)?;
        self.expect(&actual, element, self.original.body.expressions[value as usize].span)?;
        let place = self.release_access()?;
        self.access(&place, true, false, keyword)?;
        Ok(Closed::Unit)
    }
    pub(super) fn inline_argument(&mut self, at: u32, depth: usize) -> Result<usize, Failure> {
        check_depth(depth)?;
        charge(self.spent, 1)?;
        let source = &self.original.body.expressions[at as usize];
        let (value, exclusive, keyword) = match source.kind {
            Expr::Borrow { value, keyword_span, .. } => (value, false, keyword_span),
            Expr::BorrowMut { value, keyword_span, .. } => (value, true, keyword_span),
            _ => return Err(Failure::InternalFailure),
        };
        let place = self.place(value, depth + 1)?;
        self.access(&place, exclusive, false, keyword)?;
        let Closed::Stored(bytes) = &place.ty else { return Err(Failure::InternalFailure) };
        let ty = Closed::Borrow(copy_key(bytes, self.spent)?, exclusive);
        let cells = add(self.locals.len(), 1)?;
        charge(
            self.spent,
            cells.checked_mul(size_of::<Local<'_>>()).ok_or(Failure::InternalFailure)?,
        )?;
        let index = self.locals.len();
        push(
            &mut self.locals,
            Local {
                name: "",
                ty,
                available: true,
                moved_at: None,
                holes: Vec::new(),
                mutable: false,
                loan: Some(super::loans::Loan {
                    root: place.root,
                    path: place.path,
                    parent: place.through,
                    indexed: place.indexed,
                }),
            },
            self.spent,
        )?;
        Ok(index)
    }
}
