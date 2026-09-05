use super::*;
use zryna_layout::{StorageTarget, TypeCategory, raw as layout_raw};
use zryna_source::Span;

fn layouts(sources: &SourceMap) -> zryna_layout::VerifiedLayouts {
    let kinds = [
        layout_raw::TypeKind::Bool,
        layout_raw::TypeKind::I32,
        layout_raw::TypeKind::String,
        layout_raw::TypeKind::Weak { payload: layout_raw::NodeId(2) },
        layout_raw::TypeKind::Shared { payload: layout_raw::NodeId(2) },
    ];
    let graph = layout_raw::Graph {
        modules: vec![layout_raw::Module {
            id: layout_raw::ModuleId(0),
            source_file: sources.verify_file_id(0).expect("source file"),
            data_declarations: 0,
        }],
        types: kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| layout_raw::TypeNode {
                id: layout_raw::NodeId(u32::try_from(index).expect("five types")),
                span: None,
                kind,
            })
            .collect(),
        program_roots: vec![layout_raw::NodeId(3), layout_raw::NodeId(4)],
    };
    zryna_layout::verify(&graph, sources, StorageTarget::Linear32V1).expect("sealed types")
}

fn skeleton(
    at: Span,
    integer: raw::TypeId,
    shared: raw::TypeId,
    errors: &mut Errors<'_>,
) -> OwnedCfgState {
    let mut cfg = OwnedCfgState::single_block(at, errors).expect("entry");
    cfg.seed_function_parameter(
        &raw::ValueDefinition { id: raw::ValueId(0), ty: integer, span: at },
        errors,
    )
    .expect("ordinary parameter");
    let success = cfg.reserve_block(at, errors).expect("success");
    let expired = cfg.reserve_block(at, errors).expect("expired");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::WeakUpgradeBranch {
                weak: raw::PlaceId(0),
                success: raw::Edge { target: success, arguments: vec![raw::ValueId(0)] },
                expired: raw::Edge { target: expired, arguments: vec![raw::ValueId(0)] },
                cleanup: raw::CleanupPlanId(0),
            },
        },
        errors
    ));
    cfg.begin_block(
        success,
        vec![
            raw::ValueDefinition { id: raw::ValueId(1), ty: shared, span: at },
            raw::ValueDefinition { id: raw::ValueId(2), ty: integer, span: at },
        ],
        at,
        errors,
    )
    .expect("success schema");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(2),
                cleanup: raw::CleanupPlanId(1),
            }
        },
        errors
    ));
    cfg.begin_block(
        expired,
        vec![raw::ValueDefinition { id: raw::ValueId(3), ty: integer, span: at }],
        at,
        errors,
    )
    .expect("expired schema");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(3),
                cleanup: raw::CleanupPlanId(2),
            }
        },
        errors
    ));
    cfg
}

#[test]
fn owned_cfg_upgrade_success_prefix_is_sealed_and_ordinary_arguments_remain_exact() {
    let sources = sources_for("x");
    let at = sources.span(sources.verify_file_id(0).expect("file"), 0, 1).expect("span");
    let layouts = layouts(&sources);
    let ty = |category| {
        raw::TypeId(
            layouts.types().find(|ty| ty.category() == category).expect("type").id().index(),
        )
    };
    let integer = ty(TypeCategory::I32);
    let shared = ty(TypeCategory::Shared);
    let places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: ty(TypeCategory::Weak),
        span: at,
        kind: raw::PlaceKind::Parameter(0),
    }];
    let mut baseline = None;
    for mutation in 0..8 {
        let mut errors = Errors::new(&sources);
        let mut cfg = skeleton(at, integer, shared, &mut errors);
        let mut places = places.clone();
        match mutation {
            0 | 7 => {}
            1 => cfg.arena.blocks[1].parameters[0].ty = integer,
            2 => {
                cfg.arena.blocks[1].parameters.remove(0);
            }
            3 => cfg.arena.blocks[2].parameters[0].ty = shared,
            4 => places[0].ty = integer,
            5 => places[0].id = raw::PlaceId(1),
            6 => cfg.arena.blocks[1].parameters[1].ty = shared,
            _ => unreachable!("bounded cases"),
        }
        let result = if mutation == 7 {
            cfg.finish(at, &mut errors)
        } else {
            cfg.finish_with_layouts(Some((&layouts, &places)), at, &mut errors)
        };
        if mutation == 0 {
            baseline = Some(result.expect("success prefix plus ordinary arguments"));
            assert!(errors.finish().is_empty());
        } else {
            assert!(result.is_none(), "mutation {mutation}");
            let diagnostics = errors.finish();
            assert_eq!(diagnostics.len(), 1, "mutation {mutation}");
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }
    let mut errors = Errors::new(&sources);
    let recovered = skeleton(at, integer, shared, &mut errors).finish_with_layouts(
        Some((&layouts, &places)),
        at,
        &mut errors,
    );
    assert_eq!(recovered, baseline);
    assert!(errors.finish().is_empty());
}
