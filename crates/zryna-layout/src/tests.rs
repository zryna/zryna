use std::fmt::Write as _;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use zryna_source::{SourceFileInput, SourceMap};

use super::{StorageTarget, TypeCategory, raw, verify};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fixture {
    schema_version: u32,
    contract_id: String,
    five_type_universe: FiveTypeUniverse,
    normative_layouts: Vec<NormativeLayout>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FiveTypeUniverse {
    types: Vec<String>,
    type_ids: Vec<u32>,
    record_count: usize,
    document_bytes: usize,
    targets: Vec<TargetOracle>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TargetOracle {
    target: String,
    fingerprint_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormativeLayout {
    name: String,
    linear32: LayoutOracle,
    linux_x8664: LayoutOracle,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LayoutOracle {
    size: u64,
    alignment: u64,
    #[serde(default)]
    offsets: Option<Vec<u64>>,
    #[serde(default)]
    stride: Option<u64>,
    #[serde(default)]
    payload_offset: Option<u64>,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/layout-v1.json"))
        .expect("canonical layout fixture must parse")
}

fn sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: "data Pair\n".into() }])
        .expect("fixture source map must build")
}

fn full_sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "Pair\nMixed\nNested\nMaybeI32\nChoice\nTextFlag\nLinks\n".into(),
    }])
    .expect("full fixture source map must build")
}

#[allow(clippy::too_many_lines)]
fn full_graph(sources: &SourceMap) -> raw::Graph {
    let file = sources.verify_file_id(0).expect("fixture file");
    let spans = [(0, 4), (5, 10), (11, 17), (18, 26), (27, 33), (34, 42), (43, 48)]
        .map(|(start, end)| sources.span(file, start, end).expect("fixture declaration span"));
    raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: 7,
        }],
        types: vec![
            raw::TypeNode { id: raw::NodeId(0), span: None, kind: raw::TypeKind::Bool },
            raw::TypeNode { id: raw::NodeId(1), span: None, kind: raw::TypeKind::I32 },
            raw::TypeNode { id: raw::NodeId(2), span: None, kind: raw::TypeKind::String },
            raw::TypeNode {
                id: raw::NodeId(3),
                span: Some(spans[0]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 0,
                    fields: vec![
                        raw::Field { ordinal: 0, ty: raw::NodeId(1) },
                        raw::Field { ordinal: 1, ty: raw::NodeId(1) },
                    ],
                },
            },
            raw::TypeNode {
                id: raw::NodeId(4),
                span: Some(spans[1]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 1,
                    fields: vec![
                        raw::Field { ordinal: 0, ty: raw::NodeId(0) },
                        raw::Field { ordinal: 1, ty: raw::NodeId(1) },
                        raw::Field { ordinal: 2, ty: raw::NodeId(0) },
                    ],
                },
            },
            raw::TypeNode {
                id: raw::NodeId(5),
                span: Some(spans[2]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 2,
                    fields: vec![
                        raw::Field { ordinal: 0, ty: raw::NodeId(3) },
                        raw::Field { ordinal: 1, ty: raw::NodeId(0) },
                    ],
                },
            },
            raw::TypeNode {
                id: raw::NodeId(6),
                span: None,
                kind: raw::TypeKind::FixedArray { element: raw::NodeId(0), length: 3 },
            },
            raw::TypeNode {
                id: raw::NodeId(7),
                span: None,
                kind: raw::TypeKind::FixedArray { element: raw::NodeId(3), length: 2 },
            },
            raw::TypeNode {
                id: raw::NodeId(8),
                span: Some(spans[3]),
                kind: raw::TypeKind::Enum {
                    module: raw::ModuleId(0),
                    declaration: 3,
                    variants: vec![
                        raw::Variant { ordinal: 0, payload: None },
                        raw::Variant { ordinal: 1, payload: Some(raw::NodeId(1)) },
                    ],
                },
            },
            raw::TypeNode {
                id: raw::NodeId(9),
                span: Some(spans[4]),
                kind: raw::TypeKind::Enum {
                    module: raw::ModuleId(0),
                    declaration: 4,
                    variants: vec![
                        raw::Variant { ordinal: 0, payload: Some(raw::NodeId(0)) },
                        raw::Variant { ordinal: 1, payload: Some(raw::NodeId(3)) },
                    ],
                },
            },
            raw::TypeNode {
                id: raw::NodeId(10),
                span: Some(spans[5]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 5,
                    fields: vec![
                        raw::Field { ordinal: 0, ty: raw::NodeId(2) },
                        raw::Field { ordinal: 1, ty: raw::NodeId(0) },
                    ],
                },
            },
            raw::TypeNode {
                id: raw::NodeId(11),
                span: None,
                kind: raw::TypeKind::Shared { payload: raw::NodeId(3) },
            },
            raw::TypeNode {
                id: raw::NodeId(12),
                span: None,
                kind: raw::TypeKind::Weak { payload: raw::NodeId(3) },
            },
            raw::TypeNode {
                id: raw::NodeId(13),
                span: Some(spans[6]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 6,
                    fields: vec![
                        raw::Field { ordinal: 0, ty: raw::NodeId(11) },
                        raw::Field { ordinal: 1, ty: raw::NodeId(12) },
                    ],
                },
            },
            raw::TypeNode {
                id: raw::NodeId(14),
                span: None,
                kind: raw::TypeKind::Vec { element: raw::NodeId(3) },
            },
        ],
        program_roots: vec![raw::NodeId(6), raw::NodeId(7), raw::NodeId(14)],
    }
}

