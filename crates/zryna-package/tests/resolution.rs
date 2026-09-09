//! Package resolution, identity, canonicalization, and bound evidence.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use zryna_package::{
    GraphRole, LockMode, PackageFile, PackageMaterial, PackageSource, PackageSourceKind,
    PackageSourceProvider, ResolveError, resolve,
};

struct MemoryProvider {
    packages: BTreeMap<PackageSource, PackageMaterial>,
}

impl PackageSourceProvider for MemoryProvider {
    fn load(&mut self, source: &PackageSource) -> Result<PackageMaterial, ResolveError> {
        self.packages
            .get(source)
            .cloned()
            .ok_or_else(|| ResolveError::source("fixture source is absent"))
    }
}

fn local(locator: &str) -> PackageSource {
    PackageSource {
        kind: PackageSourceKind::Local,
        locator: locator.to_owned(),
        revision: String::new(),
    }
}

fn git(locator: &str, revision: &str) -> PackageSource {
    PackageSource {
        kind: PackageSourceKind::Git,
        locator: locator.to_owned(),
        revision: revision.to_owned(),
    }
}

fn canonical(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("canonical test JSON");
    bytes.push(b'\n');
    bytes
}

fn checksum(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn package(
    source: PackageSource,
    name: &str,
    content: &[u8],
    dependencies: Vec<(&str, &str, PackageSource)>,
) -> PackageMaterial {
    let source = serde_json::to_value(source).expect("source tuple");
    let dependencies = dependencies
        .into_iter()
        .map(|(alias, name, source)| {
            json!({ "alias": alias, "name": name, "source": source, "version": "1.0.0" })
        })
        .collect::<Vec<_>>();
    let manifest = json!({
        "compatibility": {
            "compiler": "0.1.0",
            "profile": "control-flow-v1",
            "targets": ["javascript", "webassembly"]
        },
        "dependencies": dependencies,
        "files": [{ "path": "src/main.zry", "sha256": checksum(content), "size": content.len() }],
        "format": "zryna.package.v1",
        "name": name,
        "source": source,
        "version": "1.0.0"
    });
    PackageMaterial {
        manifest: canonical(&manifest),
        files: vec![PackageFile { path: "src/main.zry".to_owned(), bytes: content.to_vec() }],
    }
}

fn two_packages() -> (PackageSource, MemoryProvider) {
    let app = local("packages/app");
    let library = local("packages/library");
    let packages = BTreeMap::from([
        (
            app.clone(),
            package(
                app.clone(),
                "app",
                b"export function main(): i32 { return 1; }\n",
                vec![("math", "library", library.clone())],
            ),
        ),
        (
            library.clone(),
            package(library, "library", b"export function value(): i32 { return 1; }\n", vec![]),
        ),
    ]);
    (app, MemoryProvider { packages })
}

#[test]
fn accepted_package_release_fixture_replays_exact_lock_authority() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/package-release-v1/valid.json"))
            .expect("accepted package fixture");
    let expected: Value =
        serde_json::from_str(include_str!("../../../tests/package-release-v1/expected.json"))
            .expect("accepted package result");
    let materials: BTreeMap<_, _> = fixture["materials"]
        .as_array()
        .expect("materials")
        .iter()
        .map(|material| {
            (
                material["sha256"].as_str().expect("material digest"),
                material["content"].as_str().expect("material content").as_bytes(),
            )
        })
        .collect();
    let mut packages = BTreeMap::new();
    for manifest in fixture["manifests"].as_array().expect("manifests") {
        let source: PackageSource =
            serde_json::from_value(manifest["source"].clone()).expect("source tuple");
        let files = manifest["files"]
            .as_array()
            .expect("files")
            .iter()
            .map(|file| PackageFile {
                path: file["path"].as_str().expect("file path").to_owned(),
                bytes: materials[file["sha256"].as_str().expect("file digest")].to_vec(),
            })
            .collect();
        packages.insert(source, PackageMaterial { manifest: canonical(manifest), files });
    }
    let root = local("packages/p1");
    let mut provider = MemoryProvider { packages };
    let graph = resolve(&mut provider, root, LockMode::Update).expect("accepted fixture graph");
    assert_eq!(graph.lock_bytes(), canonical(&fixture["lock"]));
    assert_eq!(graph.lock_sha256(), expected["lockSha256"].as_str().expect("lock digest"));
}

