//! Genuine source seal and actual record-inspection oracles, fixed before indexing.
use super::{Failure, VerifiedOwnedProgram, raw, wire};
use std::cell::Cell;
use zryna_layout::generic_v1::VerifiedLayouts;
use zryna_ownership_runtime_abi::generic_v1::VerifiedOwnershipRuntimeAbi;

#[path = "../../../tests/generic_struct_v4/fixture.rs"]
mod fixture;
use fixture::Fixture;

const SMALL_INDEX_WORK: usize = 13;
thread_local! {
    static INSPECTED: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn inspect_candidate() {
    INSPECTED.set(INSPECTED.get().checked_add(1).expect("bounded test inspection count"));
}

fn observed<'p>(
    program: &'p VerifiedOwnedProgram<'_>,
    function: usize,
    result: u32,
) -> (Option<&'p raw::Operation>, usize) {
    INSPECTED.set(0);
    let operation = program.extension(function, result);
    (operation, INSPECTED.get())
}

pub(super) struct Authorities<'a> {
    fixture: &'a Fixture,
    pub(super) linear: &'a VerifiedLayouts,
    linux: &'a VerifiedLayouts,
    runtime: &'a VerifiedOwnershipRuntimeAbi<'a>,
}
impl<'a> Authorities<'a> {
    pub(super) fn seal(&self, claim: &raw::Program) -> Result<VerifiedOwnedProgram<'a>, Failure> {
        super::verify_v4(
            wire::v4::decode(&wire::v4::encode(claim).expect("physically bounded raw claim"))?,
            &self.fixture.syntax,
            &self.fixture.sources,
            self.fixture.sources.verify_file_id(0).expect("original entry"),
            self.linear,
            self.linux,
            self.runtime,
        )
    }
}

pub(super) fn with_claim(test: impl FnOnce(&Authorities<'_>, &raw::Program)) {
    let fixture = Fixture::new("construction", true);
    let (linear, linux) = fixture.layouts();
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux)
            .expect("independent runtime claims"),
        &linear,
        &linux,
    )
    .expect("independently sealed runtime authority");
    let claim = super::produce_claim_v4(&fixture.syntax, &fixture.sources, &linear, &linux, &[])
        .expect("actual original Box construction, never a fabricated map authority");
    test(
        &Authorities { fixture: &fixture, linear: &linear, linux: &linux, runtime: &runtime },
        &claim,
    );
}

fn literal_dimensions(claim: &raw::Program) {
    assert_eq!(claim.graph.functions.len(), 1);
    let function = &claim.graph.functions[0];
    assert_eq!(function.blocks.len(), 1);
    assert!(function.blocks[0].parameters.is_empty());
    assert_eq!(function.blocks[0].instructions.len(), 3);
    assert_eq!(claim.extensions.len(), 1);
    assert_eq!(claim.extensions[0].len(), 2);
    assert_eq!(claim.extensions[0][0].result, 1);
    assert_eq!(claim.extensions[0][1].result, 2);
    // Independent contract census: 2F + 3B + 2D + E = 2 + 3 + 6 + 2.
    assert_eq!(2 + 3 + 2 * 3 + 2, SMALL_INDEX_WORK);
}

#[test]
fn genuine_box_seal_retains_exact_answers_and_fixed_thirteen_unit_dimensions() {
    with_claim(|authorities, claim| {
        literal_dimensions(claim);
        let program = authorities.seal(claim).expect("genuine v4 source/type/ownership seal");
        assert_eq!(
            observed(&program, 0, 1),
            (Some(&raw::Operation::StructConstruct { ty: 3, fields: vec![0] }), 1),
        );
        assert_eq!(
            program.extension(0, 2),
            Some(&raw::Operation::CopyStructField { value: 1, ordinal: 0 }),
        );
        for function in [1, usize::MAX] {
            assert_eq!(observed(&program, function, 2), (None, 0));
        }
        let foreign = authorities.fixture.fresh();
        let bytes = wire::v4::encode(claim).expect("unchanged raw replay");
        assert!(
            super::verify_v4(
                wire::v4::decode(&bytes).expect("raw decoded replay"),
                &foreign.syntax,
                &foreign.sources,
                foreign.sources.verify_file_id(0).expect("foreign entry"),
                authorities.linear,
                authorities.linux,
                authorities.runtime,
            )
            .is_err(),
            "equal source text cannot install foreign source/layout authority"
        );
    });
}

