//! Second-child denial must unwind completed child before the caller's original array.
use super::*;
use crate::generic_execution_v1::{self as target, Mode, accounting_tests, runtime::ordinary};
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
#[test]
fn genuine_array_clone_exact_and_first_extra_unwind_actual_held_prefix_and_retry() {
    for provider in catalogue::PROVIDERS {
        source::with_program("referent-catalogue", provider, |program| {
            let mut context = Context::new(program).expect("same genuine whole source seal");
            context.accounting =
                accounting::Plan::derive(&context).expect("paid actual operation census");
            assert!(context.accounting.as_ref().is_some_and(|plan| plan.bound(&context)));
            let clone = program
                .operations()
                .filter_map(zryna_ir::generic_v1::execution_v1::VerifiedOperation::clone_frontier)
                .find(|clone| {
                    matches!(clone.frontier().kind(), CloneNodeKind::FixedArray { length: 2 })
                        && clone
                            .frontier()
                            .element()
                            .is_some_and(|child| child.kind() == CloneNodeKind::String)
                })
                .expect("actual original two-element String array Clone operation");
            let id = clone.ty();
            let artifact = target::emit(program).expect("actual audited aggregate module");
            let mode = Mode::Ordinary(accounting::quota().expect("canonical fixed quotas"));
            ordinary::audit(&context, &artifact.source, mode)
                .expect("genuine public source bytes accepted");
            let script = format!(
                "{}\nconst $aggregatePrefixRows={},$aggregateArrayClone=$clone{id},$aggregateArrayDrop=$drop{id};\n{}",
                artifact.source,
                oracle()["array_prefix"],
                include_str!("prefix.mjs")
            );
            assert!(
                ordinary::audit(&context, &script, mode).is_err(),
                "private helper/counter projection cannot publish production authority"
            );
            let receipt = accounting_tests::private_boundary(
                &format!("aggregate-prefix-{provider}"),
                &script,
            );
            assert_eq!(receipt["actualRetainedArrayCloneDrop"], true);
            assert_eq!(receipt["actualCanonicalKernel"], true);
            assert_eq!(receipt["observations"].as_array().expect("actual fixed rows").len(), 5);
            assert_eq!(receipt["productionAuditAcceptsProjection"], false);
        });
    }
}
