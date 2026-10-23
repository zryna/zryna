//! Source-authenticated nominal members and public hostile claim sealing.
//! Layout claims below are authored independently of all source/body producers.
use super::authorities::{key, layouts_graph};
use crate::generic_v1::{Failure, owned_v2, raw as graph, source::Originals};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{self, VerifiedProjectSyntaxV5};

mod public_seal;

struct Fixture {
    sources: SourceMap,
    syntax: VerifiedProjectSyntaxV5,
    graph: layout::raw::Graph,
}
impl Fixture {
    fn new(form: u8) -> Self {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/m7-generic-owned-nominal-enum");
        let values =
            std::fs::read_to_string(root.join("values.zry")).expect("nominal original values");
        let main = std::fs::read_to_string(root.join("main.zry")).expect("nominal original main");
        let files = if form == 1 {
            vec![SourceFileInput { path: "main.zry".into(), text: format!("{values}\n{main}") }]
        } else {
            let imports = std::fs::read_to_string(root.join("imports.txt"))
                .expect("nominal explicit imports");
            vec![
                SourceFileInput {
                    path: "main.zry".into(),
                    text: format!("{}\n{main}", imports.trim_end()),
                },
                SourceFileInput { path: "values.zry".into(), text: values },
            ]
        };
        let sources = SourceMap::build(files).expect("exact nominal source closure");
        let bytes = std::fs::read(root.join(format!("{form}-reference.json")))
            .expect("independently emitted complete native/worker v5 DTO");
        let syntax =
            v5::verify_snapshot(v5::decode_snapshot(&bytes).expect("nominal v5 DTO"), &sources)
                .expect("complete source/arena/token authentication");
        let unit = syntax
            .files()
            .iter()
            .find(|unit| !unit.data_declarations.is_empty())
            .expect("nominal declarations");
        assert_eq!(unit.data_declarations.len(), 5);
        let spans = unit
            .data_declarations
            .iter()
            .map(|data| sources.verify_span(data.span).expect("original nominal span"))
            .collect::<Vec<_>>();
        let mut graph = layouts_graph(unit.id, &spans);
        graph.declarations.push(layout::raw::Declaration {
            module: layout::raw::ModuleId(unit.id),
            index: 4,
            kind: layout::raw::NominalKind::Enum,
            parameters: 1,
            members: 2,
            span: spans[4],
        });
        graph.modules = syntax
            .files()
            .iter()
            .map(|unit| layout::raw::Module {
                id: layout::raw::ModuleId(unit.id),
                source_file: sources.verify_file_id(unit.id).expect("file"),
                data_declarations: u32::try_from(unit.data_declarations.len())
                    .expect("declarations"),
            })
            .collect();
        Self { sources, syntax, graph }
    }
    fn layouts(
        &self,
        graph: &layout::raw::Graph,
    ) -> (layout::VerifiedLayouts, layout::VerifiedLayouts) {
        (
            layout::verify(graph, &self.sources, StorageTarget::Linear32V1)
                .expect("independently valid physical Linear32 layout"),
            layout::verify(graph, &self.sources, StorageTarget::LinuxX8664V1)
                .expect("independently valid physical Linux layout"),
        )
    }
    fn inventory(
        &self,
        linear: &layout::VerifiedLayouts,
        linux: &layout::VerifiedLayouts,
    ) -> graph::Program {
        graph::Program {
            modules: self
                .syntax
                .files()
                .iter()
                .map(|unit| graph::Module {
                    id: unit.id,
                    functions: u32::try_from(unit.functions.len()).expect("functions"),
                })
                .collect(),
            declarations: self
                .syntax
                .files()
                .iter()
                .flat_map(|unit| {
                    unit.functions.iter().enumerate().map(move |(index, f)| graph::Declaration {
                        module: unit.id,
                        function: u32::try_from(index).expect("function"),
                        parameters: u32::try_from(
                            f.type_parameters.as_ref().map_or(0, |p| p.parameters.len()),
                        )
                        .expect("arity"),
                        span: f.span,
                    })
                })
                .collect(),
            type_keys: linear.types().map(|ty| ty.key().to_vec()).collect(),
            universe: *linear.universe_identity(),
            linear32: *linear.fingerprint(),
            linux_x86_64: *linux.fingerprint(),
            functions: vec![],
        }
    }
    fn substituted(&self, graph: &layout::raw::Graph) -> Result<(), Failure> {
        let (linear, linux) = self.layouts(graph);
        let inventory = self.inventory(&linear, &linux);
        let originals = Originals::check(&inventory, &self.syntax)?;
        crate::generic_v1::substitution::check(&inventory, &linear, &originals)
    }
}

