//! Genuine deep source/DTO pair; no fabricated graph or runtime execution proof.
use super::super::{Resolver, authenticate, inventory, resolve};
use super::{
    Closed, Fact, Failure, Frame, Graph, Originals, WorldBuilder, copy_key, key, symbolic_fact,
};
use zryna_source::SourceFileInput;
use zryna_source::SourceMap;
use zryna_syntax::v5::{self, VerifiedProjectSyntaxV5};
fn project(provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-original-deep");
    let source =
        std::fs::read_to_string(directory.join("main.zry")).expect("pre-provider frozen source");
    let packet = std::fs::read(directory.join(format!("deep-indirect-original-{provider}.json")))
        .expect("fresh actual named provider packet");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
        .expect("actual original source authority");
    let syntax =
        v5::verify_snapshot(v5::decode_snapshot(&packet).expect("complete strict DTO"), &sources)
            .expect("actual authenticated original hierarchy");
    (sources, syntax)
}
fn query(originals: &Originals<'_>, key: &[u8], spent: &mut usize) -> Result<Fact, Failure> {
    symbolic_fact(
        originals,
        Frame { kind: 0, module: 0, index: 0 },
        &Closed::Stored(copy_key(key, &mut 0)?),
        spent,
    )
}
#[test]
fn actual_deep_indirect_original_query_has_no_host_recursion_limit_and_keeps_opaque_proof() {
    for provider in ["native", "typescript"] {
        let (sources, syntax) = project(provider);
        assert_eq!(syntax.files()[0].data_declarations.len(), 131);
        let mut spent = 0;
        let inventory = inventory(&syntax, &sources, &mut spent).expect("actual source inventory");
        let originals =
            authenticate(&inventory, &syntax, &mut spent).expect("actual bounded binder");
        let function = &originals.units[0].functions[0];
        let opaque = [0x30];
        let closed = [1];
        let opaque_resolver = Resolver {
            originals: &originals,
            module: 0,
            parameters: function.type_parameters.as_ref(),
            arguments: vec![opaque.as_slice()],
        };
        let closed_resolver = Resolver {
            originals: &originals,
            module: 0,
            parameters: function.type_parameters.as_ref(),
            arguments: vec![closed.as_slice()],
        };
        let opaque_ty = resolve(&opaque_resolver, function.parameters[0].type_syntax, &mut spent)
            .expect("actual E0<T> original occurrence");
        let closed_ty = resolve(&closed_resolver, function.parameters[0].type_syntax, &mut spent)
            .expect("actual E0<i32> original substitution");
        let owner = Frame { kind: 0, module: 0, index: 0 };
        spent = 0;
        assert_eq!(
            symbolic_fact(&originals, owner, &opaque_ty, &mut spent)
                .expect("262-node original path"),
            Fact { copy: false, clone: false }
        );
        spent = 0;
        assert_eq!(
            symbolic_fact(&originals, owner, &closed_ty, &mut spent)
                .expect("same deep source with valid closed Clone leaf"),
            Fact { copy: false, clone: true }
        );
        let Closed::Stored(opaque_key) = opaque_ty else {
            panic!("stored original");
        };
        let mut graph =
            Graph { originals: &originals, owner, builder: WorldBuilder::new(), known: Vec::new() };
        spent = 0;
        let root = graph
            .discover(&opaque_key, &mut spent)
            .expect("explicit task stack, all source members");
        assert_eq!(graph.builder.len(), 262, "131 nominals +130 Vec +one actual opaque slot");
        let world = graph.builder.seal(&mut spent).expect("genuine graph validity/storage/GFP");
        assert_eq!(world.fact(root), Some(Fact { copy: false, clone: false }));
        let shared =
            key::encode(0x22, &[], &[opaque_key], &mut spent).expect("compiler-owned Shared root");
        spent = 0;
        assert_eq!(
            symbolic_fact(&originals, owner, &shared, &mut spent)
                .expect("indirect count Clone terminal still validates original descendant"),
            Fact { copy: false, clone: true }
        );
    }
}
#[test]
fn deep_single_root_work_exact_first_extra_rolls_back_and_retries() {
    let (sources, syntax) = project("native");
    let mut preparation = 0;
    let inventory = inventory(&syntax, &sources, &mut preparation).expect("inventory");
    let originals = authenticate(&inventory, &syntax, &mut preparation).expect("binder");
    let function = &originals.units[0].functions[0];
    let resolver = Resolver {
        originals: &originals,
        module: 0,
        parameters: function.type_parameters.as_ref(),
        arguments: vec![&[0x30]],
    };
    let Closed::Stored(key) =
        resolve(&resolver, function.parameters[0].type_syntax, &mut preparation)
            .expect("source original root")
    else {
        panic!("stored root");
    };
    let mut cost = 0;
    query(&originals, &key, &mut cost).expect("complete deep source query");
    let mut exact = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS - cost;
    query(&originals, &key, &mut exact).expect("exact deep cold credit");
    assert_eq!(exact, crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS);
    let mut short = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS - cost + 1;
    let before = short;
    let Failure::Diagnostics(errors) =
        query(&originals, &key, &mut short).expect_err("first extra deep work")
    else {
        panic!("owning resource diagnostic");
    };
    assert_eq!(errors[0].code(), "ZRYNA-I3201");
    assert_eq!(short, before);
    query(&originals, &key, &mut 0).expect("pristine deep retry");
}
