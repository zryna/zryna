//! Symbolic loans carry actual root paths and lexical parent ancestry.
use super::places::{Place, Projection};
use super::{Checker, Closed, Expr, Failure, UntrustedSpan, add, charge, symbolic_fact};
pub(super) struct Loan {
    pub root: usize,
    pub path: Vec<Projection>,
    pub parent: Option<usize>,
    pub indexed: bool,
}
impl<'s> Checker<'_, 's, '_> {
    pub(super) fn ancestor(&mut self, ancestor: usize, mut child: usize) -> Result<bool, Failure> {
        loop {
            charge(self.spent, 1)?;
            if ancestor == child {
                return Ok(true);
            }
            match self.locals[child].loan.as_ref().and_then(|loan| loan.parent) {
                Some(parent) => child = parent,
                None => return Ok(false),
            }
        }
    }
    pub(super) fn overlaps(&mut self, left: &Place, right: &Place) -> Result<bool, Failure> {
        charge(self.spent, add(left.path.len().min(right.path.len()), 1)?)?;
        Ok(left.root == right.root && left.path.iter().zip(&right.path).all(|(a, b)| a == b))
    }
    pub(super) fn access(
        &mut self,
        place: &Place,
        write: bool,
        consume: bool,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        self.complete_original(place, span)?;
        self.access_rules(place, write, consume, span)
    }
    pub(super) fn assignment_access(
        &mut self,
        place: &Place,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        if place.through.is_none() {
            self.access_rules(place, true, false, span)
        } else {
            self.access(place, true, false, span)
        }
    }
    fn access_rules(
        &mut self,
        place: &Place,
        write: bool,
        consume: bool,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        if write {
            let allowed = match place.through {
                Some(index) => matches!(self.locals[index].ty, Closed::Borrow(_, true)),
                None => self.locals[place.root].mutable,
            };
            if !allowed {
                return Err(self.error(
                    span,
                    if place.through.is_some() { "ZRYNA-M7007" } else { "ZRYNA-M7006" },
                    "mutation requires an exclusive loan or mutable source binding",
                ));
            }
        }
        if write || consume {
            charge(self.spent, self.retained_reads.len())?;
            for read in &self.retained_reads {
                charge(self.spent, add(read.path.len().min(place.path.len()), 1)?)?;
                if read.root == place.root && read.path.iter().zip(&place.path).all(|(a, b)| a == b)
                {
                    return Err(self.error(
                        span,
                        "ZRYNA-M7007",
                        "operation invalidates a retained String operand",
                    ));
                }
            }
        }
        for (retained, exclusive) in &self.retained_accesses {
            charge(self.spent, add(retained.path.len().min(place.path.len()), 1)?)?;
            if retained.root == place.root
                && retained.path.iter().zip(&place.path).all(|(a, b)| a == b)
                && (write || consume || *exclusive)
            {
                return Err(self.error(
                    span,
                    "ZRYNA-M7007",
                    "operation overlaps prepared indexed access",
                ));
            }
        }
        for index in 0..self.locals.len() {
            charge(self.spent, 1)?;
            if self.locals[index].loan.is_none() {
                continue;
            }
            let other = self.local_place(index)?;
            if !self.overlaps(place, &other)? {
                continue;
            }
            if let Some(through) = place.through {
                if self.ancestor(index, through)? {
                    continue;
                }
                if matches!(self.locals[through].ty, Closed::Borrow(_, true))
                    && self.ancestor(through, index)?
                {
                    return Err(self.error(
                        span,
                        "ZRYNA-M7007",
                        "exclusive loan parent is frozen by an overlapping live child",
                    ));
                }
            }
            let exclusive = matches!(self.locals[index].ty, Closed::Borrow(_, true));
            if write || consume || exclusive {
                return Err(self.error(
                    span,
                    "ZRYNA-M7007",
                    "operation overlaps an incompatible live loan",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn borrow_binding(
        &mut self,
        name: &'s str,
        expected: Closed,
        mutable: bool,
        initializer: u32,
        depth: usize,
    ) -> Result<(), Failure> {
        super::check_depth(depth + 1)?;
        charge(self.spent, 1)?;
        let source = &self.original.body.expressions[initializer as usize];
        let (value, exclusive, keyword) = match &source.kind {
            Expr::Borrow { value, keyword_span, .. } => (*value, false, *keyword_span),
            Expr::BorrowMut { value, keyword_span, .. } => (*value, true, *keyword_span),
            _ => {
                return Err(self.error(
                    source.span,
                    "ZRYNA-M7007",
                    "loan binding requires an explicit matching borrow initializer",
                ));
            }
        };
        if mutable {
            return Err(self.error(keyword, "ZRYNA-M7007", "loan bindings must be constant"));
        }
        let Closed::Borrow(bytes, access) = &expected else {
            return Err(Failure::InternalFailure);
        };
        if *access != exclusive {
            return Err(self.error(
                keyword,
                "ZRYNA-M7006",
                "borrow initializer access differs from annotation",
            ));
        }
        let place = self.place(value, depth + 2)?;
        let actual = match &place.ty {
            Closed::Stored(bytes) => Closed::Borrow(super::copy_key(bytes, self.spent)?, exclusive),
            _ => return Err(Failure::InternalFailure),
        };
        self.expect(&actual, &expected, keyword)?;
        let referent = Closed::Stored(super::copy_key(bytes, self.spent)?);
        symbolic_fact(self.resolver.originals, self.owner, &referent, self.spent)?;
        self.access(&place, exclusive, false, keyword)?;
        self.bind(name, expected)?;
        let index = self.locals.len() - 1;
        self.locals[index].loan = Some(Loan {
            root: place.root,
            path: place.path,
            parent: place.through,
            indexed: place.indexed,
        });
        Ok(())
    }
}
