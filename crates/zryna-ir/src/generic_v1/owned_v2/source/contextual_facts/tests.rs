//! Genuine source roots exercise contextual affinity and transactional cache accounting.
use super::*;
use crate::generic_v1::raw;
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;

pub(super) fn with(case: &str, test: impl FnOnce(&Originals<'_>)) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-struct-v4-nested");
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: std::fs::read_to_string(root.join(format!("{case}.zry"))).expect("frozen source"),
    }])
    .expect("complete source map");
    let packet = std::fs::read(root.join(format!("{case}-native.json"))).expect("frozen DTO");
    let syntax = v5::verify_snapshot(v5::decode_snapshot(&packet).expect("raw DTO"), &sources)
        .expect("authenticated originals");
    let inventory = raw::Program {
        modules: syntax
            .files()
            .iter()
            .map(|u| raw::Module {
                id: u.id,
                functions: u32::try_from(u.functions.len())
                    .expect("small frozen function inventory"),
            })
            .collect(),
        declarations: syntax
            .files()
            .iter()
            .flat_map(|u| {
                u.functions.iter().enumerate().map(move |(i, f)| raw::Declaration {
                    module: u.id,
                    function: u32::try_from(i).expect("small frozen function index"),
                    parameters: u32::try_from(
                        f.type_parameters.as_ref().map_or(0, |p| p.parameters.len()),
                    )
                    .expect("small frozen arity"),
                    span: f.span,
                })
            })
            .collect(),
        type_keys: vec![],
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        functions: vec![],
    };
    test(&Originals::check(&inventory, &syntax).expect("independent original inventory"));
}
fn key(tag: u8, index: u32, args: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![tag];
    if matches!(tag, 0x10..=0x13) {
        out.extend(0u32.to_le_bytes());
        out.extend(index.to_le_bytes());
    }
    if matches!(tag, 0x12..=0x15) {
        out.extend(u32::try_from(args.len()).expect("small literal argument count").to_le_bytes());
    }
    for child in args {
        out.extend(u32::try_from(child.len()).expect("small literal key length").to_le_bytes());
        out.extend(child);
    }
    out
}
fn owner() -> Frame {
    Frame { kind: 0, module: 0, index: 0 }
}

#[test]
fn actual_members_control_affinity_even_below_compiler_family_roots() {
    for (case, root, affine) in [
        ("nested-copy", key(0x12, 0, &[key(0x12, 0, &[vec![1]])]), false),
        ("option-box-root", key(0x14, 0, &[key(0x12, 0, &[vec![2]])]), true),
        ("box-option-root", key(0x12, 0, &[key(0x14, 0, &[vec![2]])]), true),
        ("phantom-remains-copy", key(0x12, 1, &[key(0x12, 0, &[vec![2]])]), false),
        ("fixed-member-remains-affine", key(0x12, 1, &[vec![1]]), true),
    ] {
        with(case, |originals| {
            let mut facts = Facts::new(originals, owner(), false);
            assert_eq!(
                facts
                    .affine(&Closed::Stored(root), &mut Budget::default())
                    .expect("contextual proof"),
                affine,
                "{case}"
            );
        });
    }
}

#[test]
fn opaque_owner_and_ordered_arguments_are_normalized_identity() {
    with("opaque-copy-assumption", |originals| {
        let mut engine = Engine::new(originals);
        let mut credit = Budget::default();
        let function = engine.opaque(owner(), 0, &mut credit).expect("caller slot");
        let data = engine
            .opaque(Frame { kind: 1, module: 0, index: 0 }, 0, &mut credit)
            .expect("data slot");
        assert_ne!(function, data, "equally numbered function/data owners remain distinct");
        let original = &originals.units[0].data_declarations[0];
        let v5::RawDataDeclarationKind::Struct { fields, .. } = &original.kind else {
            panic!("Box")
        };
        let syntax::Resolved::Stored(substituted) = engine
            .syntax(
                Frame { kind: 1, module: 0, index: 0 },
                &[function],
                fields[0].type_syntax,
                &mut credit,
            )
            .expect("callee member")
        else {
            panic!("stored")
        };
        assert_eq!(
            substituted, function,
            "callee binds to the caller atom without rebinding its owner"
        );
        let forward =
            engine.application(0x15, 0, 0, &[function, data], &mut credit).expect("Result");
        let reverse = engine
            .application(0x15, 0, 0, &[data, function], &mut credit)
            .expect("reversed Result");
        assert_ne!(forward, reverse);
    });
}

