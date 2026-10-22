//! Raw successor layout checks are independent of semantic source admission.

use std::fmt::Write as _;

use super::{Failure, raw, verify, verify_claim};
use crate::{StorageTarget, raw::TypeKind as Base};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zryna_source::{SourceFileInput, SourceMap};

pub(super) fn sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "interface Box<T extends ZrynaValue> extends ZrynaStruct {value:T;}".into(),
    }])
    .expect("immutable source map")
}

pub(super) fn graph(sources: &SourceMap, kind: raw::TypeKind) -> raw::Graph {
    let file = sources.verify_file_id(0).expect("fixture file");
    let nominal = match kind {
        raw::TypeKind::Struct { .. } => Some((raw::NominalKind::Struct, 1)),
        raw::TypeKind::Enum { .. } => Some((raw::NominalKind::Enum, 2)),
        _ => None,
    };
    let span = sources.span(file, 0, 63).expect("original declaration span");
    let declarations = nominal.map_or_else(Vec::new, |(kind, parameters)| {
        vec![raw::Declaration {
            module: raw::ModuleId(0),
            index: 0,
            kind,
            parameters,
            members: parameters,
            span,
        }]
    });
    let mut types = [Base::Bool, Base::I32, Base::String]
        .into_iter()
        .enumerate()
        .map(|(index, kind)| raw::TypeNode {
            id: raw::NodeId(u32::try_from(index).expect("primitive index")),
            span: None,
            kind: raw::TypeKind::Base(kind),
        })
        .collect::<Vec<_>>();
    types.push(raw::TypeNode { id: raw::NodeId(3), span: nominal.map(|_| span), kind });
    raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: u32::try_from(declarations.len()).expect("declarations"),
        }],
        declarations,
        types,
        program_roots: vec![raw::NodeId(3)],
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::new();
    for byte in bytes {
        write!(text, "{byte:02x}").expect("fixture hex");
    }
    text
}

fn last_record(bytes: &[u8]) -> &[u8] {
    let mut cursor = b"ZRYNA-GENERIC-AGGREGATE-LAYOUT-V1\0".len() + 8;
    let mut start = cursor;
    while cursor < bytes.len() {
        start = cursor;
        let length =
            u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().expect("record length"));
        cursor += 4 + usize::try_from(length).expect("record bytes");
    }
    &bytes[start..]
}

fn fixture_kind(name: &str) -> raw::TypeKind {
    match name {
        "Option<i32>" => raw::TypeKind::Option { argument: raw::NodeId(1) },
        "Result<i32,bool>" => raw::TypeKind::Result { okay: raw::NodeId(1), error: raw::NodeId(0) },
        "Box<i32>" => raw::TypeKind::Struct {
            module: raw::ModuleId(0),
            declaration: 0,
            arguments: vec![raw::NodeId(1)],
            fields: vec![raw::Field { ordinal: 0, ty: raw::NodeId(1) }],
        },
        "Choice<i32,bool>" => raw::TypeKind::Enum {
            module: raw::ModuleId(0),
            declaration: 0,
            arguments: vec![raw::NodeId(1), raw::NodeId(0)],
            variants: vec![
                raw::Variant { ordinal: 0, payload: Some(raw::NodeId(1)) },
                raw::Variant { ordinal: 1, payload: Some(raw::NodeId(0)) },
            ],
        },
        _ => panic!("unexpected independently frozen fixture"),
    }
}

