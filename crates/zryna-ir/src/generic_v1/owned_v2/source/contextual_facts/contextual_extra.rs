//! Authenticated source graphs exercise stored edges and declaring-module identities.
//! Kernel qualification only: whole-original aggregate admission is a separate gate.
use super::*;
use crate::generic_v1::raw;
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;

fn directory() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-struct-v4-contextual-extra")
}
fn owner() -> Frame {
    Frame { kind: 0, module: 0, index: 0 }
}
fn key(tag: u8, module: u32, index: u32, args: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend(module.to_le_bytes());
    out.extend(index.to_le_bytes());
    if matches!(tag, 0x12 | 0x13) {
        out.extend(u32::try_from(args.len()).expect("small fixture arity").to_le_bytes());
    }
    for child in args {
        out.extend(u32::try_from(child.len()).expect("small fixture key").to_le_bytes());
        out.extend(child);
    }
    out
}
fn with_originals(case: &str, inspect: impl Fn(&Originals<'_>)) {
    let paths: &[&str] = if case == "imported-alias-decoy" {
        &["main.zry", "values.zry", "z-leaf.zry"]
    } else {
        &["main.zry"]
    };
    let sources = SourceMap::build(
        paths
            .iter()
            .map(|path| SourceFileInput {
                path: (*path).into(),
                text: std::fs::read_to_string(directory().join(case).join(path))
                    .expect("frozen complete source"),
            })
            .collect(),
    )
    .expect("independent exact source map");
    let mut compared: Option<v5::VerifiedProjectSyntaxV5> = None;
    for provider in ["native", "typescript"] {
        let packet = std::fs::read(directory().join(format!("{case}-{provider}.json")))
            .expect("fresh frozen complete DTO");
        let syntax =
            v5::verify_snapshot(v5::decode_snapshot(&packet).expect("closed v5 packet"), &sources)
                .expect("exact complete original source authentication");
        if let Some(previous) = compared.as_ref() {
            assert_eq!(syntax.files(), previous.files());
        }
        // Inventory derives only from authenticated syntax and is independently checked.
        // No type term, member edge, graph row or completed fact is fabricated or modified.
        let inventory = raw::Program {
            modules: syntax
                .files()
                .iter()
                .map(|unit| raw::Module {
                    id: unit.id,
                    functions: u32::try_from(unit.functions.len())
                        .expect("small fixture function count"),
                })
                .collect(),
            declarations: syntax
                .files()
                .iter()
                .flat_map(|unit| {
                    unit.functions.iter().enumerate().map(move |(index, function)| {
                        raw::Declaration {
                            module: unit.id,
                            function: u32::try_from(index).expect("small fixture function index"),
                            parameters: u32::try_from(
                                function.type_parameters.as_ref().map_or(0, |p| p.parameters.len()),
                            )
                            .expect("small fixture arity"),
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
        let originals = Originals::check(&inventory, &syntax).expect("checked original inventory");
        inspect(&originals);
        compared = Some(syntax);
    }
}
fn height_budget(error: Failure) {
    let Failure::Diagnostics(errors) = error else { panic!("structured height budget diagnostic") };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-I3201");
    assert_eq!(
        errors[0].message(),
        "contextual stored path exceeds inherited layout traversal depth"
    );
}

#[test]
fn complete_plain_source_graph_height_256_passes_but_257_exhausts_traversal_budget() {
    // These constants copy the independently frozen *-expected.json, never an observed result.
    for (case, stored_nodes) in [("member-height-256", 256), ("member-height-257", 257)] {
        with_originals(case, |originals| {
            assert_eq!(originals.units[0].data_declarations.len(), stored_nodes - 1);
            let key = key(0x10, 0, 0, &[]);
            assert_eq!(key.len(), 9, "short plain nominal key, no generic key-depth pressure");
            let mut engine = Engine::new(originals);
            let mut credit = Budget::default();
            let root = engine.decode(&key, owner(), false, &mut credit).expect("canonical root");
            assert_eq!(engine.terms[root as usize].shape.depth, 0);
            if stored_nodes == 256 {
                let fact =
                    engine.prove(root, &mut credit).expect("255 original nominals plus scalar");
                assert_eq!(fact.height, 256);
                assert!(!fact.affine);
                assert_eq!(engine.terms.len(), 256);
                assert!(engine.terms.iter().all(|term| term.fact.is_some()));
                let mut facts = Facts::new(originals, owner(), false);
                assert!(
                    !facts
                        .affine(&Closed::Stored(key), &mut Budget::default())
                        .expect("same genuine Facts query")
                );
            } else {
                let Err(error) = engine.prove(root, &mut credit) else {
                    panic!("257 stored nodes exceed256");
                };
                height_budget(error);
                let mut facts = Facts::new(originals, owner(), false);
                height_budget(
                    facts
                        .affine(&Closed::Stored(key), &mut Budget::default())
                        .expect_err("same genuine Facts boundary"),
                );
            }
        });
    }
}

#[test]
fn authenticated_shared_cached_child_height_is_added_to_the_incoming_prefix() {
    with_originals("member-height-shared-prefix", |originals| {
        let mut engine = Engine::new(originals);
        let mut credit = Budget::default();
        let mut cached = None;
        // N001 is cached at255; SharedPass shares it at256; SharedReject reaches257.
        for (index, height) in [(1, 255), (255, 256), (256, 257)] {
            let key = key(0x10, 0, index, &[]);
            assert_eq!(key.len(), 9);
            let root =
                engine.decode(&key, owner(), false, &mut credit).expect("authentic original root");
            assert_eq!(engine.terms[root as usize].shape.depth, 0);
            if height <= 256 {
                let fact = engine.prove(root, &mut credit).expect("genuine source subtree fits");
                assert_eq!(fact.height, height);
                assert!(!fact.affine);
                if cached.is_none() {
                    cached = Some(root);
                }
            } else {
                let Err(error) = engine.prove(root, &mut credit) else {
                    panic!("incoming prefix plus cached height257");
                };
                height_budget(error);
            }
            let retained = engine.terms[cached.expect("preproved authentic N001") as usize]
                .fact
                .expect("retained source-proved cached child");
            assert_eq!(retained.height, 255);
            assert!(!retained.affine);
        }
    });
}

#[test]
fn imported_alias_members_use_declaring_module_targets_despite_affine_caller_decoys() {
    with_originals("imported-alias-decoy", |originals| {
        assert_eq!(originals.units.len(), 3);
        let payload = key(0x10, 2, 0, &[]);
        let nested = key(0x12, 1, 0, std::slice::from_ref(&payload));
        let carrier = key(0x12, 1, 1, &[vec![1]]);
        let decoy_declared = key(0x10, 0, 0, &[]);
        let decoy_box = key(0x12, 0, 1, &[vec![1]]);
        assert_ne!(nested, decoy_box);
        assert_ne!(payload, decoy_declared);
        let mut facts = Facts::new(originals, owner(), false);
        for (key, affine) in [
            (&carrier, false),
            (&nested, false),
            (&payload, false),
            (&decoy_declared, true),
            (&decoy_box, true),
        ] {
            assert_eq!(
                facts
                    .affine(&Closed::Stored(key.clone()), &mut Budget::default())
                    .expect("declaring-module proof"),
                affine
            );
        }
        for (key, expected) in [
            (carrier, vec![("nested", nested.clone()), ("marker", vec![1])]),
            (nested, vec![("value", payload.clone())]),
            (payload, vec![("value", vec![1])]),
        ] {
            let description = facts
                .stored(&key, &mut Budget::default())
                .expect("exact original member description");
            let Closed::Stored(actual) = description.ty else { panic!("stored nominal") };
            assert_eq!(actual, key);
            let observed = description
                .members
                .into_iter()
                .map(|(name, ty)| {
                    let Some(Closed::Stored(key)) = ty else { panic!("stored member") };
                    (name, key)
                })
                .collect::<Vec<_>>();
            assert_eq!(observed, expected, "member resolves to declaring-module original identity");
        }
    });
}
