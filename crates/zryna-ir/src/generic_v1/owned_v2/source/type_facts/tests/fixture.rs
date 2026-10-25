//! Genuine complete source authority for private fact-credit and cache boundary checks.
use crate::generic_v1::{raw, source::Originals};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{self, VerifiedProjectSyntaxV5};

pub(super) struct Fixture {
    sources: SourceMap,
    syntax: VerifiedProjectSyntaxV5,
    inventory: raw::Program,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/m7-generic-owned-nominal-enum");
        let values =
            std::fs::read_to_string(root.join("values.zry")).expect("original declarations");
        let main = std::fs::read_to_string(root.join("main.zry")).expect("original functions");
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: format!("{values}\n{main}"),
        }])
        .expect("complete original source");
        let bytes =
            std::fs::read(root.join("1-reference.json")).expect("independent complete v5 DTO");
        let syntax = v5::verify_snapshot(v5::decode_snapshot(&bytes).expect("v5 DTO"), &sources)
            .expect("genuine source/arena authentication");
        let inventory = raw::Program {
            modules: syntax
                .files()
                .iter()
                .map(|unit| raw::Module {
                    id: unit.id,
                    functions: u32::try_from(unit.functions.len()).expect("functions"),
                })
                .collect(),
            declarations: syntax
                .files()
                .iter()
                .flat_map(|unit| {
                    unit.functions.iter().enumerate().map(move |(index, function)| {
                        raw::Declaration {
                            module: unit.id,
                            function: u32::try_from(index).expect("function"),
                            parameters: u32::try_from(
                                function.type_parameters.as_ref().map_or(0, |p| p.parameters.len()),
                            )
                            .expect("arity"),
                            span: function.span,
                        }
                    })
                })
                .collect(),
            type_keys: vec![],
            universe: [0; 32],
            linear32: [0; 32],
            linux_x86_64: [0; 32],
            functions: vec![],
        };
        Self { sources, syntax, inventory }
    }
    pub(super) fn originals(&self) -> Originals<'_> {
        assert!(self.syntax.is_bound_to(&self.sources));
        Originals::check(&self.inventory, &self.syntax).expect("independent original inventory")
    }
}
