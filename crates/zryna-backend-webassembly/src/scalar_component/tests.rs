use sha2::{Digest, Sha256};
use wasmparser::{Parser, Payload};
use zryna_ir::{Expr, ExprId, ExprKind, Function, Program, Type};
use zryna_source::{SourceFileInput, SourceMap};

use crate::{emit_scalar_component, pinned_wit_sources};

use super::{audit, component_export_name, encode, exports, metadata};

fn program() -> zryna_ir::VerifiedProgram {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "add.zry".into(), text: "add".into() }])
            .expect("source map");
    let file = sources.verify_file_id(0).expect("source identity");
    let span = sources.span(file, 0, 3).expect("source span");
    zryna_ir::verify(
        Program {
            functions: vec![Function {
                name: "add".into(),
                parameters: vec![Type::I32, Type::I32],
                return_type: Type::I32,
                expressions: vec![
                    Expr { ty: Type::I32, span, kind: ExprKind::Parameter(0) },
                    Expr { ty: Type::I32, span, kind: ExprKind::Parameter(1) },
                    Expr {
                        ty: Type::I32,
                        span,
                        kind: ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) },
                    },
                ],
                body: ExprId(2),
            }],
        },
        &sources,
    )
    .expect("verified scalar fixture")
}

fn program_with_names(names: &[&str]) -> zryna_ir::VerifiedProgram {
    let text = names.join(" ");
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "names.zry".into(), text: text.clone() }])
            .expect("source map");
    let file = sources.verify_file_id(0).expect("source identity");
    let mut cursor = 0;
    let functions = names
        .iter()
        .map(|name| {
            let start = cursor + text[cursor..].find(*name).expect("fixture name");
            let end = start + name.len();
            cursor = end;
            let span = sources
                .span(
                    file,
                    u32::try_from(start).expect("fixture start"),
                    u32::try_from(end).expect("fixture end"),
                )
                .expect("source span");
            Function {
                name: (*name).to_owned(),
                parameters: Vec::new(),
                return_type: Type::I32,
                expressions: vec![Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(0) }],
                body: ExprId(0),
            }
        })
        .collect();
    zryna_ir::verify(Program { functions }, &sources).expect("verified named scalar fixture")
}

fn component_export_names(bytes: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::ComponentExportSection(exports) = payload.expect("component payload") {
            for export in exports {
                names.push(export.expect("component export").name.name.to_owned());
            }
        }
    }
    names
}

#[test]
fn component_is_reproducible_and_retains_the_exact_core_and_world() {
    let program = program();
    let sources = pinned_wit_sources();
    let first = emit_scalar_component(&program, &sources).expect("first component");
    let second = emit_scalar_component(&program, &sources).expect("second component");
    assert_eq!(first.bytes(), second.bytes());
    let digest: [u8; 32] = Sha256::digest(first.bytes()).into();
    assert_eq!(first.digest(), &digest);
    assert_eq!(first.core().bytes(), crate::emit(&program).expect("core").bytes());
    assert_eq!(first.world_identity(), "zryna:capability-profiles/browser@0.1.0");
    assert!(first.world_audit().worlds()[0].explicit_imports().is_empty());
    first.revalidate(&program, &sources).expect("matching binding");
    assert_eq!(
        first.digest(),
        &[
            0x34, 0x8b, 0xf3, 0x2d, 0xaf, 0xe8, 0xa1, 0x46, 0x7d, 0xc3, 0x90, 0x64, 0xdd, 0x49,
            0x96, 0x3d, 0xaf, 0x4a, 0x95, 0x17, 0x53, 0xf9, 0x3f, 0x29, 0xe6, 0x06, 0x66, 0x89,
            0x67, 0x81, 0x23, 0x43,
        ]
    );
}

#[test]
fn every_admitted_logical_name_maps_to_an_exact_collision_free_component_label() {
    let logical = ["lower", "camelCase", "snake_case", "_leading", "UPPER"];
    let expected = [
        "zryna-export-6c6f776572",
        "zryna-export-63616d656c43617365",
        "zryna-export-736e616b655f63617365",
        "zryna-export-5f6c656164696e67",
        "zryna-export-5550504552",
    ];
    for (logical, expected) in logical.iter().zip(&expected) {
        assert_eq!(component_export_name(logical), *expected);
    }
    assert_ne!(component_export_name("name"), component_export_name("NAME"));

    let program = program_with_names(&logical);
    let component =
        emit_scalar_component(&program, &pinned_wit_sources()).expect("mapped component");
    assert_eq!(component_export_names(component.bytes()), expected.map(str::to_owned));
}

#[test]
fn undeclared_component_import_is_rejected() {
    let program = program();
    let sources = pinned_wit_sources();
    let component = emit_scalar_component(&program, &sources).expect("baseline component");
    let mut changed = wasm_encoder::ComponentBuilder::default();
    let function_type = {
        let (function_type, mut function) = changed.type_function(Some("injected"));
        function.params(std::iter::empty::<(&str, wasm_encoder::ComponentValType)>());
        function.result(Some(wasm_encoder::PrimitiveValType::S32.into()));
        function_type
    };
    changed.import("injected", wasm_encoder::ComponentTypeRef::Func(function_type));
    let changed = changed.finish();
    let world =
        crate::wit_world_audit::AuthenticatedBrowserWorld::new(&sources).expect("browser world");
    let failure = audit::audit(
        &changed,
        &component.core,
        &exports(&program).expect("exports"),
        &metadata(world.source_digest()),
        &world,
    )
    .expect_err("undeclared import must fail");
    assert_eq!(failure.code(), "ZRYNA-W4021");
}

