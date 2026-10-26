//! A live recursive owner with only Copy Clone must have no reachable rollback call.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_iterative_drop/source.rs"]
mod source;
use object::{Object, ObjectSection, ObjectSymbol, RelocationTarget};
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, VerifiedOperation};

#[test]
fn authenticated_copy_only_clone_retains_live_owner_without_rollback_edge() {
    source::with_program("copy-only-under-live-root", |provider, program, _wire| {
        let kinds = program
            .operations()
            .filter_map(VerifiedOperation::clone_frontier)
            .map(|frontier| frontier.frontier().kind())
            .collect::<Vec<_>>();
        assert!(!kinds.is_empty());
        assert!(kinds.iter().all(|kind| *kind == CloneNodeKind::Copy));
        let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("actual MIR");
        assert!(mir.requires_recursive_arena());
        assert!(mir.containers().expect("actual containers").clone_symbols().any(|s| s.is_some()));
        let target =
            zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
                .expect("supported object target");
        let object = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
            .unwrap_or_else(|error| panic!("{provider} Copy-only owning object: {error:?}"));
        let elf = object::File::parse(object.bytes()).expect("independent actual ELF");
        let clone = elf
            .symbols()
            .find(|symbol| symbol.name().ok() == Some("zryna_gexecution_iter_clone"))
            .expect("actual flat Clone body");
        let end = clone.address().checked_add(clone.size()).expect("bounded extent");
        let mut prefix_drops = 0;
        for (offset, relocation) in elf.section_by_name(".text").expect("actual text").relocations()
        {
            if !(clone.address()..end).contains(&offset) {
                continue;
            }
            if let RelocationTarget::Symbol(index) = relocation.target()
                && elf.symbol_by_index(index).expect("actual callee").name().ok()
                    == Some("zryna_gexecution_iter_drop")
            {
                prefix_drops += 1;
            }
        }
        assert_eq!(prefix_drops, 0, "Copy-only Clone has no source failure predecessor");
    });
}