fn five_type_graph(sources: &SourceMap) -> raw::Graph {
    let file = sources.verify_file_id(0).expect("fixture file");
    let span = sources.span(file, 0, 9).expect("fixture span");
    raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: 1,
        }],
        types: vec![
            raw::TypeNode { id: raw::NodeId(0), span: None, kind: raw::TypeKind::I32 },
            raw::TypeNode {
                id: raw::NodeId(1),
                span: Some(span),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 0,
                    fields: vec![
                        raw::Field { ordinal: 0, ty: raw::NodeId(0) },
                        raw::Field { ordinal: 1, ty: raw::NodeId(0) },
                    ],
                },
            },
            raw::TypeNode { id: raw::NodeId(2), span: None, kind: raw::TypeKind::Bool },
            raw::TypeNode { id: raw::NodeId(3), span: None, kind: raw::TypeKind::String },
            raw::TypeNode {
                id: raw::NodeId(4),
                span: None,
                kind: raw::TypeKind::FixedArray { element: raw::NodeId(1), length: 2 },
            },
        ],
        program_roots: vec![raw::NodeId(4)],
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut output, byte| {
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
        output
    })
}

#[test]
fn seals_the_exact_five_type_oracles_for_both_targets() {
    let sources = sources();
    let graph = five_type_graph(&sources);
    let fixture = fixture();
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.contract_id, "zryna-aggregate-layout-v1");
    assert_eq!(fixture.five_type_universe.types, ["bool", "i32", "String", "Pair", "[Pair;2]"]);
    assert_eq!(fixture.five_type_universe.type_ids, [0, 1, 2, 3, 4]);
    assert_eq!(fixture.normative_layouts.len(), 9);
    assert_eq!(
        fixture.normative_layouts.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
        [
            "Pair", "Mixed", "Nested", "bool[3]", "Pair[2]", "MaybeI32", "Choice", "TextFlag",
            "Links"
        ]
    );
    assert_eq!(
        fixture
            .five_type_universe
            .targets
            .iter()
            .map(|oracle| oracle.target.as_str())
            .collect::<Vec<_>>(),
        ["Linear32V1", "LinuxX8664V1"]
    );
    let cases =
        fixture.five_type_universe.targets.iter().map(|oracle| match oracle.target.as_str() {
            "Linear32V1" => {
                (StorageTarget::Linear32V1, oracle.fingerprint_sha256.as_str(), (12, 4))
            }
            "LinuxX8664V1" => {
                (StorageTarget::LinuxX8664V1, oracle.fingerprint_sha256.as_str(), (24, 8))
            }
            other => panic!("unknown fixture target {other}"),
        });
    for (target, expected_fingerprint, string_layout) in cases {
        let verified = verify(&graph, &sources, target).expect("fixture must verify");
        assert_eq!(verified.canonical_bytes.len(), fixture.five_type_universe.document_bytes);
        assert_eq!(verified.types().len(), fixture.five_type_universe.record_count);
        assert_eq!(hex(verified.fingerprint()), expected_fingerprint);
        let types = verified.types().collect::<Vec<_>>();
        assert_eq!(
            types.iter().map(|ty| ty.category()).collect::<Vec<_>>(),
            [
                TypeCategory::Bool,
                TypeCategory::I32,
                TypeCategory::String,
                TypeCategory::Struct,
                TypeCategory::FixedArray,
            ]
        );
        assert_eq!((types[2].size(), types[2].alignment()), string_layout);
        assert_eq!((types[3].size(), types[3].alignment()), (8, 4));
        assert_eq!(
            types[3].fields().iter().map(|field| field.offset()).collect::<Vec<_>>(),
            [0, 4]
        );
        assert_eq!(
            (types[4].size(), types[4].alignment(), types[4].array_stride()),
            (16, 4, Some(8))
        );
    }
}

#[test]
fn canonical_ids_do_not_depend_on_raw_discovery_order() {
    let sources = sources();
    let first_graph = five_type_graph(&sources);
    let first = verify(&first_graph, &sources, StorageTarget::Linear32V1).expect("first graph");
    let mut second_graph = five_type_graph(&sources);
    second_graph.types.swap(0, 3);
    for (index, node) in second_graph.types.iter_mut().enumerate() {
        node.id = raw::NodeId(u32::try_from(index).expect("fixture index"));
    }
    for node in &mut second_graph.types {
        match &mut node.kind {
            raw::TypeKind::Struct { fields, .. } => {
                for field in fields {
                    field.ty = raw::NodeId(3);
                }
            }
            raw::TypeKind::FixedArray { element, .. } => *element = raw::NodeId(1),
            _ => {}
        }
    }
    let second =
        verify(&second_graph, &sources, StorageTarget::Linear32V1).expect("reordered graph");
    assert_eq!(first.canonical_bytes, second.canonical_bytes);
    assert_eq!(first.fingerprint(), second.fingerprint());
    let sealed = *second.fingerprint();
    second_graph.types[0].kind = raw::TypeKind::Bool;
    assert_eq!(second.fingerprint(), &sealed);
}

