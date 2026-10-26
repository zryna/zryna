//! Source-first lower bound: short nominal keys cannot hide expanded native cleanup work.
//!
//! The original 512-drop witness exhausted source specialization credit (M7201) before the
//! owned issuer, so it could not test native preflight. This refinement is frozen before native
//! production: fewer drops use the existing block weight and statically required instructions.
use std::fmt::Write;
use zryna_ir::generic_v1::{owned_v2, raw};
use zryna_layout::{StorageTarget, TypeCategory};

const MEMBERS: usize = 1_024;
const DROPS: usize = 24;
// effects::drop_value creates two blocks per variant; plan::function charges 16 per block.
// Each variant also emits icmp/brif/jump. Its String destructor emits stack_addr, three stores,
// call and trapnz. The shared done block and terminal trap only increase this lower bound.
const BLOCK_UNITS: u64 = 16;
const MINIMUM_VARIANT_UNITS: u64 = 2 * BLOCK_UNITS + 3 + 6;

fn source(drops: usize) -> String {
    assert_eq!(MEMBERS, zryna_syntax::v4::MAX_MEMBERS_PER_DECLARATION);
    const { assert!(MEMBERS <= zryna_layout::MAX_MEMBERS_PER_DECLARATION) };
    assert!(drops + 1 < zryna_syntax::v4::MAX_BLOCKS_PER_FUNCTION);
    assert!(drops * 2 + 1 < zryna_syntax::v4::MAX_STATEMENTS_PER_FUNCTION);
    assert!(drops * 2 + 1 < zryna_syntax::v4::MAX_EXPRESSIONS_PER_FUNCTION);
    assert!(MEMBERS + drops * 4 + 2 < zryna_syntax::v4::MAX_TYPE_NODES_PER_MODULE);
    let mut text = String::from("interface Many<T extends ZrynaValue> extends ZrynaEnum {");
    for ordinal in 0..MEMBERS {
        write!(text, "v{ordinal}: T;").expect("write fixed member into String");
    }
    text.push_str("} export function stress(input:i32):i32 {");
    for _ in 0..drops {
        text.push_str("{ const owner:Many<String> =Many.v0<String>(\"x\"); }");
    }
    text.push_str("return input; }");
    assert!(text.len() < zryna_source::MAX_SOURCE_FILE_BYTES);
    text
}

// Diagnostic comparison with the existing fixed-charge scheme, not an expected target result.
// Its actual sealed graph must fit that scheme before the new amplification rejection is useful.
fn old_fixed_charge(program: &owned_v2::VerifiedOwnedProgram<'_>) -> u64 {
    let widths: Vec<_> = program
        .layouts(StorageTarget::LinuxX8664V1)
        .types()
        .map(|view| match view.category() {
            TypeCategory::Bool | TypeCategory::I32 => 1,
            TypeCategory::String => 3,
            TypeCategory::Enum => {
                assert_eq!(view.variants().len(), MEMBERS);
                assert!(view.key().len() < 32, "complete nominal key stays short");
                4
            }
            _ => panic!("unexpected source type"),
        })
        .collect();
    let width = |ty| match ty {
        raw::Type::Unit => 0,
        raw::Type::Stored(index) => widths[index as usize],
        raw::Type::Borrow { .. } => panic!("stress source has no loans"),
    };
    let mut units = 0_u64;
    for (index, function) in program.functions().iter().enumerate() {
        for block in &function.blocks {
            units += 16;
            for definition in
                block.parameters.iter().chain(block.instructions.iter().map(|i| &i.result))
            {
                units += width(definition.ty) * 4;
            }
            for instruction in &block.instructions {
                assert!(!matches!(
                    instruction.operation,
                    raw::Operation::ClosedGenericCall { .. } | raw::Operation::SourceCall { .. }
                ));
                units += 256;
                if let Some(owned_v2::raw::Operation::StringLiteral(bytes)) =
                    program.extension(index, instruction.result.id)
                {
                    units += bytes.len() as u64 * 64;
                }
            }
            assert!(
                matches!(block.terminator, raw::Terminator::Return(_)),
                "disjoint lexical scopes remain straight-line source replay"
            );
        }
        for step in &program.plan(index).expect("sealed ownership plan").steps {
            units += step.cleanup.len() as u64 * 256;
        }
    }
    units
}

#[test]
fn source_bound_nominal_drop_expansion_rejects_before_native_ir_generation() {
    let minimum = (MEMBERS as u64) * (DROPS as u64) * MINIMUM_VARIANT_UNITS;
    assert_eq!(MINIMUM_VARIANT_UNITS, 41);
    assert_eq!(minimum, 1_007_616);
    assert!(minimum > zryna_native_mir::generic_owned_v2::MAX_CODEGEN_UNITS);
    for drops in [1, DROPS] {
        super::support::with_source(source(drops), |program| {
            assert!(
                old_fixed_charge(program) < zryna_native_mir::generic_owned_v2::MAX_CODEGEN_UNITS
            );
            let result = zryna_native_mir::generic_owned_v2::lower(program);
            if drops == 1 {
                result.expect("one complete nominal destructor fits existing native budgets");
            } else {
                let Err(error) = result else {
                    panic!(
                        "native MIR admitted complete nominal cleanup above its unchanged work cap"
                    );
                };
                assert_eq!(error.code(), "ZRYNA-N7001");
            }
        });
    }
}
