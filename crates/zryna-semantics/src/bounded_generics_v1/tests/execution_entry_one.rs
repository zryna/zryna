//! Actual module1 entry selects the new source owner and issuer, never implicit module0.
use crate::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{execution_v1, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_ir::generic_v1::execution_v1 as execution;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
mod source;
const INSTANCE: &[u8] = &[0x40, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1];
const ENTRY: &[u8] = &[0x41, 1, 0, 0, 0, 0, 0, 0, 0];
#[test]
fn both_actual_complete_packets_authenticate_the_same_two_original_files() {
    let (native_sources, native) = source::project("native");
    let (ts_sources, typescript) = source::project("typescript");
    assert!(native.is_bound_to(&native_sources));
    assert!(typescript.is_bound_to(&ts_sources));
    assert_eq!(native.files(), typescript.files());
    assert_eq!(native.files().len(), 2);
    assert_eq!(native.files()[0].functions[0].name.text, "identity");
    assert_eq!(native.files()[1].functions[0].name.text, "entry");
}
#[test]
fn genuine_entry_one_produces_and_issues_only_main_scalar_export() {
    for provider in ["native", "typescript"] {
        let (sources, syntax) = source::project(provider);
        let entry = source::entry(&sources);
        assert_eq!(entry.index(), 1);
        let input = SemanticInput::try_new(&syntax, &sources, entry).expect("genuine entry1 input");
        assert_eq!(input.entry(), entry);
        let declarations =
            resolve_declarations(input).expect("complete original declarations/imports");
        assert_eq!(declarations.entry(), entry);
        let bodies = check_body_types(&declarations).expect("original symbolic body check");
        let instances = execution_v1::discover(&bodies).expect("original before closed discovery");
        assert_eq!(instances.instances().function_keys().collect::<Vec<_>>(), vec![INSTANCE]);
        let linear =
            verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
        let linux =
            verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("Linux");
        let claim = execution_v1::produce_claim(&instances, &linear, &linux)
            .expect("bridge forwards retained entry1");
        assert_eq!(claim.entry_module, 1);
        let direct =
            execution::produce_claim(&syntax, &sources, entry, &linear, &linux, &[INSTANCE])
                .expect("explicit genuine entry1 source API");
        assert_eq!(claim, direct);
        check_claim_inventory(&claim);
        let runtime = runtime::verify_v1(
            runtime::raw_v1(&linear, &linux).expect("runtime raw"),
            &linear,
            &linux,
        )
        .expect("real runtime declaration authority");
        let encoded = execution::wire::encode(&claim).expect("full execution wire");
        execution::verify(
            execution::wire::decode(&encoded).expect("new carrier"),
            &syntax,
            &sources,
            entry,
            &linear,
            &linux,
            &runtime,
        )
        .expect("NEW issuer binds actual module1 entry");
        let mut wrong = claim.clone();
        wrong.entry_module = 0;
        let wrong = execution::wire::encode(&wrong).expect("bounded hostile raw entry claim");
        assert!(
            execution::verify(
                execution::wire::decode(&wrong).expect("untrusted mutated entry"),
                &syntax,
                &sources,
                entry,
                &linear,
                &linux,
                &runtime
            )
            .is_err()
        );
        let foreign_sources = source::sources();
        let foreign_entry = source::entry(&foreign_sources);
        assert_eq!(foreign_entry.index(), entry.index());
        assert!(SemanticInput::try_new(&syntax, &sources, foreign_entry).is_none());
        assert!(
            execution::produce_claim(
                &syntax,
                &sources,
                foreign_entry,
                &linear,
                &linux,
                &[INSTANCE]
            )
            .is_err()
        );
        assert!(
            execution::verify(
                execution::wire::decode(&encoded).expect("fresh pristine carrier"),
                &syntax,
                &sources,
                foreign_entry,
                &linear,
                &linux,
                &runtime
            )
            .is_err()
        );
        execution::verify(
            execution::wire::decode(&encoded).expect("pristine retry"),
            &syntax,
            &sources,
            entry,
            &linear,
            &linux,
            &runtime,
        )
        .expect("original authority remains usable after all attacks");
    }
}

fn check_claim_inventory(claim: &execution::raw::Program) {
    assert_eq!(
        claim
            .functions
            .iter()
            .filter(|function| function.key.first() == Some(&0x40))
            .map(|function| function.key.as_slice())
            .collect::<Vec<_>>(),
        vec![INSTANCE]
    );
    assert_eq!(
        claim
            .functions
            .iter()
            .filter_map(|function| function
                .executable_export
                .as_deref()
                .map(|name| (function.key.as_slice(), name)))
            .collect::<Vec<_>>(),
        vec![(ENTRY, "entry")]
    );
}
