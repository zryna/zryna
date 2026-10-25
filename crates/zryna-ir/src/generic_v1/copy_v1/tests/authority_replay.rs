//! Foreign authority replay and exact stored-type demand rejection.

use super::*;

#[test]
fn equal_text_foreign_source_runtime_or_entry_cannot_replay() {
    let first = Fixture::new();
    let second = Fixture::new();
    let abi = runtime::verify_v1(
        runtime::raw_v1(&second.linear, &second.linux).expect("genuine fixture invariant"),
        &second.linear,
        &second.linux,
    )
    .expect("genuine fixture invariant");
    assert!(
        verify(
            decode(&encode(&first.raw).expect("genuine fixture invariant"))
                .expect("genuine fixture invariant"),
            &first.syntax,
            &first.sources,
            first.sources.verify_file_id(0).expect("genuine fixture invariant"),
            &first.linear,
            &first.linux,
            &abi
        )
        .is_err()
    );
    let abi = runtime::verify_v1(
        runtime::raw_v1(&first.linear, &first.linux).expect("genuine fixture invariant"),
        &first.linear,
        &first.linux,
    )
    .expect("genuine fixture invariant");
    assert!(
        verify(
            decode(&encode(&first.raw).expect("genuine fixture invariant"))
                .expect("genuine fixture invariant"),
            &first.syntax,
            &first.sources,
            second.sources.verify_file_id(0).expect("genuine fixture invariant"),
            &first.linear,
            &first.linux,
            &abi
        )
        .is_err()
    );
}

#[test]
fn foreign_verified_function_identity_cannot_select_a_local_instance() {
    let first = Fixture::new();
    let second = Fixture::new();
    let a = runtime::verify_v1(
        runtime::raw_v1(&first.linear, &first.linux).expect("first contract"),
        &first.linear,
        &first.linux,
    )
    .expect("first runtime authority");
    let b = runtime::verify_v1(
        runtime::raw_v1(&second.linear, &second.linux).expect("second contract"),
        &second.linear,
        &second.linux,
    )
    .expect("second runtime authority");
    let left = verify(
        decode(&encode(&first.raw).expect("first wire")).expect("first decode"),
        &first.syntax,
        &first.sources,
        first.sources.verify_file_id(0).expect("first entry"),
        &first.linear,
        &first.linux,
        &a,
    )
    .expect("first program");
    let right = verify(
        decode(&encode(&second.raw).expect("second wire")).expect("second decode"),
        &second.syntax,
        &second.sources,
        second.sources.verify_file_id(0).expect("second entry"),
        &second.linear,
        &second.linux,
        &b,
    )
    .expect("second program");
    let id = left.function_id(0).expect("first generic identity");
    assert!(left.function(id).is_some());
    assert!(right.function(id).is_none());
}

#[test]
fn physically_valid_extra_stored_type_cannot_enter_exact_source_demand() {
    use zryna_layout::{
        generic_v1::{raw as types, verify},
        raw::TypeKind as Base,
    };
    let mut fixture = Fixture::new();
    let kinds = vec![
        types::TypeKind::Base(Base::Bool),
        types::TypeKind::Base(Base::I32),
        types::TypeKind::Base(Base::String),
        types::TypeKind::Option { argument: types::NodeId(1) },
        types::TypeKind::Result { okay: types::NodeId(0), error: types::NodeId(1) },
        types::TypeKind::Base(Base::Vec { element: types::NodeId(1) }),
    ];
    let graph = types::Graph {
        modules: vec![types::Module {
            id: types::ModuleId(0),
            source_file: fixture.sources.verify_file_id(0).expect("entry"),
            data_declarations: 0,
        }],
        declarations: vec![],
        types: kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| types::TypeNode {
                id: types::NodeId(u32::try_from(index).expect("index")),
                span: None,
                kind,
            })
            .collect(),
        program_roots: (0..6).map(types::NodeId).collect(),
    };
    fixture.linear = verify(&graph, &fixture.sources, StorageTarget::Linear32V1)
        .expect("independent valid linear graph");
    fixture.linux = verify(&graph, &fixture.sources, StorageTarget::LinuxX8664V1)
        .expect("independent valid Linux graph");
    fixture.raw.universe = *fixture.linear.universe_identity();
    fixture.raw.linear32 = *fixture.linear.fingerprint();
    fixture.raw.linux_x86_64 = *fixture.linux.fingerprint();
    fixture.raw.type_keys = fixture.linear.types().map(|ty| ty.key().to_vec()).collect();
    crate::generic_v1::validate_source_graph(
        &fixture.raw,
        &fixture.syntax,
        &fixture.sources,
        &fixture.linear,
        &fixture.linux,
    )
    .expect("old partial source check remains insufficient for exact type demand");
    assert!(check(&fixture, &fixture.raw).is_err());
}
