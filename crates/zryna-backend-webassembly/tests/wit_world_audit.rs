//! Authenticated WIT dependency-closure and world-audit integration tests.

use std::fs;
use std::path::PathBuf;

use zryna_backend_webassembly::{WitSource, audit_pinned_wit_worlds};

const ROOT_LOGICAL_PATH: &str = "spec/wit/capability-profiles-v1/worlds.wit";
const DEPENDENCY_PACKAGES: &[&str] =
    &["cli", "clocks", "filesystem", "http", "io", "random", "sockets"];

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn accepted_sources() -> Vec<WitSource> {
    let root_path = crate_root().join("../../spec/wit/capability-profiles-v1/worlds.wit");
    let mut sources = vec![WitSource::new(
        ROOT_LOGICAL_PATH,
        fs::read(root_path).expect("accepted local WIT source"),
    )];
    let dependencies = crate_root().join("tests/wit-world-audit-v1/dependencies");
    for package_name in DEPENDENCY_PACKAGES {
        let package = dependencies.join(package_name);
        let mut files = fs::read_dir(&package)
            .expect("WASI package sources")
            .map(|entry| entry.expect("WASI source entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "wit"))
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            let file_name = file.file_name().expect("WIT file name").to_string_lossy();
            sources.push(WitSource::new(
                format!("wasi/{package_name}/{file_name}"),
                fs::read(file).expect("WASI WIT source"),
            ));
        }
    }
    assert_eq!(sources.len(), 34, "complete pinned WIT source inventory");
    sources
}

fn replace_root(sources: &mut [WitSource], contents: Vec<u8>) {
    let root =
        sources.iter_mut().find(|source| source.path() == ROOT_LOGICAL_PATH).expect("root source");
    *root = WitSource::new(ROOT_LOGICAL_PATH, contents);
}

#[test]
fn resolves_the_authenticated_dependency_closure_and_exact_worlds() {
    let audit = audit_pinned_wit_worlds(&accepted_sources()).expect("accepted WIT closure");
    assert_eq!(
        audit.packages(),
        [
            "wasi:cli@0.2.12",
            "wasi:clocks@0.2.12",
            "wasi:filesystem@0.2.12",
            "wasi:http@0.2.12",
            "wasi:io@0.2.12",
            "wasi:random@0.2.12",
            "wasi:sockets@0.2.12",
            "zryna:capability-profiles@0.1.0",
        ]
    );
    let worlds = audit.worlds();
    assert_eq!(worlds.len(), 3);
    assert_eq!(worlds[0].identity(), "zryna:capability-profiles/browser@0.1.0");
    assert!(worlds[0].explicit_imports().is_empty());
    assert!(worlds[0].resolved_imports().is_empty());
    assert!(worlds[0].exports().is_empty());
    assert_eq!(worlds[1].identity(), "zryna:capability-profiles/command@0.1.0");
    assert_eq!(
        worlds[1].explicit_imports(),
        [
            "wasi:cli/environment@0.2.12",
            "wasi:clocks/monotonic-clock@0.2.12",
            "wasi:clocks/wall-clock@0.2.12",
            "wasi:filesystem/preopens@0.2.12",
            "wasi:filesystem/types@0.2.12",
            "wasi:random/random@0.2.12",
            "wasi:sockets/instance-network@0.2.12",
            "wasi:sockets/ip-name-lookup@0.2.12",
            "wasi:sockets/network@0.2.12",
            "wasi:sockets/tcp-create-socket@0.2.12",
            "wasi:sockets/tcp@0.2.12",
            "wasi:sockets/udp-create-socket@0.2.12",
            "wasi:sockets/udp@0.2.12",
        ]
    );
    assert_eq!(
        worlds[1].resolved_imports(),
        [
            "wasi:cli/environment@0.2.12",
            "wasi:clocks/monotonic-clock@0.2.12",
            "wasi:clocks/wall-clock@0.2.12",
            "wasi:filesystem/preopens@0.2.12",
            "wasi:filesystem/types@0.2.12",
            "wasi:io/error@0.2.12",
            "wasi:io/poll@0.2.12",
            "wasi:io/streams@0.2.12",
            "wasi:random/random@0.2.12",
            "wasi:sockets/instance-network@0.2.12",
            "wasi:sockets/ip-name-lookup@0.2.12",
            "wasi:sockets/network@0.2.12",
            "wasi:sockets/tcp-create-socket@0.2.12",
            "wasi:sockets/tcp@0.2.12",
            "wasi:sockets/udp-create-socket@0.2.12",
            "wasi:sockets/udp@0.2.12",
        ]
    );
    assert_eq!(worlds[1].exports(), ["wasi:cli/run@0.2.12"]);
    assert_eq!(worlds[2].identity(), "zryna:capability-profiles/server@0.1.0");
    assert_eq!(
        worlds[2].explicit_imports(),
        [
            "wasi:clocks/monotonic-clock@0.2.12",
            "wasi:clocks/wall-clock@0.2.12",
            "wasi:http/outgoing-handler@0.2.12",
            "wasi:random/random@0.2.12",
        ]
    );
    assert_eq!(
        worlds[2].resolved_imports(),
        [
            "wasi:clocks/monotonic-clock@0.2.12",
            "wasi:clocks/wall-clock@0.2.12",
            "wasi:http/outgoing-handler@0.2.12",
            "wasi:http/types@0.2.12",
            "wasi:io/error@0.2.12",
            "wasi:io/poll@0.2.12",
            "wasi:io/streams@0.2.12",
            "wasi:random/random@0.2.12",
        ]
    );
    assert_eq!(worlds[2].exports(), ["wasi:http/incoming-handler@0.2.12"]);
}

#[test]
fn input_order_cannot_change_the_audit() {
    let sources = accepted_sources();
    let expected = audit_pinned_wit_worlds(&sources).expect("ordered audit");
    let mut reversed = sources.clone();
    reversed.reverse();
    assert_eq!(audit_pinned_wit_worlds(&reversed).expect("reversed audit"), expected);
    reversed.rotate_left(11);
    assert_eq!(audit_pinned_wit_worlds(&reversed).expect("rotated audit"), expected);
}

#[test]
fn missing_substituted_and_unknown_dependency_sources_fail_closed() {
    let mut missing = accepted_sources();
    missing.retain(|source| source.path() != "wasi/io/poll.wit");
    assert_eq!(
        audit_pinned_wit_worlds(&missing).expect_err("missing dependency").code(),
        "ZRYNA-W4001"
    );

    let mut substituted = accepted_sources();
    let dependency = substituted
        .iter_mut()
        .find(|source| source.path() == "wasi/cli/environment.wit")
        .expect("dependency source");
    let mut bytes = dependency.bytes().to_vec();
    bytes.push(b' ');
    *dependency = WitSource::new("wasi/cli/environment.wit", bytes);
    assert_eq!(
        audit_pinned_wit_worlds(&substituted).expect_err("substituted dependency").code(),
        "ZRYNA-W4001"
    );

    let mut unknown = accepted_sources();
    unknown[1] = WitSource::new("wasi/cli/unknown.wit", unknown[1].bytes().to_vec());
    assert_eq!(
        audit_pinned_wit_worlds(&unknown).expect_err("unknown dependency").code(),
        "ZRYNA-W4001"
    );
}

#[test]
fn hostile_version_world_broadening_and_malformed_substitutions_fail_closed() {
    let hostile = crate_root().join("tests/wit-world-audit-v1/hostile");
    for name in
        ["wrong-wasi-version.wit", "wrong-world.wit", "broadened-browser.wit", "malformed.wit"]
    {
        let mut sources = accepted_sources();
        replace_root(&mut sources, fs::read(hostile.join(name)).expect("hostile fixture"));
        let error = audit_pinned_wit_worlds(&sources).expect_err(name);
        assert_eq!(error.code(), "ZRYNA-W4001", "{name}");
    }
}

#[test]
fn exact_source_count_passes_and_the_first_extra_fails_before_parsing() {
    let mut sources = accepted_sources();
    assert_eq!(sources.len(), 34);
    audit_pinned_wit_worlds(&sources).expect("exact source-count limit");
    sources.push(WitSource::new("wasi/extra/extra.wit", Vec::new()));
    assert_eq!(
        audit_pinned_wit_worlds(&sources).expect_err("first extra source").code(),
        "ZRYNA-W4003"
    );
}

#[test]
fn rejection_does_not_leak_authority_into_the_next_call() {
    let mut bad = accepted_sources();
    bad.pop();
    assert_eq!(audit_pinned_wit_worlds(&bad).expect_err("rejected call").code(), "ZRYNA-W4001");

    let first = audit_pinned_wit_worlds(&accepted_sources()).expect("recovery audit");
    let second = audit_pinned_wit_worlds(&accepted_sources()).expect("repeat recovery audit");
    assert_eq!(first, second);
}
