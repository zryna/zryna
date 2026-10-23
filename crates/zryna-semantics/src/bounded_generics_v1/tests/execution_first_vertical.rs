//! New owning `execution_v1` controls; baseline private-v5 rejection is a different test.
use super::full_profile_first_vertical::source;
use crate::bounded_generics_v1::{
    SemanticInput,
    body_types::{BodyTypeFailure, check_body_types},
    instantiation::{discover as discover_types, execution_v1, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_ir::generic_v1::execution_v1 as ir_execution;
use zryna_layout::{StorageTarget, generic_v1::VerifiedLayouts};
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_source::{SourceMap, UntrustedSpan};

#[test]
fn five_frozen_originals_discover_and_independently_issue_the_new_execution_domain() {
    for case in source::POSITIVE {
        for provider in source::PROVIDERS {
            let (sources, syntax) = source::project(case, provider);
            let entry = sources.verify_file_id(0).expect("entry");
            let declarations = resolve_declarations(
                SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
            )
            .expect("all original declarations");
            let bodies = check_body_types(&declarations).expect("full original capabilities");
            let execution = execution_v1::discover(&bodies)
                .expect("new owning original-before-generated bridge");
            let independent =
                discover_types(&bodies).expect("canonical semantic demand control only");
            assert_eq!(
                execution.instances().type_keys().collect::<Vec<_>>(),
                independent.type_keys().collect::<Vec<_>>()
            );
            assert_eq!(
                execution.instances().function_keys().collect::<Vec<_>>(),
                independent.function_keys().collect::<Vec<_>>()
            );
            let linear = verify_layouts(execution.instances(), StorageTarget::Linear32V1)
                .expect("linear layout");
            let linux = verify_layouts(execution.instances(), StorageTarget::LinuxX8664V1)
                .expect("Linux layout");
            if case == "fixed-array-identity" {
                check_array(execution.instances(), &linear, &linux);
            }
            let claim = execution_v1::produce_claim(&execution, &linear, &linux)
                .expect("new genuine source-owned claims");
            let wire = ir_execution::wire::encode(&claim).expect("new complete execution wire");
            let runtime = runtime::verify_v1(
                runtime::raw_v1(&linear, &linux).expect("runtime declarations"),
                &linear,
                &linux,
            )
            .expect("independent runtime declarations");
            ir_execution::verify(
                ir_execution::wire::decode(&wire).expect("distinct execution carrier"),
                &syntax,
                &sources,
                entry,
                &linear,
                &linux,
                &runtime,
            )
            .expect("independent NEW execution source/type/state/plan/ABI issuer");
        }
    }
}
fn check_array(
    instances: &crate::bounded_generics_v1::instantiation::InstanceContext<'_, '_, '_>,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
) {
    let expected = source::expected();
    let array = &expected["fixed_array"];
    let key = decode_hex(array["type_key_hex"].as_str().expect("independent array key"));
    let function = decode_hex(
        array["identity_instance_key_hex"].as_str().expect("independent actual generic call key"),
    );
    assert!(instances.type_keys().any(|actual| actual == key));
    assert_eq!(instances.function_keys().collect::<Vec<_>>(), vec![function.as_slice()]);
    for layouts in [linear, linux] {
        let ty = layouts.types().find(|ty| ty.key() == key).expect("actual FixedArray instance");
        assert_eq!(ty.size(), 8);
        assert_eq!(ty.alignment(), 4);
        assert_eq!(ty.array_length(), Some(2));
        assert_eq!(ty.array_stride(), Some(4));
        assert_eq!(
            layouts
                .type_by_id(ty.referenced_type().expect("array element"))
                .expect("same-universe element")
                .key(),
            &[1]
        );
        for other in layouts.types().filter(|ty| ty.key() != key) {
            assert_eq!(other.array_length(), None);
            assert_eq!(other.array_stride(), None);
        }
    }
}
fn decode_hex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0, "complete frozen hex bytes");
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).expect("hex"), 16).expect("byte"))
        .collect()
}