#[test]
fn update_and_frozen_replay_produce_one_exact_graph() {
    let (root, mut provider) = two_packages();
    let first = resolve(&mut provider, root.clone(), LockMode::Update).expect("update graph");
    assert_eq!(first.packages().len(), 2);
    assert_eq!(first.lock_sha256().len(), 64);

    let (_, mut replay) = two_packages();
    let second =
        resolve(&mut replay, root, LockMode::Frozen(first.lock_bytes())).expect("frozen graph");
    assert_eq!(first, second);

    let mut stale: Value = serde_json::from_slice(first.lock_bytes()).expect("lock JSON");
    stale["compatibility"]["profile"] = json!("i32-v1");
    let (_, mut replay) = two_packages();
    let error = resolve(&mut replay, local("packages/app"), LockMode::Frozen(&canonical(&stale)))
        .expect_err("stale lock must reject");
    assert_eq!(error.code(), "ZRYNA-P4010");

    let mut duplicate: Value = serde_json::from_slice(first.lock_bytes()).expect("lock JSON");
    let repeated = duplicate["packages"][0].clone();
    duplicate["packages"].as_array_mut().expect("packages").insert(1, repeated);
    let duplicate_bytes = canonical(&duplicate);
    let (_, mut replay) = two_packages();
    let error = resolve(&mut replay, local("packages/app"), LockMode::Frozen(&duplicate_bytes))
        .expect_err("duplicate lock instance must reject");
    assert_eq!(error.code(), "ZRYNA-P4003");
}

#[test]
fn package_semantic_domains_are_graph_and_instance_bound() {
    let (root, mut provider) = two_packages();
    let graph = resolve(&mut provider, root, LockMode::Update).expect("graph");
    let app = graph.packages().iter().find(|package| package.name() == "app").expect("app");
    let library =
        graph.packages().iter().find(|package| package.name() == "library").expect("library");
    let first = graph.package_semantic_domain(app.instance().manifest_id()).expect("app domain");
    let replay = graph.package_semantic_domain(app.instance().manifest_id()).expect("same domain");
    let distinct =
        graph.package_semantic_domain(library.instance().manifest_id()).expect("library domain");
    assert_eq!(first, replay);
    assert_ne!(first, distinct);
    assert_eq!(first.role(), GraphRole::TargetRuntime);
    assert_eq!(first.profile(), graph.compatibility().profile);
    assert_eq!(first.package(), app.instance());
    assert!(graph.package_semantic_domain(&"0".repeat(64)).is_err());
}

#[test]
fn exact_git_source_is_identity_bearing_without_network_resolution() {
    let revision = "0123456789abcdef0123456789abcdef01234567";
    let root = local("packages/app");
    let dependency = git("https://example.com/zryna/math.git", revision);
    let mut provider = MemoryProvider {
        packages: BTreeMap::from([
            (
                root.clone(),
                package(root.clone(), "app", b"app\n", vec![("math", "math", dependency.clone())]),
            ),
            (dependency.clone(), package(dependency, "math", b"math\n", vec![])),
        ]),
    };
    let graph = resolve(&mut provider, root, LockMode::Update).expect("exact Git graph");
    assert_eq!(graph.packages().len(), 2);
}

