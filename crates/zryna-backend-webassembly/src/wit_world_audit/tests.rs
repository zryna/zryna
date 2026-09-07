use super::*;
use std::fs;
use std::path::PathBuf;

const ROOT: &str = r"
    package zryna:capability-profiles@0.1.0;
    world browser {}
    world command {
      import wasi:clocks/monotonic-clock@0.2.12;
      import wasi:clocks/wall-clock@0.2.12;
      import wasi:cli/environment@0.2.12;
      import wasi:filesystem/preopens@0.2.12;
      import wasi:filesystem/types@0.2.12;
      import wasi:sockets/instance-network@0.2.12;
      import wasi:sockets/ip-name-lookup@0.2.12;
      import wasi:sockets/network@0.2.12;
      import wasi:sockets/tcp-create-socket@0.2.12;
      import wasi:sockets/tcp@0.2.12;
      import wasi:sockets/udp-create-socket@0.2.12;
      import wasi:sockets/udp@0.2.12;
      import wasi:random/random@0.2.12;
      export wasi:cli/run@0.2.12;
    }
    world server {
      import wasi:clocks/monotonic-clock@0.2.12;
      import wasi:clocks/wall-clock@0.2.12;
      import wasi:http/outgoing-handler@0.2.12;
      import wasi:random/random@0.2.12;
      export wasi:http/incoming-handler@0.2.12;
    }
";

const DEPENDENCY_PACKAGES: &[&str] =
    &["cli", "clocks", "filesystem", "http", "io", "random", "sockets"];

fn independently_resolve(root: &str) -> Result<(Resolve, wit_parser::PackageId), Diagnostic> {
    let mut owned = vec![(pins::ROOT_PACKAGE.to_owned(), "root.wit".to_owned(), root.to_owned())];
    let dependencies =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/wit-world-audit-v1/dependencies");
    for package_name in DEPENDENCY_PACKAGES {
        let package = dependencies.join(package_name);
        let package_identity = format!("wasi:{package_name}@0.2.12");
        let mut files = fs::read_dir(&package)
            .expect("WASI package sources")
            .map(|entry| entry.expect("WASI source entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "wit"))
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            let file_name = file.file_name().expect("WIT file name").to_string_lossy();
            owned.push((
                package_identity.clone(),
                format!("wasi/{package_name}/{file_name}"),
                fs::read_to_string(file).expect("UTF-8 WASI WIT source"),
            ));
        }
    }
    assert_eq!(owned.len(), pins::SOURCES.len(), "complete pinned WIT source inventory");
    resolve_source_texts(
        owned.iter().map(|(package, path, text)| (package.as_str(), path.as_str(), text.as_str())),
    )
}

fn authentication_failure(sources: &[WitSource], context: &str) -> Diagnostic {
    match authenticate(sources) {
        Ok(_) => panic!("{context}"),
        Err(error) => error,
    }
}

#[test]
fn independent_baseline_resolves_and_matches_the_exact_world_closure() {
    let (resolve, root) = independently_resolve(ROOT).expect("accepted graph resolves");
    audit_resolved(&resolve, root).expect("accepted resolved graph matches");
}

#[test]
fn independent_resolved_interface_comparison_rejects_browser_broadening() {
    let broadened =
        ROOT.replace("world browser {}", "world browser { export wasi:cli/run@0.2.12; }");
    let (resolve, root) = independently_resolve(&broadened).expect("broadened graph resolves");
    assert_eq!(audit_resolved(&resolve, root).expect_err("broadening").code(), "ZRYNA-W4002");
}

#[test]
fn independent_resolution_rejects_wrong_version_world_and_malformed_source() {
    let wrong_version = ROOT.replacen("@0.2.12", "@0.2.13", 1);
    assert_eq!(
        independently_resolve(&wrong_version).expect_err("wrong version").code(),
        "ZRYNA-W4000"
    );

    let renamed = ROOT.replace("world command", "world renamed-command");
    let (resolve, root) = independently_resolve(&renamed).expect("renamed graph resolves");
    assert_eq!(audit_resolved(&resolve, root).expect_err("wrong world").code(), "ZRYNA-W4002");

    assert_eq!(
        independently_resolve("package zryna:capability-profiles@0.1.0; world browser {")
            .expect_err("malformed root")
            .code(),
        "ZRYNA-W4000"
    );
}

#[test]
fn parser_input_byte_limits_accept_the_boundary_and_reject_the_first_extra() {
    let exact_root = vec![b'x'; MAX_ROOT_SOURCE_BYTES];
    let exact = [WitSource::new(pins::SOURCES[0].path, exact_root)];
    assert_ne!(authentication_failure(&exact, "unauthenticated exact root").code(), "ZRYNA-W4003");

    let first_extra_root = vec![b'x'; MAX_ROOT_SOURCE_BYTES + 1];
    let first_extra = [WitSource::new(pins::SOURCES[0].path, first_extra_root)];
    assert_eq!(authentication_failure(&first_extra, "first extra root byte").code(), "ZRYNA-W4003");

    let exact_total = (0..8)
        .map(|index| WitSource::new(format!("unknown-{index}.wit"), vec![b'x'; MAX_SOURCE_BYTES]))
        .collect::<Vec<_>>();
    assert_ne!(
        authentication_failure(&exact_total, "unauthenticated exact total").code(),
        "ZRYNA-W4003"
    );
    let mut first_extra_total = exact_total;
    first_extra_total[0] = WitSource::new("unknown-0.wit", vec![b'x'; MAX_SOURCE_BYTES + 1]);
    assert_eq!(
        authentication_failure(&first_extra_total, "first extra total byte").code(),
        "ZRYNA-W4003"
    );

    let exact_path = [WitSource::new("x".repeat(MAX_SOURCE_PATH_BYTES), Vec::new())];
    assert_ne!(
        authentication_failure(&exact_path, "unauthenticated exact path").code(),
        "ZRYNA-W4003"
    );
    let first_extra_path = [WitSource::new("x".repeat(MAX_SOURCE_PATH_BYTES + 1), Vec::new())];
    assert_eq!(
        authentication_failure(&first_extra_path, "first extra path byte").code(),
        "ZRYNA-W4003"
    );
}