#[test]
fn substituted_wit_and_component_identity_fail_before_authority_is_returned() {
    let program = program();
    let mut sources = pinned_wit_sources();
    let mut changed_source = sources[0].bytes().to_vec();
    changed_source[0] ^= 1;
    sources[0] = crate::WitSource::new(sources[0].path(), changed_source);
    let wit_error =
        emit_scalar_component(&program, &sources).err().expect("substituted WIT source must fail");
    assert_eq!(wit_error.code(), "ZRYNA-W4001");

    let sources = pinned_wit_sources();
    let component = emit_scalar_component(&program, &sources).expect("baseline component");
    let mut changed = component.bytes().to_vec();
    let identity = b"zryna:capability-profiles/browser@0.1.0";
    let offset = changed
        .windows(identity.len())
        .position(|bytes| bytes == identity)
        .expect("embedded world identity");
    changed[offset + identity.len() - 1] = b'1';
    let world =
        crate::wit_world_audit::AuthenticatedBrowserWorld::new(&sources).expect("browser world");
    let failure = audit::audit(
        &changed,
        &component.core,
        &exports(&program).expect("exports"),
        &metadata(world.source_digest()),
        &world,
    )
    .expect_err("changed identity must fail");
    assert_eq!(failure.code(), "ZRYNA-W4021");
}

#[test]
fn undeclared_alias_and_malformed_topology_are_rejected() {
    let program = program();
    let sources = pinned_wit_sources();
    let component = emit_scalar_component(&program, &sources).expect("baseline component");
    let mut changed = component.bytes().to_vec();
    let alias_range = Parser::new(0)
        .parse_all(&changed)
        .find_map(|payload| match payload.expect("component payload") {
            Payload::ComponentAliasSection(aliases) => Some(aliases.range()),
            _ => None,
        })
        .expect("component alias section");
    let alias_range = usize::try_from(alias_range.start).expect("alias start")
        ..usize::try_from(alias_range.end).expect("alias end");
    let alias = b"add";
    let relative = changed[alias_range.clone()]
        .windows(alias.len())
        .position(|bytes| bytes == alias)
        .expect("core alias name");
    changed[alias_range.start + relative + alias.len() - 1] = b'b';
    assert_eq!(
        component_export_names(&changed),
        [component_export_name("add")],
        "alias mutation must leave the public export section unchanged"
    );
    let world =
        crate::wit_world_audit::AuthenticatedBrowserWorld::new(&sources).expect("browser world");
    assert!(
        audit::audit(
            &changed,
            &component.core,
            &exports(&program).expect("exports"),
            &metadata(world.source_digest()),
            &world,
        )
        .is_err()
    );

    let mut malformed = component.bytes().to_vec();
    malformed.push(0);
    assert!(
        audit::audit(
            &malformed,
            &component.core,
            &exports(&program).expect("exports"),
            &metadata(world.source_digest()),
            &world,
        )
        .is_err()
    );
}

#[test]
fn complete_component_bytes_accept_the_exact_byte_limit_and_reject_the_first_extra() {
    let program = program();
    let sources = pinned_wit_sources();
    let component = emit_scalar_component(&program, &sources).expect("baseline component");
    let exports = exports(&program).expect("exports");
    let world =
        crate::wit_world_audit::AuthenticatedBrowserWorld::new(&sources).expect("browser world");
    let mut metadata_size = audit::MAX_COMPONENT_BYTES;
    let (metadata, exact) = (0..8)
        .find_map(|_| {
            let expected_metadata = "x".repeat(metadata_size);
            let bytes = encode(&component.core, &exports, &expected_metadata);
            match bytes.len().cmp(&audit::MAX_COMPONENT_BYTES) {
                std::cmp::Ordering::Equal => Some((expected_metadata, bytes)),
                std::cmp::Ordering::Greater => {
                    metadata_size -= bytes.len() - audit::MAX_COMPONENT_BYTES;
                    None
                }
                std::cmp::Ordering::Less => {
                    metadata_size += audit::MAX_COMPONENT_BYTES - bytes.len();
                    None
                }
            }
        })
        .expect("component custom metadata must reach the exact byte ceiling");
    audit::audit(&exact, &component.core, &exports, &metadata, &world)
        .expect("complete component at exact byte ceiling");

    let mut first_extra = exact;
    first_extra.push(0);
    let failure = audit::audit(&first_extra, &component.core, &exports, &metadata, &world)
        .expect_err("complete component with first extra byte");
    assert_eq!(failure.code(), "ZRYNA-W4021");
}

#[test]
fn complete_component_bytes_accept_exact_interfaces_and_reject_the_first_extra() {
    let program = program_with_names(&["first", "second"]);
    let sources = pinned_wit_sources();
    let world =
        crate::wit_world_audit::AuthenticatedBrowserWorld::new(&sources).expect("browser world");
    let core = crate::emit(&program).expect("core");
    let exports = exports(&program).expect("exports");
    let metadata = metadata(world.source_digest());
    let bytes = encode(&core, &exports, &metadata);
    audit::audit(&bytes, &core, &exports, &metadata, &world)
        .expect("complete component at exact interface ceiling");

    let failure = audit::audit(&bytes, &core, &exports[..1], &metadata, &world)
        .expect_err("complete component with first extra interface entry");
    assert_eq!(failure.code(), "ZRYNA-W4021");
}
