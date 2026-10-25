//! Source roots are independent of executable function uses and raw universe membership.
use super::support::{PROVIDERS, seal, with_project};
use zryna_ir::generic_v1::owned_v2 as owned;
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot};
mod claims;

fn project(case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-struct-v4-demand");
    let sources: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("sources.json")).expect("literal complete originals"),
    )
    .expect("source JSON");
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: sources[case].as_str().expect("literal case").to_owned(),
    }])
    .expect("source identity");
    let packet = std::fs::read(root.join(format!("{case}-{provider}.json")))
        .expect("actual provider candidate");
    let syntax = verify_snapshot(decode_snapshot(&packet).expect("closed raw DTO"), &sources)
        .expect("complete original source authentication");
    (sources, syntax)
}

#[test]
fn mandatory_plain_and_unused_closed_template_roots_seal_for_both_actual_providers() {
    for provider in PROVIDERS {
        for case in ["plain-struct", "closed-template"] {
            let (sources, syntax) = project(case, provider);
            with_project(&sources, &syntax, |claim, _, _, _, _| {
                let nominal = claim
                    .graph
                    .type_keys
                    .iter()
                    .filter(|key| matches!(key.first(), Some(0x10 | 0x12)))
                    .collect::<Vec<_>>();
                assert_eq!(nominal.len(), if case == "plain-struct" { 1 } else { 2 });
                assert_eq!(
                    claim.graph.functions.len(),
                    1,
                    "unused template gains no invented instance"
                );
            });
        }
    }
}

#[test]
fn closed_children_under_opaque_and_borrow_parents_remain_original_roots() {
    for provider in PROVIDERS {
        for case in ["open-parent", "borrow-parent"] {
            let (sources, syntax) = project(case, provider);
            with_project(&sources, &syntax, |claim, _, _, _, _| {
                assert_eq!(
                    claim.graph.type_keys,
                    vec![vec![0], vec![1], vec![2], vec![0x14, 1, 0, 0, 0, 1, 0, 0, 0, 1]]
                );
                assert_eq!(claim.graph.functions.len(), 1);
            });
        }
    }
}

#[test]
fn independent_extra_missing_and_foreign_universes_never_gain_source_root_authority() {
    for provider in PROVIDERS {
        let (sources, syntax) = project("closed-template", provider);
        for attack in 0..4 {
            let graph = claims::graph(&syntax, &sources, attack);
            if attack == 2 {
                // Existing layout authority already requires every nongeneric original.
                // Keep the hostile universe unchanged and require its owning rejection.
                claims::missing_original_rejects_at_layout(&graph, &sources);
                continue;
            }
            let (linear, linux) = claims::layouts(&graph, &sources);
            let claim = owned::produce_claim_v4(&syntax, &sources, &linear, &linux, &[])
                .expect("source producer is not a complete stored-demand authority");
            let result = seal(&claim, &syntax, &sources, &linear, &linux);
            if attack == 0 {
                result.expect("literal independent complete source-root inventory");
            } else {
                assert!(
                    result.is_err(),
                    "extra specialization / omitted original / equal-width foreign attack {attack}"
                );
            }
        }
        let graph = claims::graph(&syntax, &sources, 0);
        let (linear, linux) = claims::layouts(&graph, &sources);
        let claim = owned::produce_claim_v4(&syntax, &sources, &linear, &linux, &[])
            .expect("pristine producer");
        seal(&claim, &syntax, &sources, &linear, &linux)
            .expect("pristine after all hostile universes");
    }
}
