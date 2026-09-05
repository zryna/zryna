use super::*;

const JOIN: &str =
    "ownership, initialization, or active-enum state differs across a CFG join or backedge";

#[test]
fn copy_enum_join_omitting_restoration_rejects_unequal_arm_refinements() {
    let fixture = Fixture::new();
    let mut raw = fixture.seed();
    raw.modules[0].functions[0].blocks[1].instructions.clear();
    raw.modules[0].functions[0].blocks[2].instructions.clear();
    fixture.reject(raw, "ZRYNA-I3010", Some(JOIN));
}

#[test]
fn copy_enum_join_write_requires_its_active_exclusive_authority_and_exact_type() {
    let fixture = Fixture::new();
    for mutation in 0..5 {
        let mut raw = fixture.seed();
        let instructions = &mut raw.modules[0].functions[0].blocks[1].instructions;
        let code = match mutation {
            0 => {
                instructions.swap(1, 2);
                "ZRYNA-I3011"
            }
            1 => {
                if let raw::InstructionKind::BeginBorrow(definition) = &mut instructions[0].kind {
                    definition.access = raw::BorrowAccess::Shared;
                }
                "ZRYNA-I3005"
            }
            2 => {
                if let raw::InstructionKind::BorrowWrite { borrow, .. } = &mut instructions[1].kind
                {
                    *borrow = raw::BorrowId(1);
                }
                "ZRYNA-I3011"
            }
            3 => {
                if let raw::InstructionKind::BorrowWrite { value, .. } = &mut instructions[1].kind {
                    *value = raw::ValueId(1);
                }
                "ZRYNA-I3005"
            }
            4 => {
                if let raw::InstructionKind::BeginBorrow(definition) = &mut instructions[0].kind {
                    definition.place = raw::PlaceId(2);
                }
                "ZRYNA-I3010"
            }
            _ => unreachable!(),
        };
        fixture.reject(raw, code, (mutation == 4).then_some(JOIN));
    }
}

#[test]
fn copy_enum_join_restoration_transition_preflight_exact_extra_and_overflow() {
    let fixture = Fixture::new();
    let mut raw = fixture.seed();
    fixture.verify(raw.clone()).expect("authenticated seed");
    // Counter controls exercise the actual preflight, not a full repeated-write program.
    let instruction = raw.modules[0].functions[0].blocks[1].instructions[1].clone();
    raw.modules[0].functions[0].blocks[1]
        .instructions
        .resize(MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 5, instruction.clone());
    let mut exact = Errors::default();
    super::super::super::preflight(&raw, &fixture.linear, &mut exact);
    assert!(exact.is_empty());
    raw.modules[0].functions[0].blocks[1].instructions.push(instruction);
    let mut extra = Errors::default();
    super::super::super::preflight(&raw, &fixture.linear, &mut extra);
    let errors = extra.finish();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-I3201");
    let mut overflow = Errors::default();
    assert_eq!(
        super::super::super::checked_add(usize::MAX, 1, "Copy enum restoration", &mut overflow),
        usize::MAX
    );
    assert_eq!(overflow.finish()[0].code(), "ZRYNA-I3201");
    fixture.verify(fixture.seed()).expect("valid recovery after resource rejection");
}