#[test]
fn successor_four_fixed_records_keys_and_dual_target_digests_match_independent_oracles() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../../../spec/memory-model/generic-enum-layout-v1-fixtures.json"
    ))
    .expect("fixed reference JSON");
    let sources = sources();
    for case in fixtures["cases"].as_array().expect("fixed cases") {
        let graph = graph(&sources, fixture_kind(case["name"].as_str().expect("fixture name")));
        for (target, prefix) in
            [(StorageTarget::Linear32V1, "linear32"), (StorageTarget::LinuxX8664V1, "linuxX8664")]
        {
            let layouts = verify(&graph, &sources, target).expect("independent successor layout");
            let ty = layouts.types().last().expect("closed family record");
            assert_eq!(hex(ty.key()), case["typeKeyHex"].as_str().expect("fixed complete key"));
            assert_eq!(
                hex(&Sha256::digest(ty.key())),
                case["typeKeySha256"].as_str().expect("fixed key hash")
            );
            assert_eq!(
                hex(last_record(layouts.canonical_bytes())),
                case["recordHex"].as_str().expect("fixed record")
            );
            assert_eq!(
                layouts.canonical_bytes().len() as u64,
                case[format!("{prefix}DocumentBytes")].as_u64().expect("document size")
            );
            assert_eq!(
                hex(layouts.fingerprint()),
                case[format!("{prefix}Sha256")].as_str().expect("fixed target digest")
            );
            assert!(
                verify_claim(
                    &graph,
                    &sources,
                    target,
                    layouts.canonical_bytes(),
                    layouts.fingerprint()
                )
                .is_ok()
            );
        }
    }
}

#[test]
fn owned_nested_standard_enum_layouts_match_every_fixed_prefix_on_both_targets() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../../../spec/memory-model/generic-owned-layout-v1-fixtures.json"
    ))
    .expect("owned fixed reference JSON");
    let sources = sources();
    let mut graph = graph(&sources, raw::TypeKind::Option { argument: raw::NodeId(2) });
    for (index, case) in fixtures["cases"].as_array().expect("owned cases").iter().enumerate() {
        if index == 1 {
            graph.types.push(raw::TypeNode {
                id: raw::NodeId(4),
                span: None,
                kind: raw::TypeKind::Option { argument: raw::NodeId(3) },
            });
        }
        if index == 2 {
            graph.types.push(raw::TypeNode {
                id: raw::NodeId(5),
                span: None,
                kind: raw::TypeKind::Result { okay: raw::NodeId(3), error: raw::NodeId(2) },
            });
        }
        graph.program_roots =
            (3..u32::try_from(graph.types.len()).expect("prefix size")).map(raw::NodeId).collect();
        for expected in case["targets"].as_array().expect("both targets") {
            let target = if expected["target"] == 1 {
                StorageTarget::Linear32V1
            } else {
                StorageTarget::LinuxX8664V1
            };
            let layouts = verify(&graph, &sources, target).expect("owned successor layout");
            let ty = layouts.types().last().expect("last closed prefix member");
            assert_eq!(ty.size(), expected["size"].as_u64().expect("stored size"));
            assert_eq!(ty.alignment(), expected["alignment"].as_u64().expect("alignment"));
            assert_eq!(ty.drop_kind(), 1);
            assert_eq!(ty.runtime_kind(), 1);
            assert_eq!(
                hex(last_record(layouts.canonical_bytes())),
                expected["recordHex"].as_str().expect("fixed record")
            );
            assert_eq!(
                hex(layouts.fingerprint()),
                expected["sha256"].as_str().expect("fixed full prefix hash")
            );
        }
    }
}

#[test]
fn independent_record_mutations_and_wrong_target_fail_before_any_successor_seal() {
    let sources = sources();
    let graph = graph(&sources, fixture_kind("Option<i32>"));
    let layouts = verify(&graph, &sources, StorageTarget::Linear32V1).expect("complete layout");
    for index in 0..layouts.canonical_bytes().len() {
        let mut bytes = layouts.canonical_bytes().to_vec();
        bytes[index] ^= 1;
        assert!(
            verify_claim(
                &graph,
                &sources,
                StorageTarget::Linear32V1,
                &bytes,
                layouts.fingerprint()
            )
            .is_err(),
            "mutation byte {index}"
        );
    }
    let mut fingerprint = *layouts.fingerprint();
    fingerprint[0] ^= 1;
    assert!(
        verify_claim(
            &graph,
            &sources,
            StorageTarget::Linear32V1,
            layouts.canonical_bytes(),
            &fingerprint
        )
        .is_err()
    );
    assert!(
        verify_claim(
            &graph,
            &sources,
            StorageTarget::LinuxX8664V1,
            layouts.canonical_bytes(),
            layouts.fingerprint()
        )
        .is_err()
    );
}

mod graph_admission;
