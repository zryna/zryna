//! Genuine source/owned seals enter native MIR and its independent final object audit.
use object::{Object, ObjectSection, ObjectSymbol, RelocationTarget};
use zryna_layout::StorageTarget;

#[path = "generic_owned_nominal_enum/amplification.rs"]
mod amplification;
#[path = "generic_owned_nominal_enum/audit.rs"]
mod audit;
#[path = "../../../tests/m7-generic-owned-native/hostile.rs"]
mod hostile;
#[path = "generic_owned_nominal_enum/support.rs"]
mod support;
use support::with_program;

#[test]
fn genuine_nominal_enums_emit_deterministic_audited_native_objects() {
    for cross in [false, true] {
        with_program(cross, |program| {
            let mir = zryna_native_mir::generic_owned_v2::lower(program)
                .expect("bounded nominal member/drop expansion before native IR");
            let views: Vec<_> = program.layouts(StorageTarget::LinuxX8664V1).types().collect();
            for (declaration, expected_drop, width) in
                [(0_u32, true, 6), (1, false, 2), (2, true, 4), (3, true, 4)]
            {
                let view = views
                    .iter()
                    .find(|view| {
                        matches!(view.key()[0], 0x11 | 0x13)
                            && u32::from_le_bytes(
                                view.key()[5..9].try_into().expect("declaration key"),
                            ) == declaration
                    })
                    .expect("demanded source nominal identity");
                assert_eq!(
                    view.drop_kind() != 0,
                    expected_drop,
                    "payload-derived owned status, including phantom/fixed arguments"
                );
                assert_eq!(
                    mir.width(zryna_ir::generic_v1::raw::Type::Stored(view.id().index()))
                        .expect("sealed lanes"),
                    width
                );
            }
            let target = zryna_backend_native::select_object_target(
                zryna_backend_native::NATIVE_OBJECT_TARGET,
            )
            .expect("qualified native target");
            let object = zryna_backend_native::generic_owned_v2::emit_object(&mir, target)
                .expect("active-payload nominal cleanup and independently audited relocations");
            assert_eq!(
                object,
                zryna_backend_native::generic_owned_v2::emit_object(&mir, target)
                    .expect("deterministic nominal object replay")
            );
            zryna_backend_native::generic_owned_v2::validate_object_inventory(object.bytes(), &mir)
                .expect("exact original relocation inventory");
            audit::hostile_release_inventory(object.bytes(), &mir);
            let file = object::File::parse(object.bytes()).expect("independent ELF reader");
            let release = program
                .runtime()
                .declarations()
                .native_linux_x86_64
                .iter()
                .find(|entry| entry.operation == "stringRelease")
                .expect("fixed runtime release")
                .symbol
                .as_str();
            let releases = file
                .section_by_name(".text")
                .expect("code section")
                .relocations()
                .filter(|(_, relocation)| {
                    let RelocationTarget::Symbol(index) = relocation.target() else { return false };
                    file.symbol_by_index(index).expect("bound symbol").name().expect("name")
                        == release
                })
                .count();
            assert!(releases > 0, "owned active payloads require real audited release calls");
        });
    }
}
