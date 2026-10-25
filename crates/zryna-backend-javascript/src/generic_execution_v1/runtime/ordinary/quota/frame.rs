//! Invocation-local indices alias caller-owned abstract cells for formal loan effects.
use super::{
    Analysis, Argument, Cost, Diagnostic, Function, Potential, VerifiedParameter, charge,
    emission_budget, filled, internal, normalized, raw,
};
use crate::generic_execution_v1::storage::loans::{Base, resolve};

pub(super) struct Frame {
    pub(super) places: Vec<usize>,
    pub(super) loans: Vec<Potential>,
    pub(super) temporary: Vec<Option<usize>>,
    pub(super) marker: usize,
}
impl Analysis<'_, '_, '_> {
    pub(super) fn frame(
        &mut self,
        function: &Function<'_, '_>,
        arguments: &[Argument],
        values: &mut [Cost],
    ) -> Result<Frame, Diagnostic> {
        let context = self.c;
        let marker = self.cells.len();
        let end = marker.checked_add(function.places.len()).ok_or_else(emission_budget)?;
        charge(
            &context.work,
            // The same arena retains caller cells while a callee frame is appended.
            // Exact reserve may copy those existing cells before initializing new rows.
            end.checked_mul(size_of::<Cost>()).ok_or_else(emission_budget)?,
        )?;
        self.cells.try_reserve_exact(function.places.len()).map_err(|_| emission_budget())?;
        self.cells.resize(end, Cost::default());
        let mut places = filled(context, function.places.len(), 0usize)?;
        let mut temporary = filled(context, function.values.len(), None)?;
        charge(
            &context.work,
            function
                .parameters
                .len()
                .checked_add(function.places.len())
                .ok_or_else(emission_budget)?,
        )?;
        for (parameter, argument) in function.parameters.iter().zip(arguments) {
            match (parameter, argument) {
                (VerifiedParameter::Value(value), Argument::Value(potential)) => {
                    values[value.id() as usize] = normalized(context, value.ty(), *potential)?;
                }
                (VerifiedParameter::Loan(_), Argument::Loan(_)) => {}
                _ => return Err(internal()),
            }
        }
        for (index, place) in function.places.iter().enumerate() {
            places[index] = marker + index;
            match place.kind() {
                raw::PlaceKind::Parameter(ordinal) => {
                    let VerifiedParameter::Value(value) = function.parameters[ordinal as usize]
                    else {
                        return Err(internal());
                    };
                    self.cells[marker + index] = values[value.id() as usize];
                    temporary[value.id() as usize] = Some(marker + index);
                }
                raw::PlaceKind::Temporary(value) => {
                    temporary[value as usize] = Some(marker + index);
                }
                _ => {}
            }
        }
        let mut loans = filled(context, function.loans.len(), Potential::default())?;
        charge(&context.work, function.loans.len())?;
        for loan in &function.loans {
            let address = resolve(context, function, *loan)?;
            let parent = match address.base {
                Base::Place(id) => Potential { cell: places[id as usize], projected: false },
                Base::Formal(ordinal) => {
                    let Some(Argument::Loan(parent)) = arguments.get(ordinal as usize) else {
                        return Err(internal());
                    };
                    *parent
                }
            };
            loans[loan.id() as usize] = Potential {
                projected: parent.projected || address.indexed || !address.path.is_empty(),
                ..parent
            };
        }
        Ok(Frame { places, loans, temporary, marker })
    }
    pub(super) fn operand(&self, values: &[Cost], temporary: &[Option<usize>], id: u32) -> Cost {
        temporary[id as usize].map_or(values[id as usize], |cell| self.cells[cell])
    }
    pub(super) fn observed(
        &self,
        function: &Function<'_, '_>,
        places: &[usize],
        loans: &[Potential],
        source: raw::PlaceOrLoan,
    ) -> Result<Cost, Diagnostic> {
        let (ty, pointer) = match source {
            raw::PlaceOrLoan::Place(id) => {
                let place = function.place(id)?;
                (
                    place.ty(),
                    Potential {
                        cell: places[place.root().id() as usize],
                        projected: place.id() != place.root().id(),
                    },
                )
            }
            raw::PlaceOrLoan::Loan(id) => (function.loan(id)?.referent(), loans[id as usize]),
        };
        normalized(self.c, raw::ValueType::Stored(ty), self.cells[pointer.cell])
    }
    pub(super) fn target(
        function: &Function<'_, '_>,
        places: &[usize],
        loans: &[Potential],
        source: raw::PlaceOrLoan,
    ) -> Result<Potential, Diagnostic> {
        Ok(match source {
            raw::PlaceOrLoan::Place(id) => {
                let place = function.place(id)?;
                Potential {
                    cell: places[place.root().id() as usize],
                    projected: place.id() != place.root().id(),
                }
            }
            raw::PlaceOrLoan::Loan(id) => loans[id as usize],
        })
    }
}

#[cfg(test)]
mod prepayment_tests {
    use super::*;
    use crate::generic_execution_v1::{Context, tests::with_program};
    fn analysis<'c, 'p, 'a>(context: &'c Context<'p, 'a>) -> Analysis<'c, 'p, 'a> {
        Analysis {
            c: context,
            active: vec![false; context.functions.len()],
            cells: vec![Cost::default(); 4096],
        }
    }
    #[test]
    fn callee_frame_growth_pays_live_caller_cells_before_reserve() {
        with_program("fixed-array-identity", "native", |program| {
            let context = Context::new(program).expect("genuine new execution context");
            let function = context
                .functions
                .iter()
                .find(|f| !f.places.is_empty())
                .expect("actual parameter/local storage");
            let args = vec![Argument::default(); function.parameters.len()];
            let values = || vec![Cost::default(); function.values.len()];
            let full_prefix = (4096 + function.places.len()) * size_of::<Cost>();
            let limit = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;
            let mut baseline = analysis(&context);
            let mut v = values();
            context.work.set(0);
            baseline.frame(function, &args, &mut v).expect("actual retained caller frame baseline");
            let cost = context.work.get();
            assert!(cost >= full_prefix, "caller-cell copies cannot be funded by only callee rows");
            let mut exact = analysis(&context);
            context.work.set(limit - cost);
            exact.frame(function, &args, &mut values()).expect("exact full frame credit");
            assert_eq!(context.work.get(), limit);
            for start in [limit - full_prefix + 1, usize::MAX] {
                let mut failed = analysis(&context);
                let capacity = failed.cells.capacity();
                context.work.set(start);
                let error = failed
                    .frame(function, &args, &mut values())
                    .err()
                    .expect("first missing prefix debit rejects before any frame reserve/resize");
                assert_eq!(error.code(), "ZRYNA-J2003");
                assert_eq!(context.work.get(), start);
                assert_eq!(failed.cells.len(), 4096);
                assert_eq!(failed.cells.capacity(), capacity);
            }
            context.work.set(0);
            analysis(&context)
                .frame(function, &args, &mut values())
                .expect("pristine caller/callee retry");
            assert_eq!(context.work.get(), cost);
        });
    }
}
