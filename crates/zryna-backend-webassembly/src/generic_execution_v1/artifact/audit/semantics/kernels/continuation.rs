//! Complete typed-operator obligations for the two fixed canonical continuation adapters.
//! These are derived from runtime roles and sealed metadata, never encoder recipes or bytes.
use super::patterns::{A, atom};
use super::{Diagnostic, FunctionBody, Model, invariant};
use wasmparser::ValType;
mod concat;
mod reserve;
struct Proof<'m, 'p, 'a, 's> {
    model: &'m Model<'p, 'a, 's>,
    obligations: Vec<A>,
}
impl Proof<'_, '_, '_, '_> {
    fn add(&mut self, obligations: &[A]) -> Result<(), Diagnostic> {
        // Growing this vector may copy every initialized old obligation before append.
        let copied_and_new =
            if obligations.len() > self.obligations.capacity() - self.obligations.len() {
                self.obligations
                    .len()
                    .checked_add(obligations.len())
                    .ok_or_else(super::super::budget)?
            } else {
                obligations.len()
            };
        self.model
            .take(copied_and_new.checked_mul(size_of::<A>()).ok_or_else(super::super::budget)?)?;
        #[cfg(test)]
        crate::generic_execution_v1::work::reserve_seen(obligations.len());
        self.obligations.try_reserve(obligations.len()).map_err(|_| super::super::budget())?;
        self.obligations.extend_from_slice(obligations);
        Ok(())
    }
    fn fail(&mut self, category: i64, status: i64) -> Result<(), Diagnostic> {
        self.add(&[A(6, category), A(5, 1), A(6, status), A(22, 0)])
    }
    fn valid(&mut self, local: i64) -> Result<(), Diagnostic> {
        self.add(&[
            A(1, local),
            A(6, i64::from(self.model.memory.claims().heap_start) + 16),
            A(15, 0),
            A(1, local),
            A(4, 0),
            A(17, 0),
            A(18, 0),
            A(1, local),
            A(6, 7),
            A(19, 0),
            A(18, 0),
            A(21, 0),
        ])?;
        self.fail(255, 255)?;
        self.add(&[
            A(28, 0),
            A(1, local),
            A(6, 16),
            A(10, 0),
            A(7, 8),
            A(6, 1),
            A(13, 0),
            A(21, 0),
        ])?;
        self.fail(255, 255)?;
        self.add(&[A(28, 0)])
    }
}
pub(in crate::generic_execution_v1::artifact::audit::semantics) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    model: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let index32 = crate::generic_execution_v1::checked_u32(index)?;
    let role = if model.concat && index32 == model.concat_allocator() {
        12
    } else if model.concat && index32 == model.concat_index() {
        13
    } else {
        index32 - model.container_base
    };
    let locals = match role {
        10 => 5,
        11 => 0,
        12 => 3,
        13 => 8,
        _ => return Err(invariant()),
    };
    let mut reader = body.get_locals_reader().map_err(|_| invariant())?;
    if locals == 0 {
        if reader.get_count() != 0 {
            return Err(invariant());
        }
    } else if reader.get_count() != 1
        || reader.read().map_err(|_| invariant())? != (locals, ValType::I32)
    {
        return Err(invariant());
    }
    let mut proof = Proof { model, obligations: Vec::new() };
    match role {
        10 => reserve::prove(&mut proof)?,
        11 => upgrade(&mut proof)?,
        12 => concat::allocator(&mut proof)?,
        13 => concat::prove(&mut proof)?,
        _ => return Err(invariant()),
    }
    proof.add(&[A(28, 0)])?;
    let mut operators = body.get_operators_reader().map_err(|_| invariant())?;
    for expected in proof.obligations {
        model.take(1)?;
        if operators.eof() || atom(&operators.read().map_err(|_| invariant())?) != expected {
            return Err(invariant());
        }
    }
    if !operators.eof() {
        return Err(invariant());
    }
    Ok(())
}
fn upgrade(proof: &mut Proof<'_, '_, '_, '_>) -> Result<(), Diagnostic> {
    proof.valid(0)?;
    proof.add(&[
        A(1, 0),
        A(6, 16),
        A(10, 0),
        A(7, 4),
        A(6, 8),
        A(15, 0),
        A(1, 0),
        A(6, 16),
        A(10, 0),
        A(7, 12),
        A(6, 256),
        A(19, 0),
        A(18, 0),
        A(21, 0),
    ])?;
    proof.fail(255, 255)?;
    proof.add(&[A(28, 0), A(1, 0), A(7, 4), A(14, 0), A(21, 0)])?;
    proof.fail(255, 255)?;
    proof.add(&[
        A(28, 0),
        A(1, 0),
        A(7, 0),
        A(14, 0),
        A(21, 0),
        A(6, 5),
        A(22, 0),
        A(28, 0),
        A(1, 0),
        A(7, 0),
        A(6, -1),
        A(12, 0),
        A(21, 0),
    ])?;
    proof.fail(4, 3)?;
    proof.add(&[A(28, 0), A(1, 0), A(1, 0), A(7, 0), A(6, 1), A(9, 0), A(8, 0), A(6, 0)])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoded_concat_obligation_retention_is_paid_before_reserve_and_failure_is_atomic() {
        crate::generic_execution_v1::tests::concat_program("same-source", |program, _| {
            let plan = crate::generic_execution_v1::memory::verify(
                crate::generic_execution_v1::memory::derive(program, false).expect("actual memory"),
                program,
            )
            .expect("same-program plan");
            let model = Model::new(program, &plan).expect("genuine concat model");
            model.work.set(crate::generic_execution_v1::MAX_BYTES - size_of::<A>());
            let mut proof = Proof { model: &model, obligations: Vec::new() };
            proof.add(&[A(6, 0)]).expect("exact one-obligation debit");
            assert_eq!(model.work.get(), crate::generic_execution_v1::MAX_BYTES);
            crate::generic_execution_v1::work::reset_seen();
            assert!(proof.add(&[A(28, 0)]).is_err());
            assert!(proof.obligations == vec![A(6, 0)]);
            assert_eq!(crate::generic_execution_v1::work::seen(), 0);
        });
    }
}

#[cfg(test)]
mod prefix_tests;
