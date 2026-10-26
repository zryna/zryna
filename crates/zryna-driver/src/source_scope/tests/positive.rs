use super::fixtures;
use crate::{
    generic_closure::{VerifiedGenericClosure, test_source::CurrentSyntax, with_closure},
    source_scope::{
        GenericScalarExport, GenericSourceEdge, GenericSourceModule, GenericSourceReceipt,
        with_execution,
    },
};
use std::sync::atomic::Ordering;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_source::NormalizedSourcePath;
#[test]
fn genuine_existing_sources_issue_only_owned_receipts_and_artifacts() {
    for case in ["scalar", "entry-one", "mixed"] {
        let (root, packet, entry) = fixtures::source(case);
        let frontend = CurrentSyntax::ordinary();
        let owned = with_closure(
            &root,
            NormalizedSourcePath::new("main.zry").expect("entry"),
            &frontend,
            &|| Ok(()),
            |closure, checkpoint| {
                let frozen = zryna_syntax::v5::verify_snapshot(
                    zryna_syntax::v5::decode_snapshot(&fixtures::packet(packet))
                        .expect("actual frozen TS DTO"),
                    closure.sources(),
                )
                .expect("exact original TS source authority");
                assert_eq!(closure.syntax().files(), frozen.files());
                assert_eq!(closure.syntax().advisories(), frozen.advisories());
                with_execution(closure, checkpoint, |program, receipt, checkpoint| {
                    assert!(root.active.load(Ordering::SeqCst));
                    checkpoint()?;
                    metadata(closure, program, receipt, entry);
                    Ok((receipt.clone(), artifacts(program)))
                })
            },
        )
        .expect("scoped source issuer");
        assert_eq!(owned.0.entry_module(), entry);
        assert!(!owned.1.0.is_empty() && !owned.1.1.is_empty() && !owned.1.2.is_empty());
        assert!(!root.active.load(Ordering::SeqCst));
        assert_eq!(frontend.calls.load(Ordering::SeqCst), if entry == 1 { 3 } else { 2 });
    }
}
fn metadata(
    closure: &VerifiedGenericClosure,
    program: &VerifiedExecution<'_>,
    receipt: &GenericSourceReceipt,
    entry: u32,
) {
    assert!(receipt.is_bound_to(program));
    assert_eq!(receipt.profile(), "bounded-generics-v1");
    assert_eq!(receipt.syntax_protocol(), 5);
    assert_eq!(receipt.entrypoint().as_str(), "main.zry");
    assert_eq!(receipt.entry_module(), entry);
    assert_eq!(program.entry_file().index(), entry);
    let modules: &[GenericSourceModule] = receipt.modules();
    assert_eq!(modules.len(), if entry == 1 { 2 } else { 1 });
    for (index, module) in modules.iter().enumerate() {
        assert_eq!(usize::try_from(module.id()).expect("bounded id"), index);
        assert!(!module.path().is_empty());
        assert!(module.bytes() > 0);
        assert_ne!(module.sha256(), &[0; 32]);
    }
    let edges: &[GenericSourceEdge] = receipt.edges();
    assert_eq!(edges.len(), usize::from(entry == 1));
    if entry == 1 {
        let edge = &edges[0];
        assert_eq!(
            (edge.importer(), edge.target(), edge.specifier(), edge.imported(), edge.local()),
            ("main.zry", "a.zry", "./a.zry", "identity", "identity")
        );
    }
    assert_eq!(receipt.graph_sha256(), closure.graph_sha256());
    for digest in [
        receipt.execution_sha256(),
        receipt.type_universe_sha256(),
        receipt.linear32_sha256(),
        receipt.linux_x86_64_sha256(),
        receipt.runtime_abi_sha256(),
    ] {
        assert_ne!(digest, &[0; 32]);
    }
    assert_ne!(receipt.linear32_sha256(), receipt.linux_x86_64_sha256());
    let exports: &[GenericScalarExport] = receipt.exports();
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].name(), "entry");
    assert!(exports[0].parameters().is_empty());
    assert_eq!(exports[0].result(), zryna_abi::ScalarType::I32);
}
fn artifacts(program: &VerifiedExecution<'_>) -> (String, Vec<u8>, Vec<u8>) {
    let js =
        zryna_backend_javascript::generic_execution_v1::emit(program).expect("NEW ordinary JS");
    let wasm =
        zryna_backend_webassembly::generic_execution_v1::emit(program).expect("NEW ordinary Wasm");
    let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("same NEW native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified object target");
    let native = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("audited NEW native object");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(native.bytes(), &mir)
        .expect("independent native inventory");
    (js.source, wasm.bytes().to_vec(), native.bytes().to_vec())
}