#[test]
fn cold_hit_eviction_and_failed_miss_preserve_exact_root_cost_and_last_row() {
    with("nested-copy", |originals| {
        let root = Closed::Stored(key(0x12, 0, &[key(0x12, 0, &[vec![1]])]));
        let mut facts = Facts::new(originals, owner(), false);
        let mut cold = Budget::default();
        assert!(!facts.affine(&root, &mut cold).expect("cold"));
        let cost = cold.nominal_used();
        for evict in [false, true] {
            if evict {
                facts.affine(&Closed::Stored(vec![2]), &mut Budget::default()).expect("eviction");
            }
            let before = facts.last.as_ref().map(|r| (r.key.clone(), r.affine, r.work));
            let start = super::super::super::plan::MAX_OWNER_UNITS - cost + 1;
            let mut short = Budget::original(start);
            assert!(facts.affine(&root, &mut short).is_err());
            let Closed::Stored(bytes) = &root else { unreachable!() };
            assert_eq!(short.nominal_used(), start + 1 + bytes.len());
            assert_eq!(facts.last.as_ref().map(|r| (r.key.clone(), r.affine, r.work)), before);
            let mut retry = Budget::default();
            assert!(!facts.affine(&root, &mut retry).expect("pristine retry"));
            assert_eq!(retry.nominal_used(), cost);
        }
    });
}

#[test]
fn shared_diamond_descendant_is_completed_once_per_request() {
    with("nested-copy", |originals| {
        let mut engine = Engine::new(originals);
        let mut credit = Budget::default();
        let scalar = engine.intern([1, 0, 0, 0, 0, 0], &mut credit).expect("scalar");
        let shared = engine.application(0x12, 0, 0, &[scalar], &mut credit).expect("Box<i32>");
        let left = engine.application(0x14, 0, 0, &[shared], &mut credit).expect("Option");
        let right = engine.application(0x15, 0, 0, &[shared, scalar], &mut credit).expect("Result");
        let diamond = engine.application(0x15, 0, 0, &[left, right], &mut credit).expect("diamond");
        engine.prove(shared, &mut credit).expect("already credited descendant");
        engine.prove(diamond, &mut credit).expect("unique DAG");
        assert_eq!(engine.terms[shared as usize].members.len(), 1);
        assert!(engine.terms.iter().all(|t| t.state == 2));
        assert!(!engine.terms[diamond as usize].fact.expect("complete root").affine);
    });
}

#[test]
fn opaque_large_owner_index_is_never_interpreted_as_child_count() {
    with("opaque-copy-assumption", |originals| {
        let mut engine = Engine::new(originals);
        let mut credit = Budget::default();
        // Isolated identity control, not an invented authenticated declaration.
        let id = engine
            .opaque(Frame { kind: 0, module: 0, index: 9 }, 0, &mut credit)
            .expect("identity row");
        assert!(engine.terms[id as usize].children().is_empty());
        assert!(engine.prove(id, &mut credit).expect("opaque is conservative").affine);
        assert_eq!(engine.encode(id, &mut credit).expect("symbolic encoding"), [0x30]);
    });
}
#[test]
fn key_depth_64_and_first_extra_remain_separate_from_member_height() {
    with("nested-copy", |originals| {
        let mut engine = Engine::new(originals);
        let mut credit = Budget::default();
        let mut id = engine.intern([1, 0, 0, 0, 0, 0], &mut credit).expect("scalar");
        for _ in 0..64 {
            id = engine.application(0x14, 0, 0, &[id], &mut credit).expect("depth64 fits");
        }
        assert_eq!(engine.terms[id as usize].shape.depth, 64);
        assert!(engine.application(0x14, 0, 0, &[id], &mut credit).is_err());
        assert_eq!(
            engine.prove(id, &mut credit).expect("65 stored nodes fit member256").height,
            65
        );
    });
}

#[test]
fn argument_backedges_do_not_become_stored_cycles_or_copy_shortcuts() {
    with("nested-copy", |originals| {
        // Isolated dependency-kernel control; these rows grant no source/layout authority.
        let mut engine = Engine::new(originals);
        let mut credit = Budget::default();
        let node = engine.application(0x10, 0, 7, &[], &mut credit).expect("plain key identity");
        let phantom =
            engine.application(0x12, 0, 8, &[node], &mut credit).expect("argument refers to Node");
        let scalar = engine.intern([1, 0, 0, 0, 0, 0], &mut credit).expect("scalar");
        engine.terms[node as usize].members.push(Some(phantom));
        engine.terms[phantom as usize].members.push(Some(scalar));
        engine.expanded = engine.terms.len();
        for term in &mut engine.terms {
            term.state = 1;
        }
        let fact =
            engine.prove(node, &mut credit).expect("only argument validation returns to Node");
        assert!(!fact.affine);
        assert_eq!(fact.height, 3);
        for term in &mut engine.terms {
            term.fact = None;
            term.state = 1;
        }
        engine.terms[phantom as usize].members[0] = Some(node);
        assert!(engine.prove(node, &mut credit).is_err(), "actual stored backedge still rejects");
    });
}
