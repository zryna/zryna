//! Unchanged portable observer scripts/formulas and batching; actual status is retained first.
use super::{process, source::digest};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
fn node() -> PathBuf {
    ["ZRYNA_TEST_NODE", "NODE"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .chain(
            std::env::var_os("PATH")
                .into_iter()
                .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
                .map(|path| path.join("node.exe")),
        )
        .find(|path| path.is_file())
        .expect("existing Windows Node host")
        .canonicalize()
        .expect("exact direct Node path")
}
fn run(
    case: &str,
    provider: &str,
    directory: &Path,
    script: &str,
    sha: &str,
    depth: Option<u32>,
    label: &str,
) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_recursive_owner")
        .join(script);
    assert_eq!(digest(&std::fs::read(&script).expect("unchanged observer script")), sha);
    let node = node();
    let mut args =
        vec![script.into_os_string(), directory.as_os_str().to_owned(), OsString::from(case)];
    if let Some(depth) = depth {
        args.push(OsString::from(depth.to_string()));
    }
    let observed = process::run(&node, &args, directory);
    std::fs::write(directory.join(format!("{label}.stdout.json")), &observed.stdout)
        .expect("retain stdout before assertion");
    std::fs::write(directory.join(format!("{label}.stderr.txt")), &observed.stderr)
        .expect("retain stderr before assertion");
    let status = serde_json::json!({"case":case,"provider":provider,"label":label,"depth":depth,"program":node,"argv":args.iter().map(|arg|arg.to_string_lossy()).collect::<Vec<_>>(),
        "exit_code":observed.status.and_then(|status|status.code()),"host_success":observed.status.is_some_and(|status|status.success()),"host_status":observed.status.map(|status|status.to_string()),
        "containment_failure":observed.failure.as_deref(),"stdout_bytes":observed.stdout.len(),"stdout_sha256":digest(&observed.stdout),"stderr_bytes":observed.stderr.len(),"stderr_sha256":digest(&observed.stderr),
        "execution_seconds":20,"stream_bytes":65536,"existing_cleanup_seconds":5,"native_attempted":false});
    std::fs::write(
        directory.join(format!("{label}.status.json")),
        serde_json::to_vec_pretty(&status).expect("actual status JSON"),
    )
    .expect("retain status before assertion");
    assert!(observed.failure.is_none(), "{status}");
    assert!(observed.status.is_some_and(|status| status.success()), "{status}");
    assert!(observed.stderr.is_empty(), "{status}");
}
pub(super) fn javascript(case: &str, provider: &str, directory: &Path) {
    let depths: &[Option<u32>] =
        if case == "clone" { &[Some(0), Some(1), Some(8192), Some(65536)] } else { &[None] };
    for depth in depths {
        let label = depth.map_or_else(|| "release".to_owned(), |depth| format!("release-{depth}"));
        run(
            case,
            provider,
            directory,
            "release.mjs",
            "0eb211cb30f6d082c1dfc4d0a69f6ec4fe709500f4dbd20315786ff1185d722a",
            *depth,
            &label,
        );
    }
}
pub(super) fn webassembly(case: &str, provider: &str, directory: &Path) {
    for depth in [0, 1, 8192, 65536] {
        run(
            case,
            provider,
            directory,
            "wasm.mjs",
            "2d1601dc2cb304f099882e6208c8a9307febb8ebe2c9f9841433f59491a593ab",
            Some(depth),
            &format!("wasm-{depth}"),
        );
    }
}
