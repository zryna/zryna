use super::*;

fn chain(package_count: usize) -> TemporaryRoot {
    let root = TemporaryRoot::new("graph-bound");
    for index in (0..package_count).rev() {
        let (locator, name) = if index == 0 {
            ("packages/app".to_owned(), "app".to_owned())
        } else {
            (format!("packages/library-{index:02}"), format!("library-{index:02}"))
        };
        let dependencies = if index + 1 == package_count {
            vec![]
        } else {
            let next_locator = format!("packages/library-{:02}", index + 1);
            vec![json!({
                "alias": "next",
                "name": format!("library-{:02}", index + 1),
                "source": source(&next_locator),
                "version": "1.0.0"
            })]
        };
        write_package(
            root.path(),
            &locator,
            &name,
            format!("export function value(): i32 {{ return {index}; }}\n").as_bytes(),
            &dependencies,
        );
    }
    root
}

#[test]
fn exact_graph_bound_executes_every_package_dependencies_first() {
    let sources = chain(16);
    resolved(&sources, PackageLockMode::Update);
    let resolution = resolved(&sources, PackageLockMode::Frozen);
    let (_cache_project, cache, _output_project, output) = roots("graph-exact");
    let mut compiler = RecordingCompiler::new(vec!["javascript/app.mjs".to_owned()]);
    execute_package_build(
        &PackageBuildRequest {
            resolution: &resolution,
            configuration: configuration(one_output_for_tests()),
            mode: PackageBuildMode::Frozen,
            cache_root: &cache,
            output_root: &output,
        },
        &mut compiler,
    )
    .expect("exact graph bound");
    assert_eq!(compiler.calls.len(), 16);
    assert_eq!(compiler.calls.last().map(String::as_str), Some(resolution.graph().root()));
}

#[test]
fn first_extra_graph_is_rejected_before_a_build_authority_exists() {
    let sources = chain(17);
    let error = resolve_package(&PackageResolutionRequest {
        source_root: sources.path().to_path_buf(),
        package: "packages/app".to_owned(),
        git_cache: None,
        mode: PackageLockMode::Update,
    })
    .expect_err("first extra graph package");
    assert_eq!(error.code(), "ZRYNA-P4001");
    assert!(!sources.path().join("packages/app/zryna.lock.json").exists());
}
