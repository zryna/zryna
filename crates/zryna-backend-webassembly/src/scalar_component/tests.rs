use sha2::{Digest, Sha256};
use zryna_ir::{Expr, ExprId, ExprKind, Function, Program, Type};
use zryna_source::{SourceFileInput, SourceMap};

use crate::{emit_scalar_component, pinned_wit_sources};

use super::{audit, exports, metadata};

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

#[test]
fn component_is_reproducible_and_retains_the_exact_core_and_world() {
    let program = program();
    let sources = pinned_wit_sources();
    let first = emit_scalar_component(&program, &sources).expect("first component");
    let second = emit_scalar_component(&program, &sources).expect("second component");
    assert_eq!(first.bytes(), second.bytes());
    let digest: [u8; 32] = Sha256::digest(first.bytes()).into();
    assert_eq!(first.digest(), &digest);
    assert_eq!(
        first.digest(),
        &[
            0xd7, 0x7c, 0x08, 0xa3, 0x4c, 0x8f, 0xb3, 0xea, 0xa7, 0x2d, 0xf1, 0x37, 0xb6, 0xea,
            0x27, 0x06, 0xf4, 0x5d, 0x1e, 0x5d, 0xb3, 0x03, 0xb4, 0xbb, 0xe2, 0xe3, 0x23, 0x4b,
            0x9b, 0x16, 0x9d, 0x7c,
        ]
    );
    assert_eq!(first.core().bytes(), crate::emit(&program).expect("core").bytes());
    assert_eq!(first.world_identity(), "zryna:capability-profiles/browser@0.1.0");
    assert!(first.world_audit().worlds()[0].explicit_imports().is_empty());
    first.revalidate(&program, &sources).expect("matching binding");
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
    let export = b"add";
    let offset =
        changed.windows(export.len()).rposition(|bytes| bytes == export).expect("component export");
    changed[offset + export.len() - 1] = b'b';
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
fn component_resource_budgets_accept_exact_limits_and_reject_first_extra() {
    audit::check_byte_budget(audit::MAX_COMPONENT_BYTES).expect("exact byte ceiling");
    let failure =
        audit::check_byte_budget(audit::MAX_COMPONENT_BYTES + 1).expect_err("first extra byte");
    assert_eq!(failure.code(), "ZRYNA-W4021");

    audit::check_interface_budget(0, 1).expect("exact interface ceiling");
    let failure = audit::check_interface_budget(1, 1).expect_err("first extra interface entry");
    assert_eq!(failure.code(), "ZRYNA-W4021");
}
