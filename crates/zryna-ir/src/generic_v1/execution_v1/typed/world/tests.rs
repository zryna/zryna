//! Genuine `layout::verify` views exercise source-world correspondence without `SourceProof`.
use super::record;
use crate::generic_v1::execution_v1::type_world::{Builder, Kind};
use zryna_layout::{
    StorageTarget,
    generic_v1::{self, VerifiedLayouts, raw},
};
use zryna_source::{SourceFileInput, SourceMap};
fn layouts(
    sources: &SourceMap,
    target: StorageTarget,
    length: u64,
    element: u32,
) -> VerifiedLayouts {
    use zryna_layout::raw::TypeKind;
    let kinds = [
        TypeKind::Bool,
        TypeKind::I32,
        TypeKind::String,
        TypeKind::FixedArray { element: raw::NodeId(element), length },
    ];
    let graph = raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: sources.verify_file_id(0).expect("valid checked test fixture"),
            data_declarations: 0,
        }],
        declarations: vec![],
        types: kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| raw::TypeNode {
                id: raw::NodeId(u32::try_from(index).expect("valid checked test fixture")),
                span: None,
                kind: raw::TypeKind::Base(kind),
            })
            .collect(),
        program_roots: (0..4).map(raw::NodeId).collect(),
    };
    generic_v1::verify(&graph, sources, target)
        .expect("genuine independent physical layout authority")
}
#[test]
fn array_world_matches_genuine_stride_and_rejects_other_valid_length_or_element_layouts() {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function entry(): i32 { return 7; }\n".into(),
    }])
    .expect("valid checked test fixture");
    let mut builder = Builder::new();
    builder.declare(Kind::Bool, &[], &mut 0).expect("valid checked test fixture");
    let integer = builder.declare(Kind::I32, &[], &mut 0).expect("valid checked test fixture");
    builder.declare(Kind::String, &[], &mut 0).expect("valid checked test fixture");
    let array = builder
        .declare(Kind::FixedArray { length: 2 }, &[integer], &mut 0)
        .expect("valid checked test fixture");
    let world = builder.seal(&mut 0).expect("valid checked test fixture");
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        let exact = layouts(&sources, target, 2, 1);
        let view = exact
            .types()
            .find(|ty| ty.array_length() == Some(2))
            .expect("valid checked test fixture");
        assert_eq!(view.array_stride(), Some(4));
        assert_eq!(view.size(), 8);
        record(&world, array, view, &mut 0).expect("exact source length/element and sealed stride");
        let wrong_length = layouts(&sources, target, 3, 1);
        let view = wrong_length
            .types()
            .find(|ty| ty.array_length() == Some(3))
            .expect("valid checked test fixture");
        record(&world, array, view, &mut 0)
            .expect_err("valid length3 layout cannot replace source length2");
        let wrong_element = layouts(&sources, target, 2, 0);
        let view = wrong_element
            .types()
            .find(|ty| ty.array_length() == Some(2))
            .expect("valid checked test fixture");
        assert_eq!(view.array_stride(), Some(1), "real bool layout, not fabricated stride");
        record(&world, array, view, &mut 0)
            .expect_err("valid bool array cannot replace exact i32 array");
        let view = exact
            .types()
            .find(|ty| ty.array_length() == Some(2))
            .expect("valid checked test fixture");
        record(&world, array, view, &mut 0).expect("pristine exact retry");
    }
}
