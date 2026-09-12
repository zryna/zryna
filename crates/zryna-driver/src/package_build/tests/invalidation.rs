use super::*;

#[test]
fn changed_source_bytes_lock_and_dependency_select_new_plan_identities() {
    let (_first_sources, first) = fixture();
    let second_sources = TemporaryRoot::new("changed-sources");
    write_package(
        second_sources.path(),
        "packages/app",
        "app",
        b"export function main(): i32 { return 2; }\n",
        &[json!({
            "alias": "math",
            "name": "library",
            "source": source("packages/library"),
            "version": "1.0.0"
        })],
    );
    write_package(
        second_sources.path(),
        "packages/library",
        "library",
        b"export function value(): i32 { return 1; }\n",
        &[],
    );
    resolved(&second_sources, PackageLockMode::Update);
    let second = resolved(&second_sources, PackageLockMode::Frozen);
    let (_cache_project, cache, _output_project, output) = roots("source-identity");
    let first_key = plan_key(&first, configuration(one_output_for_tests()), &cache, &output)
        .expect("test fixture");
    let second_key = plan_key(&second, configuration(one_output_for_tests()), &cache, &output)
        .expect("test fixture");
    assert_ne!(first.graph().lock_sha256(), second.graph().lock_sha256());
    assert_ne!(first_key, second_key);

    let dependency_sources = TemporaryRoot::new("changed-dependency");
    write_package(
        dependency_sources.path(),
        "packages/app",
        "app",
        b"export function main(): i32 { return 1; }\n",
        &[],
    );
    resolved(&dependency_sources, PackageLockMode::Update);
    let dependency_changed = resolved(&dependency_sources, PackageLockMode::Frozen);
    let dependency_key =
        plan_key(&dependency_changed, configuration(one_output_for_tests()), &cache, &output)
            .expect("test fixture");
    assert_ne!(first.graph().lock_sha256(), dependency_changed.graph().lock_sha256());
    assert_ne!(first_key, dependency_key);
}

#[test]
fn mismatched_tool_and_reproduction_environment_reject_before_compilation() {
    let (_sources, resolution) = fixture();
    let (_cache_project, cache, _output_project, output) = roots("invalid-plan");
    let mut tool = configuration(one_output_for_tests());
    tool.host_tools[0].sha256 = ZERO.to_owned();
    assert_eq!(
        plan_key(&resolution, tool, &cache, &output).expect_err("tool mismatch").code(),
        "ZRYNA-B4101"
    );

    let mut environment = configuration(one_output_for_tests());
    environment.host.environment[0].value = "en_US.UTF-8".to_owned();
    assert_eq!(
        plan_key(&resolution, environment, &cache, &output).expect_err("ambient locale").code(),
        "ZRYNA-B4101"
    );
}
