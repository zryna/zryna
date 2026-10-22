use zryna_ir::data_ownership_v1::{VerifiedBlock, VerifiedInstructionKind as K};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::{command_h1_v1, v4};

#[test]
fn command_indexed_scalar_operands_keep_typed_results_after_scope_cleanup() {
    for (name, text, bytes, indexed_reads, opcode) in [
        (
            "vec-i32",
            include_str!("fixtures/vec-i32.zry"),
            include_bytes!("fixtures/vec-i32.json").as_slice(),
            2,
            K::Eq,
        ),
        (
            "array-i32",
            include_str!("fixtures/array-i32.zry"),
            include_bytes!("fixtures/array-i32.json").as_slice(),
            2,
            K::Eq,
        ),
        (
            "vec-bool",
            include_str!("fixtures/vec-bool.zry"),
            include_bytes!("fixtures/vec-bool.json").as_slice(),
            2,
            K::Ne,
        ),
        (
            "clone-vector-growth",
            include_str!(
                "../../../../../tests/wasi-command-source-fixtures/clone-vector-growth.zry"
            ),
            include_bytes!(
                "../../../../../tests/wasi-command-source-fixtures/clone-vector-growth.json"
            )
            .as_slice(),
            1,
            K::Eq,
        ),
    ] {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".into(),
            text: text.into(),
        }])
        .expect("regression source");
        let syntax = v4::verify_snapshot(v4::decode_snapshot(bytes).expect("DTO"), &sources)
            .expect("independently authenticated syntax");
        let source = command_h1_v1::admit(&syntax, &sources).expect("complete command source");
        let program =
            super::lower(&source, &sources).unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
        let function = program
            .verified_ir()
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("main");
        let instructions =
            function.blocks().flat_map(VerifiedBlock::instructions).collect::<Vec<_>>();
        let reads = instructions
            .iter()
            .filter(|instruction| {
                matches!(instruction.kind(), K::VecIndexCopy | K::BorrowRead | K::CopyFromPlace)
            })
            .map(|instruction| instruction.result().expect("typed indexed read"))
            .collect::<Vec<_>>();
        assert_eq!(reads.len(), indexed_reads, "{name}: each indexed operand evaluated once");
        let scalar_operands = instructions
            .iter()
            .filter(|instruction| matches!(instruction.kind(), K::I32Add | K::Eq | K::Ne))
            .flat_map(|instruction| instruction.value_operands())
            .collect::<Vec<_>>();
        assert!(reads.iter().all(|read| scalar_operands.contains(read)), "{name}");
        assert!(instructions.iter().any(|instruction| instruction.kind() == opcode), "{name}");
        if name == "array-i32" {
            assert_eq!(
                instructions
                    .iter()
                    .filter(|instruction| instruction.kind() == K::EndBorrow)
                    .count(),
                0,
                "statically checked Copy array reads do not create a borrow"
            );
        }
        if name != "array-i32" {
            assert!(
                function
                    .blocks()
                    .last()
                    .expect("return block")
                    .terminator()
                    .derived_drop_actions()
                    .next()
                    .is_some(),
                "{name}: retained container owner is cleaned at return"
            );
        }
    }
}