#[test]
fn type_ids_are_rejected_outside_their_issuing_universe() {
    let sources = sources();
    let first = verify(&five_type_graph(&sources), &sources, StorageTarget::Linear32V1)
        .expect("first universe");
    let foreign_id = first.types().last().expect("array type").id();

    let mut second_graph = five_type_graph(&sources);
    let raw::TypeKind::FixedArray { length, .. } = &mut second_graph.types[4].kind else {
        panic!("fixture array")
    };
    *length = 3;
    let second =
        verify(&second_graph, &sources, StorageTarget::Linear32V1).expect("second universe");

    assert_ne!(first.universe_identity(), second.universe_identity());
    assert!(second.type_by_id(foreign_id).is_none());
}

#[test]
fn admission_requires_layout_success_on_both_storage_targets() {
    let sources = sources();
    let mut graph = five_type_graph(&sources);
    graph.types.push(raw::TypeNode {
        id: raw::NodeId(5),
        span: None,
        kind: raw::TypeKind::FixedArray {
            element: raw::NodeId(3),
            length: super::MAX_ARRAY_LENGTH,
        },
    });
    graph.types.push(raw::TypeNode {
        id: raw::NodeId(6),
        span: None,
        kind: raw::TypeKind::FixedArray { element: raw::NodeId(5), length: 300 },
    });
    graph.program_roots.extend([raw::NodeId(5), raw::NodeId(6)]);

    let diagnostics = verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect_err("Linux overflow must reject otherwise-valid linear32 admission");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3005"));
}

#[test]
fn rejects_foreign_source_authority() {
    let first_sources = sources();
    let second_sources = sources();
    let diagnostics =
        verify(&five_type_graph(&first_sources), &second_sources, StorageTarget::Linear32V1)
            .expect_err("foreign FileId must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3001"));
}

#[test]
fn rejects_direct_by_value_recursion() {
    let sources = sources();
    let mut graph = five_type_graph(&sources);
    let raw::TypeKind::Struct { fields, .. } = &mut graph.types[1].kind else {
        panic!("fixture Pair must be a struct")
    };
    fields[0].ty = raw::NodeId(1);
    let diagnostics = verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect_err("recursive struct must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3002"));
}

#[test]
fn rejects_borrows_before_layout_construction() {
    let sources = sources();
    let mut graph = five_type_graph(&sources);
    graph.types.push(raw::TypeNode {
        id: raw::NodeId(5),
        span: None,
        kind: raw::TypeKind::Borrow { referent: raw::NodeId(0) },
    });
    graph.program_roots.push(raw::NodeId(5));
    let diagnostics =
        verify(&graph, &sources, StorageTarget::Linear32V1).expect_err("stored borrow must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3004"));
}