#[test]
fn genuine_v4_last_extension_lookup_inspects_one_retained_operation() {
    with_claim(|authorities, claim| {
        literal_dimensions(claim);
        let program = authorities.seal(claim).expect("independent v4 issuance");
        assert_eq!(
            observed(&program, 0, 2),
            (Some(&raw::Operation::CopyStructField { value: 1, ordinal: 0 }), 1),
            "the second extension must not scan both retained operation records",
        );
    });
}

#[test]
fn genuine_v4_absent_extension_lookup_inspects_no_operation_record() {
    with_claim(|authorities, claim| {
        literal_dimensions(claim);
        let program = authorities.seal(claim).expect("independent v4 issuance");
        for result in [0, 3, u32::MAX] {
            assert_eq!(
                observed(&program, 0, result),
                (None, 0),
                "a core-only or out-of-range result must not scan operation records",
            );
        }
    });
}

#[test]
fn hostile_extension_ids_never_acquire_source_or_typed_index_authority() {
    with_claim(|authorities, claim| {
        for attack in 0..4 {
            let mut hostile = claim.clone();
            match attack {
                0 => {
                    let duplicate = hostile.extensions[0][0].clone();
                    hostile.extensions[0].insert(1, duplicate);
                }
                1 => hostile.extensions[0][1].result = 3,
                2 => hostile.extensions[0][1].result = u32::MAX,
                _ => {
                    hostile.extensions[0][1].operation =
                        raw::Operation::CopyStructField { value: 1, ordinal: u32::MAX }
                }
            }
            assert!(authorities.seal(&hostile).is_err(), "raw hostile case {attack}");
            let pristine =
                authorities.seal(claim).expect("rejection cannot contaminate next issuance");
            assert_eq!(pristine.extension(0, 2), Some(&claim.extensions[0][1].operation));
        }
    });
}

#[test]
fn genuine_issuer_index_uses_the_same_ten_spent_plan_credits_and_commits_twenty_three() {
    with_claim(|authorities, claim| {
        let old = super::plan::derive(&claim.graph, &claim.extensions, authorities.linear)
            .expect("unchanged original replay");
        let (plans, spent) =
            super::plan::derive_counted(&claim.graph, &claim.extensions, authorities.linear)
                .expect("same invocation returns its actual ledger");
        assert_eq!(plans, old);
        assert_eq!(plans, claim.plans);
        // Independent source census: topology8, one field transfer1, return step1.
        assert_eq!(spent, 10);
        super::extension_index::statistics::reset();
        let program = authorities.seal(claim).expect("real v4 issuer, not a raw map");
        assert!(program.extension_index.is_some());
        assert_eq!(
            super::extension_index::statistics::observed(),
            (Some(10), Some(23), 2),
            "the actual issuer must thread prior10 plus fixed13 before two reserves",
        );
    });
}

#[test]
fn genuine_v4_pass_only_graphs_still_issue_empty_effect_indexes() {
    for (name, generic) in [("pass-generic", true), ("pass-plain", false)] {
        let fixture = Fixture::new(name, generic);
        let (linear, linux) = fixture.layouts();
        let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
            zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("claims"),
            &linear,
            &linux,
        )
        .expect("runtime");
        let claim =
            super::produce_claim_v4(&fixture.syntax, &fixture.sources, &linear, &linux, &[])
                .expect("genuine pass-only source");
        assert!(claim.extensions.iter().all(Vec::is_empty));
        let authorities =
            Authorities { fixture: &fixture, linear: &linear, linux: &linux, runtime: &runtime };
        let program = authorities.seal(&claim).expect("v4 pass-only typed seal");
        assert!(program.extension_index.is_some());
        assert_eq!(observed(&program, 0, 0), (None, 0));
    }
}
