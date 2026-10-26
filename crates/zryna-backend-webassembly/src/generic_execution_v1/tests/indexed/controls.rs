//! Same-seal index/container/access/cell identities and own-phase resource controls.
use super::{artifact, code, memory, model, program, raw};
use zryna_ir::generic_v1::execution_v1::VerifiedCleanupAction;
pub(super) fn identities(m: &model::Model<'_, '_, '_>, case: &str) {
    let mut count = 0;
    for function in &m.functions {
        for block in function.blocks() {
            for op in block.operations() {
                if let Some(indexed) = op.indexed_loan() {
                    count += 1;
                    let loan = indexed.loan();
                    assert_eq!(loan.function(), function.index());
                    assert!(
                        loan.origin().is_none(),
                        "indexed metadata never masquerades as a static origin"
                    );
                    assert_eq!(indexed.index().function(), function.index());
                    let raw::ValueType::Stored(index_ty) = indexed.index().ty() else {
                        panic!("index is retained i32")
                    };
                    assert_eq!(
                        m.types[index_ty as usize].category(),
                        zryna_layout::TypeCategory::I32
                    );
                    let container = m.types[indexed.container_type() as usize];
                    assert!(matches!(
                        container.category(),
                        zryna_layout::TypeCategory::Vec | zryna_layout::TypeCategory::FixedArray
                    ));
                    assert_eq!(
                        container.referenced_type().expect("exact element").index(),
                        loan.referent()
                    );
                    assert_ne!(indexed.base_place().is_some(), indexed.base_loan().is_some());
                    if let Some(place) = indexed.base_place() {
                        assert_eq!(place.function(), function.index());
                        assert_eq!(place.ty(), indexed.container_type());
                    }
                    if let Some(parent) = indexed.base_loan() {
                        assert_eq!(parent.function(), function.index());
                        assert_eq!(parent.referent(), indexed.container_type());
                    }
                    let frame = &m.memory.claims().functions[function.index()];
                    let cell = &frame.loans[loan.id() as usize];
                    assert_eq!(cell.referent, loan.referent());
                    assert_eq!(cell.access, loan.access());
                    assert_eq!(cell.offset % 4, 0);
                    assert!(frame.values.iter().all(|value| value.offset != cell.offset));
                    let failure = op.failure().expect("independent pre-Begin failure snapshot");
                    assert!(failure.actions().all(|action|!matches!(action,VerifiedCleanupAction::EndLoan(child)if child.id()==loan.id()&&child.function()==loan.function())),"bounds failure cannot own unpublished child");
                }
            }
        }
    }
    if case == "copy-read-control" {
        assert_eq!(count, 0);
    } else {
        assert!(count > 0, "actual indexed source witness");
    }
}
#[test]
fn indexed_raw_pointer_cell_type_access_and_storage_aliases_cannot_gain_authority() {
    for provider in super::source::PROVIDERS {
        program("chained-shared-copy", provider, |program, _| {
            let op = program
                .operations()
                .find(|op| op.indexed_loan().is_some())
                .expect("genuine indexed Begin");
            let indexed = op.indexed_loan().expect("bound origin");
            let loan = indexed.loan();
            for attack in 0..3 {
                let mut claim = memory::derive(program, false).expect("genuine indexed geometry");
                let frame = &mut claim.functions[loan.function()];
                match attack {
                    0 => frame.loans[loan.id() as usize].referent = indexed.container_type(),
                    1 => {
                        frame.loans[loan.id() as usize].access =
                            if loan.access() == raw::Access::Shared {
                                raw::Access::Exclusive
                            } else {
                                raw::Access::Shared
                            }
                    }
                    _ => frame.loans[loan.id() as usize].offset = frame.values[0].offset,
                }
                assert!(
                    memory::verify(claim, program).is_err(),
                    "mutated raw cell cannot issue same-seal address authority"
                );
            }
            memory::verify(memory::derive(program, false).expect("pristine claim retry"), program)
                .expect("no state retained from hostile claim");
        });
    }
}
#[test]
fn genuine_indexed_emission_exact_credit_first_debit_overflow_and_pristine_retry() {
    program("array-indexed-rhs-match", "native", |program, _| {
        let plan =
            memory::verify(memory::derive(program, true).expect("genuine geometry"), program)
                .expect("same-seal memory");
        let baseline = model::Model::new(program, &plan).expect("actual indexed model");
        baseline.work.set(0);
        let code = code::build(&baseline).expect("actual indexed body baseline");
        let cost = baseline.work.get();
        assert!(cost > 0);
        let bytes = artifact::encode(&baseline, &code).expect("actual baseline bytes");
        for before in [crate::generic_execution_v1::MAX_BYTES, usize::MAX] {
            let m = model::Model::new(program, &plan).expect("same model");
            m.work.set(before);
            let writer = m.writer.get();
            let Err(error) = code::build(&m) else { panic!("unfunded first reserve must reject") };
            assert_eq!(error.code(), "ZRYNA-W4101");
            assert_eq!(m.work.get(), before);
            assert_eq!(m.writer.get(), writer);
            m.work.set(0);
            let retry = code::build(&m).expect("pristine retry");
            assert_eq!(m.work.get(), cost);
            assert_eq!(artifact::encode(&m, &retry).expect("retry bytes"), bytes);
        }
        let m = model::Model::new(program, &plan).expect("exact-credit model");
        m.work.set(crate::generic_execution_v1::MAX_BYTES - cost);
        let exact = code::build(&m).expect("exact complete own-phase prepayment");
        assert_eq!(m.work.get(), crate::generic_execution_v1::MAX_BYTES);
        m.work.set(0);
        assert_eq!(artifact::encode(&m, &exact).expect("exact output bytes"), bytes);
    });
}