#[test]
fn both_authenticated_forms_bind_every_nominal_member_and_sealed_original_inventory() {
    for form in [1, 2] {
        let f = Fixture::new(form);
        f.substituted(&f.graph)
            .expect("independent exact original substitution without a body producer");
        for attack in 0..11 {
            let mut graph = f.graph.clone();
            match attack {
                0..=4 => {
                    let (node, ordinal, payload) = match attack {
                        0 => (5, 1, Some(0)),
                        1 => (5, 2, Some(2)),
                        2 => (5, 3, None),
                        3 => (6, 1, Some(2)),
                        _ => (7, 1, Some(1)),
                    };
                    let layout::raw::TypeKind::Enum { variants, .. } = &mut graph.types[node].kind
                    else {
                        panic!("generic enum")
                    };
                    variants[ordinal].payload = payload.map(layout::raw::NodeId);
                }
                5 => {
                    let layout::raw::TypeKind::Base(zryna_layout::raw::TypeKind::Enum {
                        variants,
                        ..
                    }) = &mut graph.types[8].kind
                    else {
                        panic!("plain enum")
                    };
                    variants[1].payload = Some(layout::raw::NodeId(1));
                }
                6 => {
                    let layout::raw::TypeKind::Enum { arguments, .. } = &mut graph.types[6].kind
                    else {
                        panic!("marker")
                    };
                    arguments.push(layout::raw::NodeId(1));
                    graph.declarations[1].parameters = 2;
                }
                7 => {
                    let declaration = &mut graph.declarations[0];
                    let span = declaration.span;
                    let changed = f
                        .sources
                        .span(span.file(), span.start() + 1, span.end())
                        .expect("valid shifted range");
                    declaration.span = changed;
                    graph.types[5].span = Some(changed);
                }
                8 => {
                    let mut extra = graph.declarations[1].clone();
                    extra.index = 5;
                    graph.declarations.push(extra);
                    graph
                        .modules
                        .iter_mut()
                        .find(|module| module.data_declarations != 0)
                        .expect("data module")
                        .data_declarations = 6;
                }
                9 => graph.declarations[4].members = 1,
                _ => {
                    graph.declarations.pop();
                    graph
                        .modules
                        .iter_mut()
                        .find(|module| module.data_declarations != 0)
                        .expect("data module")
                        .data_declarations = 4;
                }
            }
            let Failure::Diagnostics(errors) = f
                .substituted(&graph)
                .expect_err("physical layout grants no original member or inventory substitution")
            else {
                panic!("diagnostic")
            };
            assert_eq!(errors[0].code, "ZRYNA-I7001");
            f.substituted(&f.graph)
                .expect("pristine source-bound member replay after every attack");
        }
    }
}

#[test]
fn original_nominal_templates_are_checked_before_any_closed_instance_or_runtime() {
    for form in [1, 2] {
        let f = Fixture::new(form);
        owned_v2::check_original_bodies(&f.syntax, &f.sources)
            .expect("all nominal original bodies are independently checked with opaque slots");
        let foreign = Fixture::new(form);
        assert!(owned_v2::check_original_bodies(&foreign.syntax, &f.sources).is_err());
        owned_v2::check_original_bodies(&f.syntax, &f.sources).expect("pristine original replay");
    }
}
