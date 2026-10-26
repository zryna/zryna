//! Actual source entry and canonical kernel; modified private projection fails production byte audit.
use super::*;
use crate::generic_execution_v1::{self as target, Mode, accounting_tests, runtime::ordinary};
#[test]
fn aggregate_children_and_partial_cleanup_share_real_source_invocation_debt() {
    for record in oracle()["full_source"].as_array().expect("unchanged qualified source rows") {
        let case = record["case"].as_str().expect("case");
        for provider in catalogue::PROVIDERS {
            source::with_program(case, provider, |program| {
                let mut context = Context::new(program).expect("exact genuine owner");
                context.accounting =
                    accounting::Plan::derive(&context).expect("actual paid effect census");
                assert!(context.accounting.as_ref().is_some_and(|plan| plan.bound(&context)));
                let entry = program.export_functions();
                assert_eq!(entry.len(), 1);
                let function =
                    program.functions().nth(entry[0]).expect("retained exported function");
                assert_eq!(function.parameters().len(), 0, "unchanged zero-argument finite source");
                let mut exports = program.scalar_abi().exports();
                let export = exports.next().expect("export");
                assert!(exports.next().is_none());
                let artifact = target::emit(program).expect("ordinary genuine aggregate artifact");
                let mode =
                    Mode::Ordinary(accounting::quota().expect("fixed accepted runtime bounds"));
                ordinary::audit(&context, &artifact.source, mode)
                    .expect("actual complete public bytes accepted");
                let script = format!(
                    "{}\nconst $aggregateRecord={},$aggregateEntry=$g{},$aggregatePublic=$export{};\n{}",
                    artifact.source,
                    record,
                    function.index(),
                    export.index(),
                    include_str!("private_source.mjs")
                );
                assert!(
                    ordinary::audit(&context, &script, mode).is_err(),
                    "private entry/counter projection is not production authority"
                );
                let receipt = accounting_tests::private_boundary(
                    &format!("aggregate-source-{case}-{provider}"),
                    &script,
                );
                assert_eq!(receipt["case"], case);
                assert_eq!(receipt["actualSourceAndCanonicalKernel"], true);
                assert_eq!(
                    receipt["observations"].as_array().expect("actual observations").len(),
                    record["rows"].as_array().expect("frozen rows").len()
                );
                assert_eq!(receipt["productionAuditAcceptsProjection"], false);
            });
        }
    }
}
