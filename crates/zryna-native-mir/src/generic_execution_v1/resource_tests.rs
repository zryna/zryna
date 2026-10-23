//! Owning native admission controls over genuine first-five and settled-container seals.
use super::{MAX_CODEGEN_UNITS, VerifiedProgram, lower, lower_from, planning::CopySite, raw};
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, VerifiedOperation, VerifiedParameter};
use zryna_layout::StorageTarget;
mod containers;
mod fixture;
mod next;
pub(super) mod trace;

#[test]
fn genuine_first_five_exact_first_extra_overflow_and_pristine_retry() {
    for case in fixture::CASES {
        fixture::with_program(case, |program| {
            check_boundary(program, case);
        });
    }
}
fn check_boundary(program: &VerifiedExecution<'_>, case: &str) {
    let mut used = 0;
    let baseline = lower_from(program, &mut used).expect("genuine native owning baseline");
    let expected = fingerprint(&baseline);
    assert!(used > 0 && used <= MAX_CODEGEN_UNITS);
    let mut exact = MAX_CODEGEN_UNITS - used;
    let accepted = lower_from(program, &mut exact).expect("exact remaining prepaid units");
    assert_eq!(exact, MAX_CODEGEN_UNITS);
    assert_eq!(fingerprint(&accepted), expected, "{case}: initial credit is not authority");
    let mut extra = MAX_CODEGEN_UNITS - used + 1;
    let failure = lower_from(program, &mut extra).err().expect("first extra must reject");
    assert_eq!(failure.code, "ZRYNA-N7001", "{case}: actual new native owning gate");
    assert!(extra <= MAX_CODEGEN_UNITS, "no failed debit commits an excess");
    trace::start();
    let mut overflow = u64::MAX;
    let failure = lower_from(program, &mut overflow).err().expect("checked addition overflow");
    let observed = trace::finish();
    assert_eq!(failure.code, "ZRYNA-N7001");
    assert_eq!(overflow, u64::MAX, "failed first debit is atomic");
    assert_eq!(observed.reserves, 0, "overflow rejects before the first native reserve");
    assert!(observed.copies.is_empty());
    let retry = lower(program).expect("fresh actual public issuer after both failures");
    assert_eq!(fingerprint(&retry), expected, "{case}: no partial plan/cache publication");
}

#[test]
fn genuine_two_element_array_repeated_real_copy_callers_pay_each_physical_transport() {
    fixture::with_program("fixed-array-identity", |program| {
        let array = program
            .layouts(StorageTarget::LinuxX8664V1)
            .types()
            .find(|ty| ty.array_length() == Some(2))
            .expect("actual frozen array record");
        assert_eq!((array.size(), array.alignment(), array.array_stride()), (8, 4, Some(4)));
        let expected = copy_actions(program);
        for site in [CopySite::Formal, CopySite::Initialize, CopySite::Return] {
            let count = expected
                .iter()
                .filter(|(kind, ty)| {
                    *kind == site && *ty == raw::ValueType::Stored(array.id().index())
                })
                .count();
            assert_eq!(
                count,
                if site == CopySite::Initialize { 2 } else { 1 },
                "fixed original action inventory"
            );
        }
        for _ in 0..8 {
            trace::start();
            let _mir = lower(program).expect("actual production planner, same genuine owner");
            let observed = trace::finish();
            assert_eq!(observed.copies.len(), expected.len(), "each real Copy caller charges once");
            let mut unmatched = expected.clone();
            for (site, ty, paid) in observed.copies {
                let index = unmatched
                    .iter()
                    .position(|candidate| *candidate == (site, ty))
                    .expect("a genuine source action, no phantom extra charge");
                unmatched.remove(index);
                let bytes = match ty {
                    raw::ValueType::Unit => 0,
                    raw::ValueType::Stored(id) => program
                        .layouts(StorageTarget::LinuxX8664V1)
                        .types()
                        .nth(id as usize)
                        .expect("same-seal logical/physical correspondence")
                        .size(),
                };
                let physical_minimum =
                    bytes.div_ceil(8).checked_mul(64).expect("bounded actual record");
                assert!(
                    paid >= physical_minimum,
                    "{site:?}/{ty:?}: physical copy debit is missing"
                );
            }
            assert!(unmatched.is_empty(), "formal, Initialize and Return call sites all paid");
        }
    });
}
fn copy_actions(program: &VerifiedExecution<'_>) -> Vec<(CopySite, raw::ValueType)> {
    let mut expected = Vec::new();
    for f in program.functions() {
        let mut values = std::collections::BTreeMap::new();
        for parameter in f.parameters() {
            if let VerifiedParameter::Value(value) = parameter {
                values.insert(value.id(), value.ty());
                expected.push((CopySite::Formal, value.ty()));
            }
        }
        for b in f.blocks() {
            for value in
                b.value_parameters().chain(b.operations().filter_map(VerifiedOperation::result))
            {
                values.insert(value.id(), value.ty());
            }
        }
        for b in f.blocks() {
            for op in b.operations() {
                if let raw::Operation::InitializePlace { value, .. }
                | raw::Operation::ReplacePlace { value, .. } = op.kind()
                {
                    expected.push((CopySite::Initialize, values[value]));
                }
            }
            if matches!(b.terminator(), raw::Terminator::Return { .. }) {
                expected.push((CopySite::Return, f.result_type()));
            }
        }
    }
    expected
}
fn fingerprint(program: &VerifiedProgram<'_, '_>) -> Vec<String> {
    let mut rows = Vec::new();
    for f in program.functions() {
        rows.push(format!("{}:{}", f.symbol(), f.frame_bytes()));
        for id in 0.. {
            let Some(record) = f.value(id) else {
                break;
            };
            rows.push(format!("v{id}:{:?}:{}", record.ty(), record.offset()));
        }
        for place in f.source().places() {
            let record = f.place(place.id()).expect("retained exact static projection");
            rows.push(format!("p{}:{:?}:{}", place.id(), record.ty(), record.offset()));
        }
    }
    if let Some(plan) = program.containers() {
        rows.extend(
            plan.clone_symbols().enumerate().map(|(id, name)| format!("clone{id}:{name:?}")),
        );
        rows.extend(plan.drop_symbols().enumerate().map(|(id, name)| format!("drop{id}:{name:?}")));
        rows.push(format!("runtime:{:?}", plan.operations()));
        for id in 0..program.program().type_count() {
            let id = u32::try_from(id).expect("sealed logical type count");
            if let Some(metadata) = plan.element(id) {
                rows.push(format!("element{id}:{metadata:?}"));
            }
            if let Some(metadata) = plan.control(id) {
                rows.push(format!("control{id}:{metadata:?}"));
            }
            if let Some(node) = plan.clone_node(id) {
                rows.push(format!("node{id}:{:?}:{}", node.kind(), node.may_fail()));
            }
        }
    }
    rows
}
