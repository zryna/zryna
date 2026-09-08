use std::{env, ffi::OsString, fs, path::PathBuf};

use zryna_backend_webassembly::WitSource;
use zryna_frontend::{
    FrontendCapabilities, ProviderExpectation, WorkerFrontend, WorkerLimits, WorkerSpec, syntax_v2,
};
use zryna_source::{SourceFileInput, SourceMap};

pub(super) fn frontend() -> WorkerFrontend {
    let node = ["ZRYNA_TEST_NODE", "NODE"].into_iter().find_map(env::var_os).map_or_else(
        || {
            let executable = if cfg!(windows) { "node.exe" } else { "node" };
            env::split_paths(&env::var_os("PATH").expect("test PATH"))
                .map(|path| path.join(executable))
                .find(|path| path.is_file())
                .expect("pinned test Node")
        },
        PathBuf::from,
    );
    assert!(node.is_absolute() && node.is_file());
    let expected = ProviderExpectation::new(
        "typescript-6",
        "6.0.3",
        syntax_v2::PROTOCOL_VERSION,
        FrontendCapabilities { module_resolution: false, semantic_diagnostics: false },
    )
    .expect("provider identity");
    let spec = WorkerSpec::new(
        node,
        vec![OsString::from("src/worker.mjs")],
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../adapters/typescript-6"),
        expected,
        WorkerLimits::default(),
    )
    .expect("authenticated frontend worker specification");
    WorkerFrontend::new(spec)
}

pub(super) fn sources() -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "examples/universal/add.zry".into(),
        text: include_str!("../../../../../examples/universal/add.zry").into(),
    }])
    .expect("exact existing add source")
}

pub(super) fn wit() -> Vec<WitSource> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let logical_root = "spec/wit/capability-profiles-v1/worlds.wit";
    let mut result = vec![WitSource::new(
        logical_root,
        fs::read(root.join("../..").join(logical_root)).expect("WIT root"),
    )];
    for package in ["cli", "clocks", "filesystem", "http", "io", "random", "sockets"] {
        let directory = root
            .join("../zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies")
            .join(package);
        let mut files = fs::read_dir(directory)
            .expect("pinned WIT package")
            .map(|entry| entry.expect("WIT entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "wit"))
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            let name = file.file_name().expect("WIT name").to_string_lossy();
            result.push(WitSource::new(
                format!("wasi/{package}/{name}"),
                fs::read(&file).expect("WIT bytes"),
            ));
        }
    }
    assert_eq!(result.len(), 34);
    result
}