#[test]
fn rejects_limit_plus_one_array_length() {
    let sources = sources();
    let mut graph = five_type_graph(&sources);
    let raw::TypeKind::FixedArray { length, .. } = &mut graph.types[4].kind else {
        panic!("fixture array")
    };
    *length = super::MAX_ARRAY_LENGTH + 1;
    let diagnostics =
        verify(&graph, &sources, StorageTarget::Linear32V1).expect_err("excessive array must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3003"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn verifies_every_normative_aggregate_and_handle_layout() {
    let sources = full_sources();
    let graph = full_graph(&sources);
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        let verified = verify(&graph, &sources, target).expect("full fixture must verify");
        let types = verified.types().collect::<Vec<_>>();
        let nominal = |declaration| {
            types
                .iter()
                .copied()
                .find(|ty| ty.nominal_identity() == Some((0, declaration)))
                .expect("nominal fixture")
        };
        let pair = nominal(0);
        let mixed = nominal(1);
        let nested = nominal(2);
        let maybe = nominal(3);
        let choice = nominal(4);
        let text_flag = nominal(5);
        let links = nominal(6);
        assert_eq!((pair.size(), pair.alignment()), (8, 4));
        assert_eq!(pair.fields().iter().map(|field| field.offset()).collect::<Vec<_>>(), [0, 4]);
        assert_eq!((mixed.size(), mixed.alignment()), (12, 4));
        assert_eq!(
            mixed.fields().iter().map(|field| field.offset()).collect::<Vec<_>>(),
            [0, 4, 8]
        );
        assert_eq!((nested.size(), nested.alignment()), (12, 4));
        assert_eq!(nested.fields().iter().map(|field| field.offset()).collect::<Vec<_>>(), [0, 8]);
        assert_eq!(
            (maybe.size(), maybe.alignment(), maybe.enum_payload_layout()),
            (8, 4, Some((4, 4)))
        );
        assert_eq!(
            (choice.size(), choice.alignment(), choice.enum_payload_layout()),
            (12, 4, Some((4, 8)))
        );
        let arrays = types
            .iter()
            .copied()
            .filter(|ty| ty.category() == TypeCategory::FixedArray)
            .collect::<Vec<_>>();
        let bools =
            arrays.iter().copied().find(|ty| ty.array_length() == Some(3)).expect("bool array");
        let pairs =
            arrays.iter().copied().find(|ty| ty.array_length() == Some(2)).expect("Pair array");
        let named = [
            ("Pair", pair),
            ("Mixed", mixed),
            ("Nested", nested),
            ("bool[3]", bools),
            ("Pair[2]", pairs),
            ("MaybeI32", maybe),
            ("Choice", choice),
            ("TextFlag", text_flag),
            ("Links", links),
        ];
        let fixture = fixture();
        for row in &fixture.normative_layouts {
            let ty = named
                .iter()
                .find_map(|(name, ty)| (*name == row.name).then_some(*ty))
                .unwrap_or_else(|| panic!("unknown normative layout {}", row.name));
            let actual = LayoutOracle {
                size: ty.size(),
                alignment: ty.alignment(),
                offsets: (!ty.fields().is_empty())
                    .then(|| ty.fields().iter().map(|field| field.offset()).collect()),
                stride: ty.array_stride(),
                payload_offset: ty.enum_payload_layout().map(|(offset, _)| offset),
            };
            let expected = match target {
                StorageTarget::Linear32V1 => &row.linear32,
                StorageTarget::LinuxX8664V1 => &row.linux_x8664,
            };
            assert_eq!(&actual, expected, "{} {target:?}", row.name);
        }
        assert_eq!((bools.size(), bools.alignment(), bools.array_stride()), (3, 1, Some(1)));
        assert_eq!((pairs.size(), pairs.alignment(), pairs.array_stride()), (16, 4, Some(8)));
        let (text_expected, links_expected) = match target {
            StorageTarget::Linear32V1 => ((16, 4, vec![0, 12]), (8, 4, vec![0, 4])),
            StorageTarget::LinuxX8664V1 => ((32, 8, vec![0, 24]), (16, 8, vec![0, 8])),
        };
        assert_eq!(
            (
                text_flag.size(),
                text_flag.alignment(),
                text_flag.fields().iter().map(|field| field.offset()).collect::<Vec<_>>()
            ),
            text_expected
        );
        assert_eq!(
            (
                links.size(),
                links.alignment(),
                links.fields().iter().map(|field| field.offset()).collect::<Vec<_>>()
            ),
            links_expected
        );
        assert_eq!((text_flag.drop_kind(), text_flag.runtime_kind()), (1, 1));
        assert_eq!((links.drop_kind(), links.runtime_kind()), (1, 1));
        let shared =
            types.iter().copied().find(|ty| ty.category() == TypeCategory::Shared).expect("Shared");
        let weak =
            types.iter().copied().find(|ty| ty.category() == TypeCategory::Weak).expect("Weak");
        let vector =
            types.iter().copied().find(|ty| ty.category() == TypeCategory::Vec).expect("Vec");
        let handle = if target == StorageTarget::Linear32V1 { (4, 4) } else { (8, 8) };
        assert_eq!(
            (shared.size(), shared.alignment(), shared.drop_kind()),
            (handle.0, handle.1, 4)
        );
        assert_eq!((weak.size(), weak.alignment(), weak.drop_kind()), (handle.0, handle.1, 5));
        let vector_layout =
            if target == StorageTarget::Linear32V1 { (12, 4, 3) } else { (24, 8, 3) };
        assert_eq!((vector.size(), vector.alignment(), vector.drop_kind()), vector_layout);
    }
}

#[test]
fn rejects_zero_sized_vec_elements() {
    let sources = sources();
    let mut graph = five_type_graph(&sources);
    graph.types.push(raw::TypeNode {
        id: raw::NodeId(5),
        span: None,
        kind: raw::TypeKind::FixedArray { element: raw::NodeId(2), length: 0 },
    });
    graph.types.push(raw::TypeNode {
        id: raw::NodeId(6),
        span: None,
        kind: raw::TypeKind::Vec { element: raw::NodeId(5) },
    });
    graph.program_roots.extend([raw::NodeId(5), raw::NodeId(6)]);
    let diagnostics = verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect_err("zero-sized Vec element must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3003"));
}

