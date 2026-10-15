use super::*;
use crate::native::native_c_v0::foreign_library::{ForeignLibraryInput, capture_foreign_library};
use crate::native::native_c_v0::{resource_identity, test_capture};
use object::{
    Architecture, BinaryFormat, Endianness, SectionKind, SymbolFlags, SymbolKind, SymbolScope,
    write::{Object, Symbol, SymbolSection},
};

fn issued_requirements() -> HandleLinkRequirements {
    let original = test_capture::reference();
    let ir = zryna_native_c_ir::lower(&original.sources, &original.authority)
        .expect("genuine source-bound IR");
    let mir = zryna_native_mir::native_c_v0::lower(&ir).expect("original MIR issuer");
    let symbol = &mir
        .functions()
        .find(|function| function.name() == "imported")
        .expect("original imported entry")
        .entry()
        .symbol;
    let object = zryna_backend_native::native_c_v0::resources::emit_handle_entries(
        &mir,
        &[symbol],
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("exact Linux x86-64 object target"),
    )
    .expect("genuine sealed program object");
    resource_identity::handle_link_requirements(&object)
        .expect("retained original link requirements")
}
fn library(requirements: &HandleLinkRequirements) -> CapturedForeignLibrary {
    let mut object = Object::new(BinaryFormat::Elf, Architecture::X86_64, Endianness::Little);
    let text = object.section_id(object::write::StandardSection::Text);
    object.append_section_data(text, &[0xc3; 7], 1);
    object.add_section(Vec::new(), b".note.GNU-stack".to_vec(), SectionKind::Other);
    for (index, name) in [
        "add",
        "fixture_close",
        "fixture_copy_bytes",
        "fixture_open",
        "fixture_read",
        "fixture_release_bytes",
        "sum_bytes",
    ]
    .iter()
    .enumerate()
    {
        object.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: index as u64,
            size: 1,
            kind: SymbolKind::Text,
            scope: SymbolScope::Linkage,
            weak: false,
            section: SymbolSection::Section(text),
            flags: SymbolFlags::None,
        });
    }
    let bytes = object.write().expect("encode independent seven-definition foreign ELF");
    let authority = requirements
        .object()
        .program()
        .source()
        .private_authority()
        .body_authority()
        .declaration_authority();
    capture_foreign_library(
        requirements,
        &ForeignLibraryInput {
            library_id: "fixture-c-v0@0",
            header_bytes: authority
                .header_bytes("fixture-c-v0@0")
                .expect("original normative library header"),
            policy_bytes: authority
                .policy_bytes("fixture-c-v0@0")
                .expect("original normative operation policy"),
            object_bytes: &bytes,
            object_size: bytes.len(),
            object_sha256: &Sha256::digest(&bytes).into(),
            dependency_symbols: &[],
        },
    )
    .expect("audited independent foreign fixture")
}

#[test]
fn retained_original_issuer_clone_and_explicit_library_identity() {
    let fixture = Fixture::new();
    let requirements = issued_requirements();
    let library = library(&requirements);
    // This structural fixture tests capability binding only, not acquisition/provider qualification.
    let retained = RetainedLinkInputs {
        requirements: requirements.clone(),
        library: library.clone(),
        expectation: expectation(&fixture.0),
        materials: vec![],
    };
    retained
        .check_binding(&requirements.clone(), &library.clone())
        .expect("clone preserves genuine issuer");
    let other = issued_requirements();
    assert_eq!(requirements.object_sha256(), other.object_sha256());
    assert!(retained.check_binding(&other, &library).is_err());
    let mut changed = library.clone();
    changed.library_id = "another-library@0".into();
    assert_eq!(changed.bytes(), library.bytes());
    assert!(retained.check_binding(&requirements, &changed).is_err());
}

#[test]
fn independent_material_pin_rejects_same_size_substitution_and_small_declared_budget() {
    let fixture = Fixture::new();
    let requirements = issued_requirements();
    let library = library(&requirements);
    let mut expected = expectation(&fixture.0);
    fs::write(fixture.0.join("archive"), b"abd").expect("write equal-size hostile material");
    assert!(capture(&requirements, &library, &expected).is_err());
    fs::write(fixture.0.join("archive"), b"abc").expect("restore independently fixed material");
    expected.materials[0].bytes = 1;
    assert!(capture(&requirements, &library, &expected).is_err());
}

#[test]
fn selected_retained_snapshot_keeps_expectation_and_arc_bytes_immutable() {
    let fixture = Fixture::new();
    let requirements = issued_requirements();
    let library = library(&requirements);
    let mut expected = expectation(&fixture.0);
    let mut bytes = b"abc".to_vec();
    // Binding/snapshot fixture deliberately does not assert a provider-complete capture.
    let retained = RetainedLinkInputs {
        requirements,
        library,
        expectation: expected.clone(),
        materials: vec![RetainedMaterial {
            expected: expected.materials[0].clone(),
            bytes: bytes.clone().into(),
        }],
    };
    expected.materials[0].sha256 = [0; 32];
    bytes.fill(0);
    assert_eq!(
        retained.exported_materials().collect::<Vec<_>>(),
        vec![("archive", b"abc".as_slice())]
    );
    assert_eq!(retained.report()["production_approval"], false);
    assert_eq!(retained.report()["execution_authorized"], false);
    assert_eq!(retained.report()["complete_input_use_attestation"], false);
    assert_eq!(retained.expectation.materials[0].sha256, Sha256::digest(b"abc").as_slice());
}
