//! Read-only array metadata over actual independently verified successor records.
use super::{VerifiedLayouts, raw};
use crate::StorageTarget;
use zryna_source::{SourceFileInput, SourceMap};

fn layouts(length: u64, target: StorageTarget) -> VerifiedLayouts {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: String::new() }])
            .expect("same source authority used by layout verification");
    let graph = raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: sources.verify_file_id(0).expect("source file"),
            data_declarations: 0,
        }],
        declarations: vec![],
        types: vec![
            raw::TypeNode {
                id: raw::NodeId(0),
                span: None,
                kind: raw::TypeKind::Base(crate::raw::TypeKind::Bool),
            },
            raw::TypeNode {
                id: raw::NodeId(1),
                span: None,
                kind: raw::TypeKind::Base(crate::raw::TypeKind::I32),
            },
            raw::TypeNode {
                id: raw::NodeId(2),
                span: None,
                kind: raw::TypeKind::Base(crate::raw::TypeKind::String),
            },
            raw::TypeNode {
                id: raw::NodeId(3),
                span: None,
                kind: raw::TypeKind::Base(crate::raw::TypeKind::FixedArray {
                    element: raw::NodeId(1),
                    length,
                }),
            },
        ],
        program_roots: vec![raw::NodeId(3)],
    };
    super::verify(&graph, &sources, target).expect("actual sealed target array records")
}

#[test]
fn successor_array_length_and_stride_preserve_zero_and_target_record_identity() {
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        for length in [0, 2] {
            let layouts = layouts(length, target);
            let before = layouts.canonical_bytes().to_vec();
            let fingerprint = *layouts.fingerprint();
            let array = layouts
                .types()
                .find(|ty| ty.category() == crate::TypeCategory::FixedArray)
                .expect("verified array");
            assert_eq!(array.array_length(), Some(length));
            assert_eq!(array.array_stride(), Some(4));
            assert_eq!(array.size(), length * 4);
            let element = layouts
                .type_by_id(array.referenced_type().expect("same-universe array element"))
                .expect("retained branded element");
            assert_eq!(element.key(), &[1]);
            assert_eq!(element.array_length(), None);
            assert_eq!(element.array_stride(), None);
            assert_eq!(layouts.canonical_bytes(), before);
            assert_eq!(layouts.fingerprint(), &fingerprint);
        }
    }
}
