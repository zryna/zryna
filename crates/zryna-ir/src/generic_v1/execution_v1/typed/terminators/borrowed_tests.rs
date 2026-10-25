//! Match schema/metadata/CFG owners, independently of source and public seal construction.
use super::{check, raw};
use crate::generic_v1::execution_v1::{
    places,
    typed::{borrowed_match_test_support as fixture, cfg::Graph},
};
fn run(
    p: &raw::Program,
    w: &crate::generic_v1::execution_v1::type_world::TypeWorld,
) -> Result<(), super::Failure> {
    let f = &p.functions[0];
    let c = fixture::checked(f, w)?;
    check(f, &c, w, 0, &mut 0)
}
#[test]
fn borrowed_closed_heads_preserve_exact_payload_and_inherited_access() {
    for family in 0..4 {
        for access in [raw::Access::Shared, raw::Access::Exclusive] {
            let (p, w) = fixture::input(access, family);
            run(&p, &w).expect("actual borrowed Option/Result/two nominal enum schema");
        }
    }
}
#[test]
fn selected_loan_definition_parent_ordinal_and_payloadless_arm_are_exact() {
    let (p, w) = fixture::input(raw::Access::Shared, 0);
    for mutate in 0..6 {
        let mut wrong = p.clone();
        let f = &mut wrong.functions[0];
        match mutate {
            0 => {
                if let Some(raw::MatchBinding::Loan(d)) = &mut fixture::arms(f)[1].binding {
                    d.id = 0;
                }
            }
            1 => {
                if let Some(raw::MatchBinding::Loan(d)) = &mut fixture::arms(f)[1].binding {
                    d.access = raw::Access::Exclusive;
                }
            }
            2 => {
                f.loans[1].origin = raw::LoanOrigin::StaticProjection {
                    parent: 1,
                    projection: raw::StaticProjection::EnumPayload(1),
                }
            }
            3 => {
                f.loans[1].origin = raw::LoanOrigin::StaticProjection {
                    parent: 0,
                    projection: raw::StaticProjection::EnumPayload(0),
                }
            }
            4 => {
                let binding = fixture::arms(f)[1].binding.clone();
                fixture::arms(f)[0].binding = binding;
            }
            5 => {
                if let raw::Terminator::ClosedEnumMatch { mode, .. } = &mut f.blocks[0].terminator {
                    *mode = raw::MatchMode::ExclusiveLoan;
                }
            }
            _ => unreachable!(),
        }
        assert!(run(&wrong, &w).is_err(), "exact borrowed schema hostile {mutate}");
    }
}
#[test]
fn selected_issuance_is_unique_and_never_an_ordinary_begin_or_value_parameter() {
    let (p, w) = fixture::input(raw::Access::Shared, 1);
    let mut duplicate = p.clone();
    let f = &mut duplicate.functions[0];
    let binding = fixture::arms(f)[0].binding.clone();
    fixture::arms(f)[1].binding = binding;
    assert!(run(&duplicate, &w).is_err(), "one selected loan cannot come from both variants");
    let mut ordinary = p.clone();
    let f = &mut ordinary.functions[0];
    f.blocks[1].instructions.insert(
        0,
        fixture::instruction(raw::Operation::BeginLoan { loan: 1, failure: None }, None),
    );
    let c = Graph::build(f, w.len(), &mut 0).expect("coherent ordinary instruction CFG");
    let values = c.types().collect::<Vec<_>>();
    assert!(
        places::check(f, &values, &w, &mut 0).is_err(),
        "selected authority cannot also begin at an instruction"
    );
    let mut value = p.clone();
    value.functions[0].blocks[1].value_parameters.push(fixture::value(2, 0));
    assert!(
        Graph::build(&value.functions[0], w.len(), &mut 0).is_err(),
        "Loan edge skips zero ordinary parameters"
    );
}
#[test]
fn selected_reads_keep_exact_lexical_site_and_cannot_begin_in_an_instruction() {
    let (p, _) = fixture::input(raw::Access::Shared, 0);
    let f = &p.functions[0];
    let read = fixture::instruction(raw::Operation::ReadLoan { loan: 1 }, None);
    super::super::loan_sites::instruction(f, &read, &mut 0)
        .expect("selected loan exact arm expression span");
    for span in [
        fixture::span(19, 26),
        fixture::span(25, 41),
        zryna_source::UntrustedSpan { file: 1, start: 25, end: 26 },
    ] {
        let mut wrong = read.clone();
        wrong.site_span = span;
        assert!(super::super::loan_sites::instruction(f, &wrong, &mut 0).is_err());
    }
    let begin = fixture::instruction(raw::Operation::BeginLoan { loan: 1, failure: None }, None);
    assert!(super::super::loan_sites::instruction(f, &begin, &mut 0).is_err());
}
#[test]
fn selected_metadata_owner_exact_extra_overflow_and_retry_are_atomic() {
    let (p, w) = fixture::input(raw::Access::Shared, 1);
    let f = &p.functions[0];
    let graph =
        Graph::build(f, w.len(), &mut 0).expect("ordinary CFG values separate from arm loans");
    let values = graph.types().collect::<Vec<_>>();
    let mut cost = 0;
    places::check(f, &values, &w, &mut cost).expect("actual selected metadata owner cost");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    places::check(f, &values, &w, &mut exact).expect("exact selected inventory debit");
    assert_eq!(exact, limit);
    for mut work in [limit - cost + 1, usize::MAX] {
        let before = work;
        assert!(places::check(f, &values, &w, &mut work).is_err());
        assert_eq!(work, before);
    }
    let mut retry = 0;
    places::check(f, &values, &w, &mut retry).expect("pristine selected metadata retry");
    assert_eq!(retry, cost);
}