#[test]
fn sealed_document_audit_rejects_target_identity_ordinal_and_fingerprint_mutations() {
    let sources = sources();
    let verified = verify(&five_type_graph(&sources), &sources, StorageTarget::Linear32V1)
        .expect("fixture must verify");
    let original = &verified.canonical_bytes;
    let fingerprint = *verified.fingerprint();

    let mut wrong_target = original.clone();
    wrong_target[26] = 2;
    assert!(
        super::audit_document(
            &wrong_target,
            StorageTarget::Linear32V1,
            &verified.records,
            &fingerprint,
        )
        .is_err()
    );

    let mut wrong_id = original.clone();
    wrong_id[42] = 9;
    assert!(
        super::audit_document(
            &wrong_id,
            StorageTarget::Linear32V1,
            &verified.records,
            &fingerprint,
        )
        .is_err()
    );

    let mut wrong_ordinal = original.clone();
    let pair_record_start = 34 + 36 * 3;
    let first_pair_ordinal = pair_record_start + 4 + 32 + 12;
    wrong_ordinal[first_pair_ordinal] = 1;
    assert!(
        super::audit_document(
            &wrong_ordinal,
            StorageTarget::Linear32V1,
            &verified.records,
            &fingerprint,
        )
        .is_err()
    );

    let mut wrong_fingerprint = fingerprint;
    wrong_fingerprint[0] ^= 0xff;
    assert!(
        super::audit_document(
            original,
            StorageTarget::Linear32V1,
            &verified.records,
            &wrong_fingerprint,
        )
        .is_err()
    );

    let mut wrong_alignment = original.clone();
    wrong_alignment[62] = 3;
    let wrong_alignment_fingerprint: [u8; 32] = Sha256::digest(&wrong_alignment).into();
    assert!(
        super::audit_document(
            &wrong_alignment,
            StorageTarget::Linear32V1,
            &verified.records,
            &wrong_alignment_fingerprint,
        )
        .is_err()
    );
}

fn chain_graph(sources: &SourceMap, declarations: usize) -> raw::Graph {
    let file = sources.verify_file_id(0).expect("fixture file");
    let span = sources.span(file, 0, 1).expect("fixture span");
    let mut types = vec![
        raw::TypeNode { id: raw::NodeId(0), span: None, kind: raw::TypeKind::Bool },
        raw::TypeNode { id: raw::NodeId(1), span: None, kind: raw::TypeKind::I32 },
        raw::TypeNode { id: raw::NodeId(2), span: None, kind: raw::TypeKind::String },
    ];
    for declaration in 0..declarations {
        let id = u32::try_from(types.len()).expect("fixture type ID");
        let child = if declaration == 0 { raw::NodeId(0) } else { raw::NodeId(id - 1) };
        types.push(raw::TypeNode {
            id: raw::NodeId(id),
            span: Some(span),
            kind: raw::TypeKind::Struct {
                module: raw::ModuleId(0),
                declaration: u32::try_from(declaration).expect("fixture declaration"),
                fields: vec![raw::Field { ordinal: 0, ty: child }],
            },
        });
    }
    raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: u32::try_from(declarations).expect("fixture declaration count"),
        }],
        types,
        program_roots: Vec::new(),
    }
}

#[test]
fn accepts_exact_depth_and_rejects_first_extra() {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: "x".into() }])
            .expect("depth source");
    let exact = chain_graph(&sources, super::MAX_TRAVERSAL_DEPTH - 1);
    verify(&exact, &sources, StorageTarget::Linear32V1).expect("exact depth must verify");
    let extra = chain_graph(&sources, super::MAX_TRAVERSAL_DEPTH);
    let diagnostics = verify(&extra, &sources, StorageTarget::Linear32V1)
        .expect_err("depth first-extra must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3201"));
}

#[test]
fn reports_a_deterministic_indirect_cycle() {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: "ab".into() }])
            .expect("cycle source");
    let file = sources.verify_file_id(0).expect("cycle file");
    let span = sources.span(file, 0, 1).expect("cycle span");
    let graph = raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: 2,
        }],
        types: vec![
            raw::TypeNode { id: raw::NodeId(0), span: None, kind: raw::TypeKind::Bool },
            raw::TypeNode { id: raw::NodeId(1), span: None, kind: raw::TypeKind::I32 },
            raw::TypeNode { id: raw::NodeId(2), span: None, kind: raw::TypeKind::String },
            raw::TypeNode {
                id: raw::NodeId(3),
                span: Some(span),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 0,
                    fields: vec![raw::Field { ordinal: 0, ty: raw::NodeId(4) }],
                },
            },
            raw::TypeNode {
                id: raw::NodeId(4),
                span: Some(span),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 1,
                    fields: vec![raw::Field { ordinal: 0, ty: raw::NodeId(3) }],
                },
            },
        ],
        program_roots: Vec::new(),
    };
    let diagnostics =
        verify(&graph, &sources, StorageTarget::Linear32V1).expect_err("indirect cycle must fail");
    let cycle = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-L3002")
        .expect("cycle diagnostic");
    assert!(cycle.message().contains("[3, 4]"));
}

