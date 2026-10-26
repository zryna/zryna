//! Actual emitted constructor representation only; no synthetic source/runtime conformance.
use crate::generic_execution_v1::{self as execution, accounting_tests};
use zryna_layout::{StorageTarget, TypeCategory};
#[test]
fn actual_enum_constructor_preserves_descriptors_evaluation_and_leaf_identity() {
    for provider in ["native", "typescript"] {
        super::with_program("nominal-enum-clone", provider, |program| {
            let ordinary = execution::emit(program).expect("genuine NEW ordinary source");
            let observed = execution::emit_observed(program).expect("same-source observed route");
            let constructor = ordinary
                .source
                .lines()
                .find(|line| line.starts_with("function $enumOwner("))
                .expect("direct null-prototype Enum constructor must be emitted");
            assert_eq!(
                Some(constructor),
                observed.as_source().lines().find(|line| line.starts_with("function $enumOwner("))
            );
            for source in [ordinary.source.as_str(), observed.as_source()] {
                assert!(
                    !source.contains("$owner({tag:"),
                    "all hot Enum production sites use the direct constructor"
                );
                assert!(
                    source.contains("$enumOwner($s.tag,undefined)"),
                    "certified Clone destination"
                );
            }
            reject_constructor_mutations(program, &ordinary.source);
            let enum_type = program
                .layouts(StorageTarget::Linear32V1)
                .types()
                .find(|ty| ty.category() == TypeCategory::Enum)
                .expect("genuine Signal layout");
            let ordinal = enum_type
                .variants()
                .find_map(|(ordinal, child)| child.map(|_| ordinal))
                .expect("genuine String payload variant");
            let text = format!(
                "{}\nconst $transportOwner=$record{}; const $actualTag={ordinal};\n{}",
                ordinary.source,
                enum_type.id().index(),
                include_str!("enum_owner.mjs")
            );
            let receipt =
                accounting_tests::private_boundary(&format!("enum-owner-{provider}"), &text);
            assert_eq!(receipt["constructorRepresentationOnly"], true);
            assert_eq!(receipt["sourceRuntimeConformance"], false);
            assert_eq!(receipt["prototypeCallbacks"], 0);
            assert_eq!(receipt["prototypeMutations"], 0);
            assert_eq!(
                ordinary,
                execution::emit(program).expect("deterministic same-owner replay")
            );
            let limit = ordinary.source.len();
            assert_eq!(
                ordinary,
                execution::emit_with_budget(program, limit).expect("actual exact byte budget")
            );
            assert_eq!(
                execution::emit_with_budget(program, limit - 1)
                    .expect_err("first extra byte")
                    .code(),
                "ZRYNA-J2003"
            );
        });
    }
}

fn reject_constructor_mutations(
    program: &zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>,
    source: &str,
) {
    for (before, after) in [
        ("__proto__:null,tag:$tag", "__proto__:{},tag:$tag"),
        ("$dropType:-1,$dropIndex:-1}; }", "$dropType:-2,$dropIndex:-1}; }"),
    ] {
        let changed = source.replacen(before, after, 1);
        assert_ne!(changed, source, "actual constructor mutation must occur");
        let context = execution::Context::new(program).expect("same independent NEW owner");
        let mode = execution::Mode::Ordinary(
            execution::runtime::ordinary::quota::prove(&context).expect("same finite quota"),
        );
        assert!(
            execution::runtime::ordinary::audit(&context, &changed, mode).is_err(),
            "constructor bytes cannot drift from exact audit"
        );
    }
}