#[test]
fn frozen_unused_opaque_clone_rejects_at_original_keyword_before_new_context_or_program() {
    let expected = source::expected();
    let row = expected["cases"]
        .as_array()
        .expect("frozen cases")
        .iter()
        .find(|row| row["name"] == source::NEGATIVE)
        .expect("opaque control");
    let number = |field: &str| {
        u32::try_from(row["diagnostic"][field].as_u64().expect("frozen byte range"))
            .expect("offset")
    };
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(source::NEGATIVE, provider);
        let entry = sources.verify_file_id(0).expect("entry");
        let declarations =
            resolve_declarations(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
                .expect("declarations");
        let BodyTypeFailure::Diagnostics(errors) =
            check_body_types(&declarations).expect_err("no body/discovery context")
        else {
            panic!("owning original M7002")
        };
        let span = sources
            .verify_span(UntrustedSpan { file: 0, start: number("start"), end: number("end") })
            .expect("frozen source keyword");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7002");
        assert_eq!(errors[0].primary_span(), Some(span));
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            ir_execution::check_original_bodies(&syntax, &sources)
                .expect_err("independent new source issuer cannot launder unused opaque Clone")
        else {
            panic!("owning source rejection, no raw/internal substitution")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7002");
        assert_eq!(errors[0].primary_span(), Some(span));
    }
}

#[test]
fn new_semantic_producer_rejects_wrong_target_and_incomplete_same_source_layout_demand() {
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project("fixed-array-identity", provider);
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, sources.verify_file_id(0).expect("entry"))
                .expect("input"),
        )
        .expect("declarations");
        let bodies = check_body_types(&declarations).expect("body types");
        let execution = execution_v1::discover(&bodies).expect("new original ownership");
        let linear =
            verify_layouts(execution.instances(), StorageTarget::Linear32V1).expect("linear");
        let linux =
            verify_layouts(execution.instances(), StorageTarget::LinuxX8664V1).expect("Linux");
        assert!(matches!(
            execution_v1::produce_claim(&execution, &linux, &linear),
            Err(zryna_ir::generic_v1::Failure::InternalFailure)
        ));
        let missing_linear = incomplete_layout(&sources, StorageTarget::Linear32V1);
        let missing_linux = incomplete_layout(&sources, StorageTarget::LinuxX8664V1);
        assert!(matches!(
            execution_v1::produce_claim(&execution, &missing_linear, &linux),
            Err(zryna_ir::generic_v1::Failure::InternalFailure)
        ));
        assert!(matches!(
            execution_v1::produce_claim(&execution, &linear, &missing_linux),
            Err(zryna_ir::generic_v1::Failure::InternalFailure)
        ));
        let (foreign_linear, foreign_linux) = foreign_layouts(provider);
        assert_eq!(
            foreign_linear.types().map(zryna_layout::generic_v1::TypeView::key).collect::<Vec<_>>(),
            linear.types().map(zryna_layout::generic_v1::TypeView::key).collect::<Vec<_>>()
        );
        assert_ne!(foreign_linear.source_map_identity(), linear.source_map_identity());
        assert!(matches!(
            execution_v1::produce_claim(&execution, &foreign_linear, &linux),
            Err(zryna_ir::generic_v1::Failure::InternalFailure)
        ));
        assert!(matches!(
            execution_v1::produce_claim(&execution, &linear, &foreign_linux),
            Err(zryna_ir::generic_v1::Failure::InternalFailure)
        ));
        execution_v1::produce_claim(&execution, &linear, &linux)
            .expect("pristine genuine retry after all hostile layouts");
    }
}
fn incomplete_layout(sources: &SourceMap, target: StorageTarget) -> VerifiedLayouts {
    use zryna_layout::generic_v1::raw;
    // Hostile layout-phase input only; never an execution proof or fabricated source world.
    let graph = raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: sources.verify_file_id(0).expect("same source"),
            data_declarations: 0,
        }],
        declarations: vec![],
        types: [
            zryna_layout::raw::TypeKind::Bool,
            zryna_layout::raw::TypeKind::I32,
            zryna_layout::raw::TypeKind::String,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, kind)| raw::TypeNode {
            id: raw::NodeId(u32::try_from(index).expect("three primitive nodes")),
            span: None,
            kind: raw::TypeKind::Base(kind),
        })
        .collect(),
        program_roots: vec![raw::NodeId(0), raw::NodeId(1), raw::NodeId(2)],
    };
    zryna_layout::generic_v1::verify(&graph, sources, target)
        .expect("independent valid but incomplete same-source layout")
}

fn foreign_layouts(provider: &str) -> (VerifiedLayouts, VerifiedLayouts) {
    let (sources, syntax) = source::project("fixed-array-identity", provider);
    let declarations = resolve_declarations(
        SemanticInput::try_new(
            &syntax,
            &sources,
            sources.verify_file_id(0).expect("foreign entry"),
        )
        .expect("foreign input"),
    )
    .expect("same-byte independent declarations");
    let bodies = check_body_types(&declarations).expect("same-byte original capabilities");
    let instances =
        discover_types(&bodies).expect("foreign type demand only, no execution authority");
    (
        verify_layouts(&instances, StorageTarget::Linear32V1).expect("foreign linear"),
        verify_layouts(&instances, StorageTarget::LinuxX8664V1).expect("foreign Linux"),
    )
}
