use super::fixtures;
use crate::{
    generic_closure::{test_source::CurrentSyntax, with_closure},
    source_scope::with_execution,
};
use std::sync::atomic::{AtomicBool, Ordering};
use zryna_source::NormalizedSourcePath;
#[test]
fn unused_original_opaque_clone_rejects_before_artifact_callback_with_original_span() {
    let (root, _, _) = fixtures::source("unused");
    let called = AtomicBool::new(false);
    let failure = with_closure(
        &root,
        NormalizedSourcePath::new("main.zry").expect("entry"),
        &CurrentSyntax::ordinary(),
        &|| Ok(()),
        |closure, checkpoint| {
            let text = closure
                .sources()
                .source(closure.sources().file_id(closure.entrypoint()).expect("actual entry"))
                .expect("original source")
                .text();
            let offset = u32::try_from(text.find("clone").expect("exact source operation"))
                .expect("bounded span");
            let result = with_execution(closure, checkpoint, |_, _, _| {
                called.store(true, Ordering::SeqCst);
                Ok(())
            });
            let error = result.expect_err("unused template original check");
            assert_eq!(error.kind(), crate::CommandFailureKind::Source);
            assert_eq!(error.diagnostics()[0].code(), "ZRYNA-M7002");
            let span = error.diagnostics()[0].primary_span().expect("original operation span");
            assert_eq!((span.start(), span.end()), (offset, offset + 5));
            Err::<(), _>(error)
        },
    )
    .expect_err("source barrier");
    assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-M7002");
    assert!(!called.load(Ordering::SeqCst));
    assert!(!root.active.load(Ordering::SeqCst));
}
#[test]
fn owned_receipt_from_independent_same_byte_source_world_never_matches_current_program() {
    let (first, _, _) = fixtures::source("scalar");
    let prior = with_closure(
        &first,
        NormalizedSourcePath::new("main.zry").expect("entry"),
        &CurrentSyntax::ordinary(),
        &|| Ok(()),
        |closure, checkpoint| {
            with_execution(closure, checkpoint, |program, receipt, _| {
                assert!(receipt.is_bound_to(program));
                Ok(receipt.clone())
            })
        },
    )
    .expect("first actual source owner");
    let (second, _, _) = fixtures::source("scalar");
    with_closure(
        &second,
        NormalizedSourcePath::new("main.zry").expect("entry"),
        &CurrentSyntax::ordinary(),
        &|| Ok(()),
        |closure, checkpoint| {
            with_execution(closure, checkpoint, |program, receipt, _| {
                assert_eq!(prior.execution_sha256(), receipt.execution_sha256());
                assert_eq!(prior.graph_sha256(), receipt.graph_sha256());
                assert!(!prior.is_bound_to(program));
                assert!(receipt.is_bound_to(program));
                Ok(())
            })
        },
    )
    .expect("foreign receipt rejected despite equal source/execution hashes");
}

#[test]
fn selected_imported_template_and_specialization_reject_at_original_exported_name() {
    for selected in ["identity", "identity<i32>"] {
        let (root, _, _) = fixtures::source("entry-one");
        let called = AtomicBool::new(false);
        let failure = with_closure(
            &root,
            NormalizedSourcePath::new("main.zry").expect("entry"),
            &CurrentSyntax::ordinary(),
            &|| Ok(()),
            |closure, checkpoint| {
                let original = closure
                    .sources()
                    .source(
                        closure
                            .sources()
                            .file_id(&NormalizedSourcePath::new("a.zry").expect("original module"))
                            .expect("source module"),
                    )
                    .expect("original function source");
                let offset = u32::try_from(original.text().find("identity").expect("name"))
                    .expect("bounded span");
                let failure = crate::source_scope::with_execution_for_export(
                    closure,
                    Some(selected),
                    checkpoint,
                    |_, _, _| {
                        called.store(true, Ordering::SeqCst);
                        Ok(())
                    },
                )
                .expect_err("compile-time template has no executable ABI entry");
                assert_eq!(failure.kind(), crate::CommandFailureKind::Source);
                assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-M7005");
                let span = failure.diagnostics()[0].primary_span().expect("exported name");
                assert_eq!(span.file(), original.id());
                assert_eq!((span.start(), span.end()), (offset, offset + 8));
                Err::<(), _>(failure)
            },
        )
        .expect_err("source selection barrier");
        assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-M7005");
        assert!(!called.load(Ordering::SeqCst));
    }
}