#[test]
fn branch_like_git_revision_and_first_extra_edges_fail_closed() {
    let mut empty = MemoryProvider { packages: BTreeMap::new() };
    let branch = git("https://example.com/zryna/math.git", "main");
    assert_eq!(
        resolve(&mut empty, branch, LockMode::Update).expect_err("branch revision").code(),
        "ZRYNA-P4004"
    );

    let root = local("packages/app");
    let mut material = package(root.clone(), "app", b"app\n", vec![]);
    let mut manifest: Value = serde_json::from_slice(&material.manifest).expect("manifest");
    manifest["dependencies"] = Value::Array(
        (0..9)
            .map(|index| {
                json!({
                    "alias": format!("dep-{index}"),
                    "name": format!("pkg-{index}"),
                    "source": local(&format!("packages/p{index}")),
                    "version": "1.0.0"
                })
            })
            .collect(),
    );
    material.manifest = canonical(&manifest);
    let mut provider = MemoryProvider { packages: BTreeMap::from([(root.clone(), material)]) };
    assert_eq!(
        resolve(&mut provider, root, LockMode::Update).expect_err("ninth dependency").code(),
        "ZRYNA-P4001"
    );
}

#[test]
fn substitution_digest_escape_cycle_and_noncanonical_wire_fail_closed() {
    let (root, mut provider) = two_packages();
    let app = provider.packages.get_mut(&root).expect("app fixture");
    app.files[0].bytes.push(b'x');
    assert_eq!(
        resolve(&mut provider, root.clone(), LockMode::Update).expect_err("digest mismatch").code(),
        "ZRYNA-P4004"
    );

    let (root, mut provider) = two_packages();
    let app = provider.packages.get_mut(&root).expect("app fixture");
    let mut manifest: Value = serde_json::from_slice(&app.manifest).expect("manifest");
    manifest["source"]["locator"] = json!("packages/substitute");
    app.manifest = canonical(&manifest);
    assert_eq!(
        resolve(&mut provider, root.clone(), LockMode::Update).expect_err("substitution").code(),
        "ZRYNA-P4004"
    );

    let mut provider = MemoryProvider { packages: BTreeMap::new() };
    let invalid = local("../escape");
    assert_eq!(
        resolve(&mut provider, invalid, LockMode::Update).expect_err("escape").code(),
        "ZRYNA-P4005"
    );

    let app = local("packages/app");
    let library = local("packages/library");
    let mut provider = MemoryProvider {
        packages: BTreeMap::from([
            (
                app.clone(),
                package(
                    app.clone(),
                    "app",
                    b"app\n",
                    vec![("library", "library", library.clone())],
                ),
            ),
            (
                library.clone(),
                package(library, "library", b"lib\n", vec![("app", "app", app.clone())]),
            ),
        ]),
    };
    assert_eq!(
        resolve(&mut provider, app, LockMode::Update).expect_err("cycle").code(),
        "ZRYNA-P4008"
    );

    let (root, mut provider) = two_packages();
    let app = provider.packages.get_mut(&root).expect("app fixture");
    app.manifest.pop();
    app.manifest.push(b' ');
    assert_eq!(
        resolve(&mut provider, root, LockMode::Update).expect_err("wire").code(),
        "ZRYNA-P4002"
    );
}

#[test]
fn exact_package_limit_accepts_and_first_extra_rejects() {
    fn chain(count: usize) -> (PackageSource, MemoryProvider) {
        let mut packages = BTreeMap::new();
        for index in 0..count {
            let source = local(&format!("packages/p{index}"));
            let dependencies = if index + 1 == count {
                vec![]
            } else {
                vec![("next", "pkg", local(&format!("packages/p{}", index + 1)))]
            };
            packages.insert(
                source.clone(),
                package(source, if index == 0 { "root" } else { "pkg" }, b"x", dependencies),
            );
        }
        (local("packages/p0"), MemoryProvider { packages })
    }
    let (root, mut exact) = chain(16);
    assert_eq!(
        resolve(&mut exact, root, LockMode::Update).expect("exact bound").packages().len(),
        16
    );
    let (root, mut extra) = chain(17);
    assert_eq!(
        resolve(&mut extra, root, LockMode::Update).expect_err("first extra").code(),
        "ZRYNA-P4001"
    );
}