#[test]
fn cycle_path_uses_true_ascending_depth_first_edges() {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: "abcd".into() }])
            .expect("cycle source");
    let file = sources.verify_file_id(0).expect("cycle file");
    let spans = (0..4)
        .map(|start| sources.span(file, start, start + 1).expect("cycle span"))
        .collect::<Vec<_>>();
    let fields = |children: &[u32]| {
        children
            .iter()
            .enumerate()
            .map(|(ordinal, child)| raw::Field {
                ordinal: u32::try_from(ordinal).expect("ordinal"),
                ty: raw::NodeId(*child),
            })
            .collect()
    };
    let graph = raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: 4,
        }],
        types: vec![
            raw::TypeNode { id: raw::NodeId(0), span: None, kind: raw::TypeKind::Bool },
            raw::TypeNode { id: raw::NodeId(1), span: None, kind: raw::TypeKind::I32 },
            raw::TypeNode { id: raw::NodeId(2), span: None, kind: raw::TypeKind::String },
            raw::TypeNode {
                id: raw::NodeId(3),
                span: Some(spans[0]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 0,
                    fields: fields(&[4]),
                },
            },
            raw::TypeNode {
                id: raw::NodeId(4),
                span: Some(spans[1]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 1,
                    fields: fields(&[5, 6]),
                },
            },
            raw::TypeNode {
                id: raw::NodeId(5),
                span: Some(spans[2]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 2,
                    fields: fields(&[6]),
                },
            },
            raw::TypeNode {
                id: raw::NodeId(6),
                span: Some(spans[3]),
                kind: raw::TypeKind::Struct {
                    module: raw::ModuleId(0),
                    declaration: 3,
                    fields: fields(&[3]),
                },
            },
        ],
        program_roots: Vec::new(),
    };
    let diagnostics = verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect_err("branching recursion must fail");
    let cycle = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-L3002")
        .expect("cycle diagnostic");
    assert!(cycle.message().contains("[3, 4, 5, 6]"), "{}", cycle.message());
}

#[test]
fn enforces_member_array_and_checked_object_boundaries() {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: "x".into() }])
            .expect("boundary source");
    let mut exact_members = chain_graph(&sources, 1);
    let raw::TypeKind::Struct { fields, .. } = &mut exact_members.types[3].kind else {
        panic!("fixture struct")
    };
    *fields = (0..super::MAX_MEMBERS_PER_DECLARATION)
        .map(|ordinal| raw::Field {
            ordinal: u32::try_from(ordinal).expect("fixture ordinal"),
            ty: raw::NodeId(0),
        })
        .collect();
    verify(&exact_members, &sources, StorageTarget::Linear32V1)
        .expect("exact member limit must verify");
    let mut extra_members = exact_members.clone();
    let raw::TypeKind::Struct { fields, .. } = &mut extra_members.types[3].kind else {
        panic!("fixture struct")
    };
    fields.push(raw::Field {
        ordinal: u32::try_from(super::MAX_MEMBERS_PER_DECLARATION).expect("fixture ordinal"),
        ty: raw::NodeId(0),
    });
    let diagnostics = verify(&extra_members, &sources, StorageTarget::Linear32V1)
        .expect_err("member first-extra must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3201"));

    let mut arrays = chain_graph(&sources, 1);
    arrays.types.extend([
        raw::TypeNode {
            id: raw::NodeId(4),
            span: None,
            kind: raw::TypeKind::FixedArray { element: raw::NodeId(0), length: 0 },
        },
        raw::TypeNode {
            id: raw::NodeId(5),
            span: None,
            kind: raw::TypeKind::FixedArray {
                element: raw::NodeId(4),
                length: super::MAX_ARRAY_LENGTH,
            },
        },
    ]);
    arrays.program_roots.push(raw::NodeId(5));
    verify(&arrays, &sources, StorageTarget::Linear32V1)
        .expect("exact array limit with zero-sized element must verify");

    let mut overflow = chain_graph(&sources, 1);
    overflow.types.extend([
        raw::TypeNode {
            id: raw::NodeId(4),
            span: None,
            kind: raw::TypeKind::FixedArray {
                element: raw::NodeId(2),
                length: super::MAX_ARRAY_LENGTH,
            },
        },
        raw::TypeNode {
            id: raw::NodeId(5),
            span: None,
            kind: raw::TypeKind::FixedArray {
                element: raw::NodeId(4),
                length: super::MAX_ARRAY_LENGTH,
            },
        },
    ]);
    overflow.program_roots.push(raw::NodeId(5));
    let diagnostics = verify(&overflow, &sources, StorageTarget::Linear32V1)
        .expect_err("object size overflow must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3005"));
    assert_eq!(super::checked_align_up(u64::MAX, 8), None);
    assert_eq!(super::checked_storage_add(7, 1, 8), Some(8));
    assert_eq!(super::checked_storage_add(8, 1, 8), None);
    assert_eq!(super::checked_storage_mul(4, 2, 8), Some(8));
    assert_eq!(super::checked_storage_mul(4, 3, 8), None);
    assert_eq!(super::checked_align_up_with_limit(7, 8, 8), Some(8));
    assert_eq!(super::checked_align_up_with_limit(9, 8, 8), None);
    assert_eq!(
        super::checked_budget_total(
            super::MAX_DEPENDENCY_EDGES - 1,
            1,
            super::MAX_DEPENDENCY_EDGES
        ),
        Some(super::MAX_DEPENDENCY_EDGES)
    );
    assert_eq!(
        super::checked_budget_total(super::MAX_DEPENDENCY_EDGES, 1, super::MAX_DEPENDENCY_EDGES),
        None
    );
}

