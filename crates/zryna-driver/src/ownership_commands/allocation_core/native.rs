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
    let executable = workspace.root().join("allocation-observation");
    fs::write(&path, source).expect("private runtime translation unit");
    let compiled = Command::new("/usr/bin/gcc")
        .args(["-std=c11", "-pedantic", "-Wall", "-Wextra", "-Werror", "-O2", "-fno-common"])
        .arg(path)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("supported Linux C compiler");
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    let output = Command::new(executable).output().expect("private runtime observation");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"native allocation observation passed\n");
}
