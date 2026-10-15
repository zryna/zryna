//! Genuine ordinary source reaches the public scalar component byte ceiling.

use super::{audit, encode, exports, metadata};
use sha2::{Digest as _, Sha256};
use wasmparser::{Validator, WasmFeatures};
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};
use zryna_syntax::v2;

const FUNCTIONS: usize = 1_863;
const LIMIT: usize = 1_048_576;

fn span(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("bounded start"),
        end: u32::try_from(end).expect("bounded end"),
    }
}

fn source(wide: usize) -> (SourceMap, zryna_ir::VerifiedProgram) {
    let mut text = String::new();
    let functions = (0..FUNCTIONS)
        .map(|index| {
            let name = format!("f{index:04x}{}", "x".repeat(123));
            assert_eq!(name.len(), 128);
            let literal = if index < wide { "64" } else { "0" };
            let line = format!("export function {name}(): i32 {{ return {literal}; }}\n");
            let start = text.len();
            let name_start = start + 16;
            let ty = name_start + name.len() + 4;
            let body = ty + 4;
            let value = body + 9;
            text.push_str(&line);
            v2::RawFunctionSyntax {
                span: span(start, text.len() - 1),
                export_span: span(start, start + 6),
                function_span: span(start + 7, start + 15),
                name: v2::RawIdentifierSyntax {
                    text: name.clone(),
                    span: span(name_start, name_start + name.len()),
                },
                parameters: Vec::new(),
                result_type: v2::RawTypeSyntax {
                    span: span(ty, ty + 3),
                    kind: v2::RawTypeSyntaxKind::Named { name: "i32".into() },
                },
                body: v2::RawFunctionBodySyntax {
                    span: span(body, text.len() - 1),
                    statements: vec![v2::RawStatementSyntax {
                        span: span(body + 2, value + literal.len() + 1),
                        kind: v2::RawStatementKind::Return {
                            keyword_span: span(body + 2, body + 8),
                            value: 0,
                        },
                    }],
                    expressions: vec![v2::RawExpressionSyntax {
                        span: span(value, value + literal.len()),
                        kind: v2::RawExpressionKind::I32Literal { spelling: literal.into() },
                    }],
                },
            }
        })
        .collect::<Vec<_>>();
    let sources = SourceMap::build(vec![SourceFileInput { path: "boundary.zry".into(), text }])
        .expect("ordinary bounded source");
    let raw = v2::RawProjectSyntaxSnapshot {
        schema_version: 2,
        files: vec![v2::RawSourceUnit { id: 0, path: "boundary.zry".into(), functions }],
        diagnostics: Vec::new(),
    };
    let syntax =
        v2::verify_snapshot(raw, &sources).expect("every actual source token authenticated");
    let input = zryna_semantics::SemanticInput::try_new(&syntax, &sources)
        .expect("exact issuing source/syntax authority");
    let raw_program = zryna_semantics::lower(input).expect("ordinary public i32 source semantics");
    let program = zryna_ir::verify(raw_program, &sources).expect("mandatory complete scalar IR");
    (sources, program)
}

#[test]
fn public_browser_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers() {
    let (sources, program) = source(106);
    let wit = crate::pinned_wit_sources();
    let artifact = crate::emit_scalar_component(&program, &wit)
        .expect("public canonical browser factory accepts exact component boundary");
    assert_eq!(artifact.bytes().len(), LIMIT);
    assert_eq!(artifact.exports().len(), FUNCTIONS);
    assert_eq!(artifact.world_identity(), "zryna:capability-profiles/browser@0.1.0");
    assert!(artifact.world_audit().worlds()[0].explicit_imports().is_empty());
    artifact.revalidate(&program, &wit).expect("every byte rebuilt by the same public factory");
    let (extra_sources, extra_program) = source(107);
    let world = crate::wit_world_audit::AuthenticatedBrowserWorld::new(&wit)
        .expect("unchanged exact pinned WIT");
    let core =
        crate::emit(&extra_program).expect("next genuine source-emitted canonical scalar core");
    let exports = exports(&extra_program).expect("same complete canonical source interface");
    let metadata = metadata(world.source_digest());
    let extra = encode(&core, &exports, &metadata);
    assert_eq!(extra.len(), LIMIT + 1);
    Validator::new_with_features(WasmFeatures::WASM1.union(WasmFeatures::COMPONENT_MODEL))
        .validate_all(&extra)
        .expect("first extra is a valid complete canonical component");
    assert_eq!(
        audit::audit(&extra, &core, &exports, &metadata, &world)
            .expect_err("owning complete byte audit rejects first extra")
            .code(),
        "ZRYNA-W4021"
    );
    assert_eq!(
        crate::emit_scalar_component(&extra_program, &wit)
            .err()
            .expect("public factory rejects genuine source-emitted first extra")
            .code(),
        "ZRYNA-W4021"
    );
    let source_text = |sources: &SourceMap| {
        sources
            .source(sources.verify_file_id(0).expect("file"))
            .expect("actual source")
            .text()
            .to_owned()
    };
    println!(
        "browserFactoryByteBoundary={}",
        serde_json::json!({
            "sourceSha256": format!("{:x}", Sha256::digest(source_text(&sources))),
            "firstExtraSourceSha256": format!("{:x}", Sha256::digest(source_text(&extra_sources))),
            "componentBytes": artifact.bytes().len(),
            "componentSha256": format!("{:x}", Sha256::digest(artifact.bytes())),
            "firstExtraComponentBytes": extra.len(),
            "firstExtraComponentSha256": format!("{:x}", Sha256::digest(&extra)),
            "factoryRejection": "ZRYNA-W4021", "functions": FUNCTIONS
        })
    );
    artifact.revalidate(&program, &wit).expect("unchanged exact-boundary factory recovers");
}