#[test]
fn enforces_enum_variant_declaration_and_total_member_boundaries() {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: "x".into() }])
            .expect("enum boundary source");
    let file = sources.verify_file_id(0).expect("enum boundary file");
    let span = sources.span(file, 0, 1).expect("enum boundary span");
    let variants = |count: usize| {
        (0..count)
            .map(|ordinal| raw::Variant {
                ordinal: u32::try_from(ordinal).expect("variant ordinal"),
                payload: None,
            })
            .collect::<Vec<_>>()
    };
    let graph = |declaration_count: usize, final_count: usize| {
        let mut types = vec![
            raw::TypeNode { id: raw::NodeId(0), span: None, kind: raw::TypeKind::Bool },
            raw::TypeNode { id: raw::NodeId(1), span: None, kind: raw::TypeKind::I32 },
            raw::TypeNode { id: raw::NodeId(2), span: None, kind: raw::TypeKind::String },
        ];
        for declaration in 0..declaration_count {
            let count = if declaration + 1 == declaration_count {
                final_count
            } else {
                super::MAX_MEMBERS_PER_DECLARATION
            };
            types.push(raw::TypeNode {
                id: raw::NodeId(u32::try_from(types.len()).expect("type id")),
                span: Some(span),
                kind: raw::TypeKind::Enum {
                    module: raw::ModuleId(0),
                    declaration: u32::try_from(declaration).expect("declaration"),
                    variants: variants(count),
                },
            });
        }
        raw::Graph {
            modules: vec![raw::Module {
                id: raw::ModuleId(0),
                source_file: file,
                data_declarations: u32::try_from(declaration_count).expect("declarations"),
            }],
            types,
            program_roots: Vec::new(),
        }
    };

    verify(&graph(1, super::MAX_MEMBERS_PER_DECLARATION), &sources, StorageTarget::Linear32V1)
        .expect("exact per-enum limit must verify");
    let diagnostics = verify(
        &graph(1, super::MAX_MEMBERS_PER_DECLARATION + 1),
        &sources,
        StorageTarget::Linear32V1,
    )
    .expect_err("first extra enum variant must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3201"));

    verify(&graph(64, 1024), &sources, StorageTarget::Linear32V1)
        .expect("exact total variant limit must verify");
    let diagnostics = verify(&graph(65, 1), &sources, StorageTarget::Linear32V1)
        .expect_err("first extra total variant must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3201"));
}

#[test]
fn rejects_missing_duplicate_unknown_and_orphan_graph_claims() {
    let sources = sources();

    let mut missing = five_type_graph(&sources);
    missing.types[4].id = raw::NodeId(9);
    let diagnostics = verify(&missing, &sources, StorageTarget::Linear32V1)
        .expect_err("missing dense identity must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3001"));

    let mut duplicate_raw = five_type_graph(&sources);
    duplicate_raw.types[4].id = raw::NodeId(3);
    let diagnostics = verify(&duplicate_raw, &sources, StorageTarget::Linear32V1)
        .expect_err("duplicate raw TypeId must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3001"));

    let mut duplicate = five_type_graph(&sources);
    duplicate.types.push(duplicate.types[1].clone());
    duplicate.types[5].id = raw::NodeId(5);
    let diagnostics = verify(&duplicate, &sources, StorageTarget::Linear32V1)
        .expect_err("duplicate nominal identity must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3001"));

    let mut unknown = five_type_graph(&sources);
    let raw::TypeKind::Struct { fields, .. } = &mut unknown.types[1].kind else {
        panic!("fixture Pair")
    };
    fields[0].ty = raw::NodeId(99);
    let diagnostics = verify(&unknown, &sources, StorageTarget::Linear32V1)
        .expect_err("unknown field type must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3003"));

    let mut orphan = five_type_graph(&sources);
    orphan.types.push(raw::TypeNode {
        id: raw::NodeId(5),
        span: None,
        kind: raw::TypeKind::Vec { element: raw::NodeId(0) },
    });
    let diagnostics = verify(&orphan, &sources, StorageTarget::Linear32V1)
        .expect_err("orphan container must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3003"));

    let full_sources = full_sources();
    let mut unknown_payload = full_graph(&full_sources);
    let raw::TypeKind::Enum { variants, .. } = &mut unknown_payload.types[8].kind else {
        panic!("fixture enum")
    };
    variants[1].payload = Some(raw::NodeId(u32::MAX));
    let diagnostics = verify(&unknown_payload, &full_sources, StorageTarget::Linear32V1)
        .expect_err("unknown enum payload must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3003"));
}

#[test]
fn diagnostic_budget_is_bounded_and_terminal_is_last() {
    let sources = sources();
    let mut graph = five_type_graph(&sources);
    for _ in 0..super::MAX_DIAGNOSTICS + 32 {
        let id = raw::NodeId(u32::try_from(graph.types.len()).expect("fixture node ID"));
        graph.types.push(raw::TypeNode {
            id,
            span: None,
            kind: raw::TypeKind::Borrow { referent: raw::NodeId(0) },
        });
        graph.program_roots.push(id);
    }
    let diagnostics = verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect_err("diagnostic exhaustion must fail");
    assert_eq!(diagnostics.len(), super::MAX_DIAGNOSTICS);
    assert_eq!(diagnostics.last().map(zryna_diagnostics::Diagnostic::code), Some("ZRYNA-L3201"));
}

