use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use zryna_frontend::{
    FrontendCapabilities, ProviderExpectation, ProviderExpectationV3, ProviderExpectationV4,
    WorkerFrontend, WorkerFrontendV3, WorkerFrontendV4, WorkerLimits, WorkerLimitsV3,
    WorkerLimitsV4, WorkerSpec, WorkerSpecV3, WorkerSpecV4,
    native_worker::{NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION},
};
use zryna_source::{SourceFileInput, SourceMap};

#[derive(Deserialize)]
pub struct Registry {
    pub sources: Vec<SourceCase>,
    #[serde(rename = "cliBuildCases")]
    pub cli_build_cases: Vec<BuildCase>,
}
#[derive(Deserialize)]
pub struct SourceCase {
    pub id: String,
    pub protocol: u32,
    pub source: Pin,
    #[serde(rename = "syntaxVerification")]
    pub syntax_verification: String,
}
#[derive(Deserialize)]
pub struct Pin {
    pub path: String,
    pub bytes: usize,
    pub sha256: String,
}
#[derive(Deserialize)]
pub struct BuildCase {
    pub id: String,
    pub profile: String,
    pub entrypoint: String,
    pub disposition: String,
    #[serde(rename = "diagnosticCodes")]
    pub diagnostic_codes: Vec<String>,
}

pub fn registry() -> (PathBuf, Registry) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = crate_root
        .parent()
        .expect("driver crate belongs to crates directory")
        .parent()
        .expect("crates directory belongs to repository")
        .to_path_buf();
    let bytes =
        fs::read(root.join("tests/native-provider-public/registry.json")).expect("frozen registry");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "acb66251801fa9a46575b15803b4576d5171e619b2e71d598ce4c3dec136a1fc",
        "exact closed public registry"
    );
    let registry: Registry = serde_json::from_slice(&bytes).expect("registry schema");
    assert_eq!(registry.sources.len(), 114);
    assert_eq!(registry.cli_build_cases.len(), 88);
    for source in &registry.sources {
        let _ = source_map(&root, &source.source);
    }
    (root, registry)
}

pub fn source_map(root: &Path, pin: &Pin) -> SourceMap {
    let path = root.join(&pin.path);
    assert!(fs::symlink_metadata(&path).expect("fixture metadata").is_file());
    let bytes = fs::read(path).expect("frozen source bytes");
    assert_eq!(bytes.len(), pin.bytes, "{} frozen size", pin.path);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), pin.sha256, "{} frozen hash", pin.path);
    SourceMap::build(vec![SourceFileInput {
        path: pin.path.clone(),
        text: String::from_utf8(bytes).expect("source UTF-8"),
    }])
    .expect("authoritative source map")
}

pub struct Workers {
    pub native2: WorkerFrontend,
    pub bootstrap2: WorkerFrontend,
    pub native3: WorkerFrontendV3,
    pub bootstrap3: WorkerFrontendV3,
    pub native4: WorkerFrontendV4,
    pub bootstrap4: WorkerFrontendV4,
}
impl Workers {
    pub fn new(root: &Path) -> Self {
        let native = PathBuf::from(
            env::var_os("ZRYNA_NATIVE_CLI")
                .expect("qualification requires ZRYNA_NATIVE_CLI pointing to the actual built CLI"),
        );
        let node = PathBuf::from(
            env::var_os("ZRYNA_NODE").expect("qualification requires pinned ZRYNA_NODE"),
        );
        assert!(
            native.is_absolute() && node.is_absolute(),
            "explicit absolute qualification executables"
        );
        let adapter = root.join("adapters/typescript-6");
        let native_args = |protocol: &str| {
            vec![
                OsString::from("__native-frontend"),
                OsString::from("--protocol"),
                OsString::from(protocol),
            ]
        };
        let bootstrap_args = |protocol| {
            vec![OsString::from(if protocol == 2 {
                "src/worker.mjs".to_owned()
            } else {
                format!("src/worker-v{protocol}.mjs")
            })]
        };
        let v2 = |image: &Path, args, directory: &Path, provider, version| {
            WorkerFrontend::new(
                WorkerSpec::new(
                    image,
                    args,
                    directory,
                    ProviderExpectation::new(
                        provider,
                        version,
                        2,
                        FrontendCapabilities {
                            module_resolution: false,
                            semantic_diagnostics: false,
                        },
                    )
                    .expect("exact pinned protocol-v2 expectation"),
                    WorkerLimits::default(),
                )
                .expect("bounded absolute protocol-v2 worker configuration"),
            )
        };
        let v3 = |image: &Path, args, directory: &Path, provider, version| {
            WorkerFrontendV3::new(
                WorkerSpecV3::new(
                    image,
                    args,
                    directory,
                    ProviderExpectationV3::new(provider, version)
                        .expect("exact pinned protocol-v3 expectation"),
                    WorkerLimitsV3::default(),
                )
                .expect("bounded absolute protocol-v3 worker configuration"),
            )
        };
        let v4 = |image: &Path, args, directory: &Path, provider, version| {
            WorkerFrontendV4::new(
                WorkerSpecV4::new(
                    image,
                    args,
                    directory,
                    ProviderExpectationV4::new(provider, version)
                        .expect("exact pinned protocol-v4 expectation"),
                    WorkerLimitsV4::default(),
                )
                .expect("bounded absolute protocol-v4 worker configuration"),
            )
        };
        Self {
            native2: v2(&native, native_args("2"), root, NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION),
            bootstrap2: v2(&node, bootstrap_args(2), &adapter, "typescript-6", "6.0.3"),
            native3: v3(&native, native_args("3"), root, NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION),
            bootstrap3: v3(&node, bootstrap_args(3), &adapter, "typescript-6", "6.0.3"),
            native4: v4(&native, native_args("4"), root, NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION),
            bootstrap4: v4(&node, bootstrap_args(4), &adapter, "typescript-6", "6.0.3"),
        }
    }
}
