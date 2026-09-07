use super::*;

const ROOT: &str = r#"
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
"#;

const DEPENDENCIES: &[(&str, &str)] = &[
    ("wasi:cli@0.2.12", "package wasi:cli@0.2.12; interface environment {} interface run {}"),
    (
        "wasi:clocks@0.2.12",
        "package wasi:clocks@0.2.12; interface monotonic-clock {} interface wall-clock {}",
    ),
    (
        "wasi:filesystem@0.2.12",
        "package wasi:filesystem@0.2.12; interface preopens {} interface types {}",
    ),
    (
        "wasi:http@0.2.12",
        "package wasi:http@0.2.12; interface outgoing-handler {} interface incoming-handler {}",
    ),
    ("wasi:io@0.2.12", "package wasi:io@0.2.12; interface streams {}"),
    ("wasi:random@0.2.12", "package wasi:random@0.2.12; interface random {}"),
    (
        "wasi:sockets@0.2.12",
        "package wasi:sockets@0.2.12; interface instance-network {} interface ip-name-lookup {} interface network {} interface tcp-create-socket {} interface tcp {} interface udp-create-socket {} interface udp {}",
    ),
];

fn independently_resolve(root: &str) -> Result<(Resolve, wit_parser::PackageId), Diagnostic> {
    let mut owned = vec![(pins::ROOT_PACKAGE, "root.wit", root)];
    for (index, (package, text)) in DEPENDENCIES.iter().enumerate() {
        let path = match index {
            0 => "dep-0.wit",
            1 => "dep-1.wit",
            2 => "dep-2.wit",
            3 => "dep-3.wit",
            4 => "dep-4.wit",
            5 => "dep-5.wit",
            _ => "dep-6.wit",
        };
        owned.push((package, path, text));
    }
    let source_pins = owned
        .iter()
        .map(|(package, path, _)| pins::SourcePin { package, path, sha256: "" })
        .collect::<Vec<_>>();
    let sources = source_pins
        .iter()
        .zip(owned.iter())
        .map(|(pin, (_, _, text))| (pin, *text))
        .collect::<Vec<_>>();
    resolve_sources(&sources)
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
    let exact_root = [b'x'; MAX_ROOT_SOURCE_BYTES];
    let exact = [WitSource::new(pins::SOURCES[0].path, exact_root)];
    assert_ne!(authenticate(&exact).expect_err("unauthenticated exact root").code(), "ZRYNA-W4003");

    let first_extra_root = [b'x'; MAX_ROOT_SOURCE_BYTES + 1];
    let first_extra = [WitSource::new(pins::SOURCES[0].path, first_extra_root)];
    assert_eq!(
        authenticate(&first_extra).expect_err("first extra root byte").code(),
        "ZRYNA-W4003"
    );

    let exact_total = (0..8)
        .map(|index| WitSource::new(format!("unknown-{index}.wit"), vec![b'x'; MAX_SOURCE_BYTES]))
        .collect::<Vec<_>>();
    assert_ne!(
        authenticate(&exact_total).expect_err("unauthenticated exact total").code(),
        "ZRYNA-W4003"
    );
    let mut first_extra_total = exact_total;
    first_extra_total[0] = WitSource::new("unknown-0.wit", vec![b'x'; MAX_SOURCE_BYTES + 1]);
    assert_eq!(
        authenticate(&first_extra_total).expect_err("first extra total byte").code(),
        "ZRYNA-W4003"
    );

    let exact_path = [WitSource::new("x".repeat(MAX_SOURCE_PATH_BYTES), Vec::new())];
    assert_ne!(
        authenticate(&exact_path).expect_err("unauthenticated exact path").code(),
        "ZRYNA-W4003"
    );
    let first_extra_path = [WitSource::new("x".repeat(MAX_SOURCE_PATH_BYTES + 1), Vec::new())];
    assert_eq!(
        authenticate(&first_extra_path).expect_err("first extra path byte").code(),
        "ZRYNA-W4003"
    );
}
