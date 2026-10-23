//! Original Push/live/expired sources independently issue new seals before JS target admission.
use std::process::Command;
mod source;

#[test]
fn push_upgrade_original_packets_reach_exact_non_loan_views() {
    for case in source::SETTLED {
        let (_, native) = source::project(case, "native");
        let (_, typescript) = source::project(case, "typescript");
        assert_eq!(native.files(), typescript.files());
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _wire| {
                let pushes = program.operations().filter(|op| op.vec_push().is_some()).count();
                let upgrades = program
                    .functions()
                    .flat_map(zryna_ir::generic_v1::execution_v1::VerifiedFunction::blocks)
                    .filter(|block| block.weak_upgrade().is_some())
                    .count();
                assert_eq!(pushes, usize::from(case == "vec-push"));
                assert_eq!(upgrades, usize::from(case != "vec-push"));
                for op in program.operations() {
                    if let Some(push) = op.vec_push() {
                        assert!(push.target_place().is_some());
                        assert!(push.target_loan().is_none());
                    }
                }
                for block in program
                    .functions()
                    .flat_map(zryna_ir::generic_v1::execution_v1::VerifiedFunction::blocks)
                {
                    if let Some(upgrade) = block.weak_upgrade() {
                        assert!(upgrade.source_place().is_some());
                        assert!(upgrade.source_loan().is_none());
                        assert_eq!(
                            upgrade
                                .success()
                                .target()
                                .value_parameters()
                                .next()
                                .expect("genuine generated first parameter")
                                .id(),
                            upgrade.success_binding().id()
                        );
                    }
                }
            });
        }
    }
}

#[test]
fn push_upgrade_ordinary_originals_execute_without_ambient_runtime() {
    for case in source::SETTLED {
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _wire| {
                let artifact =
                    super::emit(program).expect("ordinary genuine Push/Upgrade emission");
                let result = match case {
                    "vec-push" => 18,
                    "weak-live" => 31,
                    _ => 37,
                };
                execute(case, provider, &artifact.source, "ordinary", &result.to_string());
            });
        }
    }
}

#[test]
fn push_upgrade_observed_faults_preserve_exact_owners_counts_and_retry() {
    for case in source::SETTLED {
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _wire| {
                let artifact =
                    super::emit_observed(program).expect("observed genuine Push/Upgrade emission");
                assert_eq!(
                    artifact,
                    super::emit_observed(program).expect("pristine deterministic emission")
                );
                execute(case, provider, artifact.as_source(), "observed", case);
            });
        }
    }
}

#[test]
fn push_upgrade_quota_separates_growth_attempts_from_count_aliases() {
    for (case, allocations, transitions) in
        [("vec-push", 2, 3), ("weak-live", 1, 7), ("weak-expired", 1, 7)]
    {
        source::with_program(case, "native", |program, _wire| {
            let context = super::Context::new(program).expect("same-seal target indices");
            let quota = super::runtime::ordinary::quota::prove(&context)
                .expect("finite ordinary Push/Upgrade resource proof");
            assert_eq!(
                quota.allocations, allocations,
                "physical attempts include growth, not aliases"
            );
            assert_eq!(quota.transitions, transitions, "logical calls plus funded final cleanup");
        });
    }
}

fn execute(case: &str, provider: &str, source: &str, mode: &str, argument: &str) {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let file = std::env::temp_dir()
        .join(format!("zryna-push-upgrade-{}-{case}-{provider}-{mode}.mjs", std::process::id()));
    std::fs::write(&file, source).expect("actual new-seal module");
    let script = if mode == "ordinary" {
        "src/generic_execution_v1/ordinary_tests/execute.mjs"
    } else {
        "src/generic_execution_v1/push_upgrade_tests/observe.mjs"
    };
    let output = Command::new("node").arg(directory.join(script)).arg(&file).arg(argument).output();
    std::fs::remove_file(&file).expect("remove private test module");
    let output = output.expect("Node genuine Push/Upgrade process");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
