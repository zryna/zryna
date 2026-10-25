//! Sealed layout-only arithmetic witnesses; these do not claim source execution authority.
use zryna_layout::StorageTarget;
use zryna_layout::{
    generic_v1::{self as layout, raw},
    raw as base,
};
use zryna_source::{SourceFileInput, SourceMap};

#[test]
fn affine_struct_drop_work_is_reverse_fields_plus_actual_string_emission() {
    let text = "interface Record extends ZrynaStruct { first:bool; middle:String; last:i32; }";
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.into() }])
            .expect("layout witness source");
    let file = sources.verify_file_id(0).expect("file");
    let span = sources
        .verify_span(zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("source bound"),
        })
        .expect("original declaration span");
    let kinds = [
        raw::TypeKind::Base(base::TypeKind::Bool),
        raw::TypeKind::Base(base::TypeKind::I32),
        raw::TypeKind::Base(base::TypeKind::String),
        raw::TypeKind::Base(base::TypeKind::Struct {
            module: raw::ModuleId(0),
            declaration: 0,
            fields: vec![
                base::Field { ordinal: 0, ty: raw::NodeId(0) },
                base::Field { ordinal: 1, ty: raw::NodeId(2) },
                base::Field { ordinal: 2, ty: raw::NodeId(1) },
            ],
        }),
    ];
    let graph = raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: 1,
        }],
        declarations: vec![raw::Declaration {
            module: raw::ModuleId(0),
            index: 0,
            kind: raw::NominalKind::Struct,
            parameters: 0,
            members: 3,
            span,
        }],
        types: kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| raw::TypeNode {
                id: raw::NodeId(u32::try_from(index).expect("node")),
                span: (index == 3).then_some(span),
                kind,
            })
            .collect(),
        program_roots: (0..4).map(raw::NodeId).collect(),
    };
    let layouts = layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("independently sealed Struct fields");
    let types = layouts.types().collect::<Vec<_>>();
    let mut units = 0;
    let work = super::derive(&types, &mut units).expect("new Struct field recursion");
    let record = types.iter().position(|ty| ty.key().first() == Some(&0x10)).expect("Record");
    // This exact prospective emitter uses forward visits, retained descriptors and reverse
    // visits (3*3), plus String's stack_addr/three stores/call/trapnz (6). No enum dispatch.
    assert_eq!(work[record].units, 15);
    assert!(work[record].contains_nominal);
}
