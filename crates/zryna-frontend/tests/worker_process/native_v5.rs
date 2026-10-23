//! Actual library worker in a fresh process; no public CLI v5 registration.
use std::{ffi::OsString, path::PathBuf, time::Duration};

use serde_json::Value;
use zryna_frontend::{
    ProviderExpectationV5, WorkerFailure, WorkerFrontendV5, WorkerLimitsV5, WorkerSpecV5,
    native_worker::{NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION},
};
use zryna_source::{SourceFileInput, SourceMap};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root")
        .to_path_buf()
}

fn worker(version: &str, limits: WorkerLimitsV5) -> WorkerFrontendV5 {
    WorkerFrontendV5::new(
        WorkerSpecV5::new(
            std::env::current_exe().expect("self-hosted native worker"),
            vec![OsString::from("--native-worker-v5")],
            root(),
            ProviderExpectationV5::new(NATIVE_PROVIDER, version).expect("native identity"),
            limits,
        )
        .expect("bounded direct native process"),
    )
}

fn sources(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.into() }])
        .expect("authoritative sources")
}

pub(super) fn run_contract_tests() {
    let corpus: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/provider-conformance-v5/corpus.json"))
            .expect("independent corpus"),
    )
    .expect("corpus JSON");
    for entry in corpus["cases"].as_array().expect("cases") {
        let name = entry["id"].as_str().expect("case identity");
        let reference: Value = serde_json::from_slice(
            &std::fs::read(root().join(entry["reference"].as_str().expect("reference path")))
                .expect("frozen independent DTO"),
        )
        .expect("reference JSON");
        let mut files: Vec<_> = entry["files"]
            .as_array()
            .expect("files")
            .iter()
            .map(|unit| {
                let text = unit["fragments"]
                    .as_array()
                    .expect("fragments")
                    .iter()
                    .map(|fragment| {
                        fragment["text"].as_str().map_or_else(
                            || {
                                std::fs::read_to_string(
                                    root().join(fragment["source"].as_str().expect("source path")),
                                )
                                .expect("frozen source")
                            },
                            str::to_owned,
                        )
                    })
                    .collect();
                SourceFileInput { path: unit["path"].as_str().expect("path").into(), text }
            })
            .collect();
        files.reverse();
        let map = SourceMap::build(files).expect("reversed complete source set");
        let seal = worker(NATIVE_PROVIDER_VERSION, WorkerLimitsV5::default())
            .analyze_verified_v5(&map)
            .unwrap_or_else(|error| panic!("native v5 {name}: {error:?}"));
        assert!(seal.is_bound_to(&map), "{name}");
        assert_eq!(
            serde_json::to_value(seal.files()).expect("sealed syntax bytes"),
            reference["files"],
            "native process must preserve independent DTO for {name}"
        );
    }

    let good = sources("export function answer(): i32 { return 42; }");
    let error = worker("untrusted-version", WorkerLimitsV5::default())
        .analyze_verified_v5(&good)
        .expect_err("native identity must authenticate before analysis");
    assert_eq!(error.failure(), WorkerFailure::ProviderVersion);
    let limits = WorkerLimitsV5::new(Duration::from_secs(2), 32, 1_024)
        .expect("caller-tightened stdout budget");
    let error = worker(NATIVE_PROVIDER_VERSION, limits)
        .analyze_verified_v5(&good)
        .expect_err("actual native handshake exceeds tightened stdout cap");
    assert_eq!(error.failure(), WorkerFailure::OutputLimit);

    for text in [
        "export function answer(): i32 { return `forbidden`; }",
        "export const rejected = 1; export function answer(): i32 { return 42; }",
        "export function answer(): i32 { return Option<i32>.some(7); }",
    ] {
        let error = worker(NATIVE_PROVIDER_VERSION, WorkerLimitsV5::default())
            .analyze_verified_v5(&sources(text))
            .expect_err("native recovery never creates a partial v5 seal");
        assert_eq!(error.failure(), WorkerFailure::ProviderRejected);
        worker(NATIVE_PROVIDER_VERSION, WorkerLimitsV5::default())
            .analyze_verified_v5(&good)
            .expect("fresh real worker after rejected source");
    }

    let missing = sources(
        "import { identity } from \"./missing-native-v5.zry\"; export function answer(): i32 { return identity<i32>(42); }",
    );
    worker(NATIVE_PROVIDER_VERSION, WorkerLimitsV5::default())
        .analyze_verified_v5(&missing)
        .expect("syntax worker does not discover or read an import on the host filesystem");
}