#[test]
fn diagnostic_budget_retains_the_canonical_smallest_set() {
    let mut errors = super::Errors::default();
    for number in (0..super::MAX_DIAGNOSTICS + 32).rev() {
        errors.push(super::global("ZRYNA-L3003", format!("diagnostic {number:04}"), "fixture"));
    }
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), super::MAX_DIAGNOSTICS);
    assert_eq!(diagnostics[0].message(), "diagnostic 0000");
    assert_eq!(diagnostics[super::MAX_DIAGNOSTICS - 2].message(), "diagnostic 0254");
    assert_eq!(diagnostics.last().map(zryna_diagnostics::Diagnostic::code), Some("ZRYNA-L3201"));
}

#[test]
fn diagnostic_budget_accepts_exact_and_marks_first_extra() {
    let build = |count| {
        let mut errors = super::Errors::default();
        for number in 0..count {
            errors.push(super::global("ZRYNA-L3003", format!("diagnostic {number}"), "fixture"));
        }
        errors.finish()
    };
    let exact = build(super::MAX_DIAGNOSTICS - 1);
    assert_eq!(exact.len(), super::MAX_DIAGNOSTICS - 1);
    assert!(exact.iter().all(|diagnostic| diagnostic.code() != "ZRYNA-L3201"));
    let extra = build(super::MAX_DIAGNOSTICS);
    assert_eq!(extra.len(), super::MAX_DIAGNOSTICS);
    assert_eq!(extra.last().map(zryna_diagnostics::Diagnostic::code), Some("ZRYNA-L3201"));
}

#[test]
fn accepts_exact_type_node_limit_and_rejects_first_extra() {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: String::new() }])
            .expect("type-limit source");
    let file = sources.verify_file_id(0).expect("type-limit file");
    let mut graph = raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: 0,
        }],
        types: vec![
            raw::TypeNode { id: raw::NodeId(0), span: None, kind: raw::TypeKind::Bool },
            raw::TypeNode { id: raw::NodeId(1), span: None, kind: raw::TypeKind::I32 },
            raw::TypeNode { id: raw::NodeId(2), span: None, kind: raw::TypeKind::String },
        ],
        program_roots: Vec::with_capacity(super::MAX_TYPE_NODES - 3),
    };
    for length in 0..super::MAX_TYPE_NODES - 3 {
        let id = raw::NodeId(u32::try_from(graph.types.len()).expect("fixture type ID"));
        graph.types.push(raw::TypeNode {
            id,
            span: None,
            kind: raw::TypeKind::FixedArray {
                element: raw::NodeId(0),
                length: u64::try_from(length).expect("fixture array length"),
            },
        });
        graph.program_roots.push(id);
    }
    verify(&graph, &sources, StorageTarget::Linear32V1).expect("exact type-node limit must verify");
    let id = raw::NodeId(u32::try_from(graph.types.len()).expect("fixture extra type ID"));
    graph.types.push(raw::TypeNode {
        id,
        span: None,
        kind: raw::TypeKind::FixedArray { element: raw::NodeId(0), length: 65_533 },
    });
    graph.program_roots.push(id);
    let diagnostics = verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect_err("type-node first-extra must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3201"));
}

#[test]
fn accepts_exact_total_member_limit_and_rejects_first_extra() {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: "x".into() }])
            .expect("member-limit source");
    let file = sources.verify_file_id(0).expect("member-limit file");
    let span = sources.span(file, 0, 1).expect("member-limit span");
    let declarations = super::MAX_MEMBERS / super::MAX_MEMBERS_PER_DECLARATION;
    let mut types = vec![
        raw::TypeNode { id: raw::NodeId(0), span: None, kind: raw::TypeKind::Bool },
        raw::TypeNode { id: raw::NodeId(1), span: None, kind: raw::TypeKind::I32 },
        raw::TypeNode { id: raw::NodeId(2), span: None, kind: raw::TypeKind::String },
    ];
    for declaration in 0..declarations {
        let id = raw::NodeId(u32::try_from(types.len()).expect("fixture type ID"));
        let fields = (0..super::MAX_MEMBERS_PER_DECLARATION)
            .map(|ordinal| raw::Field {
                ordinal: u32::try_from(ordinal).expect("fixture ordinal"),
                ty: raw::NodeId(0),
            })
            .collect();
        types.push(raw::TypeNode {
            id,
            span: Some(span),
            kind: raw::TypeKind::Struct {
                module: raw::ModuleId(0),
                declaration: u32::try_from(declaration).expect("fixture declaration"),
                fields,
            },
        });
    }
    let mut graph = raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: u32::try_from(declarations).expect("fixture declaration count"),
        }],
        types,
        program_roots: Vec::new(),
    };
    verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect("exact total-member limit must verify");
    let id = raw::NodeId(u32::try_from(graph.types.len()).expect("fixture extra type ID"));
    graph.types.push(raw::TypeNode {
        id,
        span: Some(span),
        kind: raw::TypeKind::Struct {
            module: raw::ModuleId(0),
            declaration: u32::try_from(declarations).expect("fixture extra declaration"),
            fields: vec![raw::Field { ordinal: 0, ty: raw::NodeId(0) }],
        },
    });
    graph.modules[0].data_declarations += 1;
    let diagnostics = verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect_err("total-member first-extra must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-L3201"));
}
