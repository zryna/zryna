use super::*;

#[test]
fn allocation_core_native_private_utf8_storage_and_failure_atomicity() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let runtime = std::str::from_utf8(crate::ownership_runtime_v1::SOURCE)
        .expect("native runtime source")
        .replace(
            "/* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */",
            "case 7U: *stride = 4U; *alignment = 4U; return 1;",
        );
    let harness =
        fs::read_to_string(corpus().join("native-storage.c")).expect("native observations");
    let source = format!(
        "#include <stdint.h>\nstatic uint64_t fail_at;\n#define ZRYNA_RT_O1_FAIL_ALLOCATION_AT fail_at\n{runtime}\n{harness}"
    );
    let path = workspace.root().join("allocation-observation.c");
    fs::write(&path, source).expect("private runtime translation unit");
    compile_and_run(&path, workspace.root(), b"native allocation observation passed\n");
}

fn compile_and_run(source: &std::path::Path, root: &std::path::Path, expected: &[u8]) {
    crate::native::ownership::allocation_fixture_process::compile_and_run(source, root, expected)
        .expect("bounded private native allocation observation");
}
