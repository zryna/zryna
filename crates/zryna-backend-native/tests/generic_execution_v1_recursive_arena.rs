//! Native infrastructure-only owner checks; no linking or guest invocation.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "recursive_arena/mutations.rs"]
mod mutations;
#[path = "recursive_arena/source.rs"]
mod source;
use object::{Object, ObjectSection, ObjectSymbol};
use zryna_backend_native::generic_execution_v1 as backend;
const TLS: &str = "zryna_gexecution_traversal_tls";
const STUB: &str = "zryna_gexecution_tls_address";

#[test]
fn genuine_recursive_sources_require_exact_loader_prepared_storage_and_stub() {
    for case in ["drop-only", "clone", "controlled-cleanup"] {
        source::with_program(case, |program| {
            let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("real MIR");
            let target = zryna_backend_native::select_object_target("x86_64-unknown-linux-gnu")
                .expect("existing exact target");
            let artifact = backend::emit_object(&mir, target).expect("ordinary audited object");
            let file = object::File::parse(artifact.bytes()).expect("actual ELF");
            let tls = file.section_by_name(".tbss").expect("owning TLS storage admission");
            assert_eq!((tls.size(), tls.align()), (33_554_448, 16));
            assert!(tls.file_range().is_none());
            assert!(tls.data().expect("NOBITS").is_empty());
            let storage = file.symbols().find(|s| s.name().ok() == Some(TLS)).expect("local TLS");
            assert_eq!(storage.kind(), object::SymbolKind::Tls);
            assert_eq!(storage.scope(), object::SymbolScope::Compilation);
            assert_eq!(storage.size(), 33_554_448);
            let stub = file.symbols().find(|s| s.name().ok() == Some(STUB)).expect("address stub");
            assert_eq!(stub.size(), 17);
            let text = file.section_by_name(".text").expect("text");
            let begin = usize::try_from(stub.address()).expect("code address");
            assert_eq!(
                &text.data().expect("code")[begin..begin + 17],
                &[0x64, 0x48, 0x8b, 0x04, 0x25, 0, 0, 0, 0, 0x48, 0x8d, 0x80, 0, 0, 0, 0, 0xc3]
            );
            let tls_relocations = text
                .relocations()
                .filter(|(_, r)| {
                    r.flags()
                        == (object::RelocationFlags::Elf { r_type: object::elf::R_X86_64_TPOFF32 })
                })
                .collect::<Vec<_>>();
            assert_eq!(tls_relocations.len(), 1);
            assert_eq!(tls_relocations[0].0, stub.address() + 12);
            assert_eq!(tls_relocations[0].1.addend(), 0);
            backend::validate_object_inventory(artifact.bytes(), &mir).expect("same-owner audit");
            mutations::check(artifact.bytes(), &mir);
        });
    }
}
#[test]
fn erased_recursive_array_and_count_only_sources_keep_no_tls_inventory() {
    for case in ["erased", "counts"] {
        source::with_program(case, |program| {
            let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("actual MIR");
            let target = zryna_backend_native::select_object_target("x86_64-unknown-linux-gnu")
                .expect("exact target");
            let artifact = backend::emit_object(&mir, target).expect("ordinary object");
            let file = object::File::parse(artifact.bytes()).expect("ELF");
            assert!(file.section_by_name(".tbss").is_none());
            assert!(!file.symbols().any(|s| matches!(s.name().ok(), Some(TLS | STUB))));
        });
    }
}
