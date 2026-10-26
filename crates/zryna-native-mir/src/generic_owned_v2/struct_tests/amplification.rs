//! Admitted flat Struct amplification; fixed before source qualification or target observation.
use super::{StorageTarget, lower, owned, with_source};
use zryna_ir::generic_v1::raw;

const FIELDS: u64 = 32;
const STRESS: u64 = 1024;
const STRING_INSTRUCTIONS: u64 = 6; // stack_addr, three stores, call, trapnz.

fn size(value: usize) -> u64 {
    u64::try_from(value).expect("bounded sealed count fits u64")
}
fn value_type(function: &raw::Function, id: u32) -> raw::Type {
    function
        .blocks
        .iter()
        .flat_map(|block| {
            block
                .parameters
                .iter()
                .chain(block.instructions.iter().map(|instruction| &instruction.result))
        })
        .find(|definition| definition.id == id)
        .expect("sealed definition")
        .ty
}
fn wide(program: &owned::VerifiedOwnedProgram<'_>, ty: raw::Type) -> bool {
    let raw::Type::Stored(index) = ty else {
        return false;
    };
    program
        .layouts(StorageTarget::LinuxX8664V1)
        .types()
        .nth(index as usize)
        .is_some_and(|view| matches!(view.key().first(), Some(0x10 | 0x12)))
}
fn destructor_census(program: &owned::VerifiedOwnedProgram<'_>) -> u64 {
    let mut count = 0;
    for (index, function) in program.functions().iter().enumerate() {
        for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
            if let Some(owned::raw::Operation::Drop(value)) =
                program.extension(index, instruction.result.id)
            {
                count += u64::from(wide(program, value_type(function, *value)));
            }
        }
        for step in &program.plan(index).expect("exact original cleanup plan").steps {
            for value in &step.cleanup {
                count += u64::from(wide(program, value_type(function, *value)));
            }
        }
    }
    count
}
// Independent diagnostic comparison with the previous fixed charging SCHEME, not a claim
// that the old issuer admitted Struct. Include every initial constructor failure prefix.
fn old_fixed_charge(program: &owned::VerifiedOwnedProgram<'_>) -> u64 {
    let views = program.layouts(StorageTarget::LinuxX8664V1).types().collect::<Vec<_>>();
    let widths = views
        .iter()
        .map(|view| match view.key().first() {
            Some(0 | 1) => 1,
            Some(2) => 3,
            Some(0x10 | 0x12) => {
                assert_eq!(size(view.fields().len()), FIELDS);
                assert!(
                    view.fields().all(|(_, field, _)| views[field.index() as usize].key() == [2])
                );
                FIELDS * 3
            }
            _ => panic!("only flat scalar/String/Struct witness types"),
        })
        .collect::<Vec<_>>();
    let width = |ty| match ty {
        raw::Type::Unit => 0,
        raw::Type::Stored(index) => widths[index as usize],
        raw::Type::Borrow { .. } => panic!("no loans in the amplification witness"),
    };
    let mut units = 0;
    for (index, function) in program.functions().iter().enumerate() {
        for block in &function.blocks {
            units += 16;
            for definition in
                block.parameters.iter().chain(block.instructions.iter().map(|i| &i.result))
            {
                units += 4 * width(definition.ty);
            }
            for instruction in &block.instructions {
                let operands = match &instruction.operation {
                    raw::Operation::ClosedGenericCall { arguments, .. }
                    | raw::Operation::SourceCall { arguments, .. } => arguments.len(),
                    _ => 1,
                };
                units += size(operands.max(1)) * 256;
                if let Some(owned::raw::Operation::StringLiteral(bytes)) =
                    program.extension(index, instruction.result.id)
                {
                    units += size(bytes.len()) * 64;
                }
            }
            assert!(matches!(block.terminator, raw::Terminator::Return(_)), "flat original CFG");
        }
        for step in &program.plan(index).expect("exact original plan").steps {
            units += size(step.cleanup.len()) * 256;
        }
    }
    units
}

#[test]
fn admitted_flat_struct_cleanup_amplification_is_charged_before_native_emission() {
    assert_eq!(FIELDS * STRING_INSTRUCTIONS * (STRESS + 1), 196_800);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/generic_owned_v2/struct_tests/fixtures");
    for scopes in [1, STRESS] {
        let source = std::fs::read_to_string(root.join(format!("wide-{scopes}.zry")))
            .expect("frozen flat Struct source");
        let bytes = std::fs::read(root.join(format!("wide-{scopes}.json")))
            .expect("frozen complete raw DTO");
        // This helper requires all original semantic, layout, runtime and v4 IR authorities
        // FIRST. An earlier source/ownership/resource failure is a test failure, not N7001 credit.
        with_source(source, &bytes, |program| {
            let old = old_fixed_charge(program);
            assert!(old < super::super::MAX_CODEGEN_UNITS, "old fixed scheme: {old}");
            let count = destructor_census(program);
            assert_eq!(count, scopes + 1, "every failure cleanup and normal whole drop");
            let minimum = FIELDS * STRING_INSTRUCTIONS * count;
            if scopes == STRESS {
                assert!(old + minimum > super::super::MAX_CODEGEN_UNITS);
            }
            let result = lower(program);
            if scopes == 1 {
                result.expect("one flat Struct and one transient scope fit unchanged native caps");
            } else {
                let Err(error) = result else {
                    panic!("native MIR omitted repeated real String destructor work");
                };
                assert_eq!(error.code(), "ZRYNA-N7001");
            }
        });
    }
}
