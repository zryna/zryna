//! ABI selection controls use literal source authority and untrusted new-domain rows only.

use super::*;
use crate::generic_v1::source_abi::tests::fixture::Fixture;

fn claim(fixture: &Fixture, parameter: u32, result: u32) -> raw::Program {
    let original = &fixture.syntax.files()[0].functions[0];
    raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![raw::Module { id: 0, functions: 1 }],
        declarations: vec![raw::Declaration {
            module: 0,
            function: 0,
            parameters: 0,
            span: original.span,
        }],
        type_keys: fixture.linear.types().map(|ty| ty.key().to_vec()).collect(),
        universe: *fixture.linear.universe_identity(),
        linear32: *fixture.linear.fingerprint(),
        linux_x86_64: *fixture.linux.fingerprint(),
        entry_module: 0,
        functions: vec![raw::Function {
            key: vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0],
            original_span: original.span,
            executable_export: Some("value".into()),
            parameters: vec![raw::Parameter::Value(raw::ValueDefinition {
                id: 0,
                ty: raw::ValueType::Stored(parameter),
            })],
            result_type: raw::ValueType::Stored(result),
            places: vec![],
            loans: vec![],
            blocks: vec![],
            cleanup_plans: vec![],
        }],
    }
}

fn run(
    fixture: &Fixture,
    program: &raw::Program,
    spent: &mut usize,
) -> Result<(zryna_abi::VerifiedScalarAbiModule, Vec<usize>), Failure> {
    derive(
        program,
        &fixture.syntax,
        &fixture.sources,
        fixture.sources.verify_file_id(0).expect("literal entry"),
        spent,
    )
}

#[test]
fn original_bool_and_i32_export_names_and_types_are_exact() {
    for (name, id) in [("bool", 0), ("i32", 1)] {
        let fixture = Fixture::new(name, name);
        let (abi, indices) = run(&fixture, &claim(&fixture, id, id), &mut 0).expect("scalar root");
        assert_eq!(indices, [0]);
        assert_eq!(abi.exports().len(), 1);
        assert_eq!(abi.exports().next().expect("export").logical_name().as_str(), "value");
    }
}

#[test]
fn original_aggregate_boundary_owns_m7005_even_if_raw_signature_claims_scalar() {
    for (parameter, result) in
        [("String", "String"), ("String", "i32"), ("i32", "String"), ("Option<i32>", "Option<i32>")]
    {
        let fixture = Fixture::new(parameter, result);
        let failure =
            run(&fixture, &claim(&fixture, 1, 1), &mut 0).expect_err("source excludes ABI");
        let Failure::Diagnostics(errors) = failure else { panic!("source diagnostic") };
        assert_eq!(errors[0].code(), "ZRYNA-M7005");
        let span = errors[0].primary_span().expect("exported source name");
        assert_eq!((span.start(), span.end()), (16, 21));
    }
}

#[test]
fn hidden_renamed_extra_template_and_wrong_scalar_exports_reject() {
    let fixture = Fixture::new("i32", "i32");
    let pristine = claim(&fixture, 1, 1);
    for attack in 0..6 {
        let mut changed = pristine.clone();
        match attack {
            0 => changed.functions[0].executable_export = None,
            1 => changed.functions[0].executable_export = Some("forged".into()),
            2 => changed.functions.push(changed.functions[0].clone()),
            3 => changed.functions[0].key[0] = 0x40,
            4 => changed.functions[0].result_type = raw::ValueType::Stored(0),
            _ => changed.entry_module = 1,
        }
        assert!(run(&fixture, &changed, &mut 0).is_err(), "attack {attack}");
    }
    run(&fixture, &pristine, &mut 0).expect("pristine retry");
}

#[test]
fn exact_complete_selection_credit_and_first_extra_are_atomic() {
    let fixture = Fixture::new("i32", "i32");
    let program = claim(&fixture, 1, 1);
    let mut cost = 0;
    run(&fixture, &program, &mut cost).expect("cold complete cost");
    let mut exact = super::super::budget::MAX_OWNER_UNITS - cost;
    run(&fixture, &program, &mut exact).expect("exact complete credit");
    assert_eq!(exact, super::super::budget::MAX_OWNER_UNITS);
    let mut short = super::super::budget::MAX_OWNER_UNITS - cost + 1;
    let before = short;
    run(&fixture, &program, &mut short).expect_err("first extra complete work");
    assert_eq!(short, before);
    run(&fixture, &program, &mut 0).expect("unmodified retry");
}

#[test]
fn independently_derived_execution_selection_credit_is_required_before_scalar_issuance() {
    // Selected1 + raw export/index2 + keycmp10 + namecmp5 + parameter slot1
    // + source parameter4 + claimed parameter1 + source result4 + claimed result1
    // + raw name copy5 + extra-export scan1. Independent scalar ABI work is a
    // separate inherited owner; this literal tests the execution-owned ledger.
    const EXECUTION_SELECTION: usize = 1 + 2 + 10 + 5 + 1 + 4 + 1 + 4 + 1 + 5 + 1;
    let fixture = Fixture::new("i32", "i32");
    let program = claim(&fixture, 1, 1);
    let mut cold = 0;
    run(&fixture, &program, &mut cold).expect("independent scalar ABI issuance");
    assert_eq!(cold, EXECUTION_SELECTION);
    let mut exact = super::super::budget::MAX_OWNER_UNITS - EXECUTION_SELECTION;
    let (abi, indices) = run(&fixture, &program, &mut exact).expect("exact selection credit");
    assert_eq!(exact, super::super::budget::MAX_OWNER_UNITS);
    assert_eq!(abi.exports().len(), 1);
    assert_eq!(indices, [0]);
    // One missing execution-owned unit prevents publication. The literal cold
    // assertion also detects silently waived selected/raw/index retention.
    let mut short = super::super::budget::MAX_OWNER_UNITS - EXECUTION_SELECTION + 1;
    let before = short;
    let Failure::Diagnostics(errors) =
        run(&fixture, &program, &mut short).expect_err("no scalar ABI on short execution credit")
    else {
        panic!("owning terminal resource diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-I3201");
    assert_eq!(short, before, "no completed caller debit or partial output");
    run(&fixture, &program, &mut 0).expect("pristine retry");
}
