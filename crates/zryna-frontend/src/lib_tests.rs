use super::*;
use zryna_source::SourceFileInput;

fn sources() -> SourceMap {
    SourceMap::build(vec![
        SourceFileInput {
            path: "src/z.zry".to_owned(),
            text: "export function z(): i32 { return 1; }".to_owned(),
        },
        SourceFileInput {
            path: "src/a.zry".to_owned(),
            text: "export function a(): i32 { return 1; }".to_owned(),
        },
    ])
    .expect("fixture source map must be valid")
}

fn raw_file(id: u32, path: &str, end: u32) -> RawSourceUnit {
    RawSourceUnit {
        id,
        path: path.to_owned(),
        functions: vec![RawFunctionSyntax {
            name: "value".to_owned(),
            parameters: Vec::new(),
            return_type: TypeSyntax::Named("i32".to_owned()),
            span: UntrustedSpan { file: id, start: 0, end },
        }],
    }
}

#[test]
fn verifies_the_exact_file_set_and_authoritative_spans() {
    let sources = sources();
    let raw = RawProjectSyntaxSnapshot {
        schema_version: FRONTEND_PROTOCOL_VERSION,
        files: vec![raw_file(1, "src/z.zry", 38), raw_file(0, "src/a.zry", 38)],
        diagnostics: Vec::new(),
    };
    let snapshot = verify_snapshot(raw, &sources).expect("complete snapshot must verify");
    assert_eq!(
        snapshot.files.iter().map(|file| (file.id.index(), file.path.as_str())).collect::<Vec<_>>(),
        vec![(0, "src/a.zry"), (1, "src/z.zry")]
    );
    for file in &snapshot.files {
        assert!(sources.resolve(file.functions[0].span).is_ok());
    }
}

#[test]
fn rejects_duplicate_missing_unknown_and_malformed_provider_data() {
    let sources = sources();
    let raw = RawProjectSyntaxSnapshot {
        schema_version: FRONTEND_PROTOCOL_VERSION,
        files: vec![raw_file(0, "src/a.zry", 38), raw_file(0, "bad\npath.zry", u32::MAX)],
        diagnostics: Vec::new(),
    };
    let diagnostics = verify_snapshot(raw, &sources).expect_err("duplicate snapshot must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.message().contains("repeats")));
    assert!(diagnostics.iter().all(|diagnostic| diagnostic.path() != Some("bad\npath.zry")));

    let raw = RawProjectSyntaxSnapshot {
        schema_version: FRONTEND_PROTOCOL_VERSION,
        files: vec![raw_file(0, "src/a.zry", u32::MAX), raw_file(7, "src/z.zry", 0)],
        diagnostics: Vec::new(),
    };
    let diagnostics =
        verify_snapshot(raw, &sources).expect_err("unknown and invalid spans must fail");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.message().contains("invalid source span"))
    );
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.message().contains("unknown file")));
}

#[test]
fn rejects_noncanonical_diagnostic_paths_and_bounded_wire_failures() {
    let sources = sources();
    let raw = RawProjectSyntaxSnapshot {
        schema_version: FRONTEND_PROTOCOL_VERSION,
        files: vec![raw_file(0, "src/a.zry", 38), raw_file(1, "src/z.zry", 38)],
        diagnostics: vec![RawProviderDiagnostic {
            code: "TS1000".to_owned(),
            severity: Severity::Error,
            path: Some("SRC/A.ZRY".to_owned()),
            message: "provider error".to_owned(),
            guidance: "fix it".to_owned(),
        }],
    };
    let diagnostics =
        verify_snapshot(raw, &sources).expect_err("case-variant provider path must fail");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.message().contains("non-canonical source path") })
    );

    let oversized = vec![b' '; MAX_PROVIDER_RESPONSE_BYTES + 1];
    assert!(decode_snapshot(&oversized).is_err());
    assert!(
        decode_snapshot(br#"{"schema_version":1,"files":[],"diagnostics":[],"extra":true}"#)
            .is_err()
    );
    assert!(decode_snapshot(br#"{"schema_version":1,"files":[{"id":-1,"path":"a.zry","functions":[]}],"diagnostics":[]}"#).is_err());
}

#[test]
fn protocol_v2_requires_an_exact_explicit_handshake() {
    let mut info = ProviderInfo {
        provider: "fixture".to_owned(),
        provider_version: "1.0.0".to_owned(),
        protocol_version: syntax_v2::PROTOCOL_VERSION,
        capabilities: FrontendCapabilities {
            module_resolution: false,
            semantic_diagnostics: false,
        },
    };
    assert!(verify_provider_v2(&info).is_ok());
    assert!(verify_provider(&info).is_err());

    info.protocol_version = FRONTEND_PROTOCOL_VERSION;
    assert!(verify_provider(&info).is_ok());
    assert!(verify_provider_v2(&info).is_err());
}
