use super::*;
use sha2::{Digest, Sha256};

fn add_module() -> VerifiedMirModule {
    let function = zryna_native_mir::raw::Function::new(
        "add".to_owned(),
        zryna_native_mir::raw::CallingConvention::ZRYNA_INTERNAL_I32_V1,
        zryna_native_mir::raw::Signature::new(vec![MirType::I32, MirType::I32], MirType::I32),
        vec![
            zryna_native_mir::raw::ValueDefinition::new(
                zryna_native_mir::raw::ValueId::new(0),
                MirType::I32,
                zryna_native_mir::raw::Operation::Parameter { index: 0 },
            ),
            zryna_native_mir::raw::ValueDefinition::new(
                zryna_native_mir::raw::ValueId::new(1),
                MirType::I32,
                zryna_native_mir::raw::Operation::Parameter { index: 1 },
            ),
            zryna_native_mir::raw::ValueDefinition::new(
                zryna_native_mir::raw::ValueId::new(2),
                MirType::I32,
                zryna_native_mir::raw::Operation::I32Add {
                    lhs: zryna_native_mir::raw::ValueId::new(0),
                    rhs: zryna_native_mir::raw::ValueId::new(1),
                },
            ),
        ],
        zryna_native_mir::raw::ValueId::new(2),
    );
    zryna_native_mir::verify(zryna_native_mir::raw::Module::new(vec![function]))
        .expect("add MIR must verify")
}

#[test]
fn public_entry_accepts_only_verified_mir() {
    let _: fn(&VerifiedMirModule) -> Result<LlvmIrArtifact, Diagnostic> = emit_llvm_ir;
    let verified = zryna_native_mir::verify(zryna_native_mir::raw::Module::new(Vec::new()))
        .expect("empty raw MIR must verify");
    assert_eq!(emit_llvm_ir(&verified).expect("empty module must emit").source, "");
    let target = select_object_target(NATIVE_OBJECT_TARGET).expect("supported target");
    assert!(emit_object(&verified, target).is_ok());
}

#[test]
fn selects_only_the_exact_linux_target() {
    assert!(select_object_target(NATIVE_OBJECT_TARGET).is_ok());
    for rejected in [
        "",
        "X86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "x86_64-pc-windows-msvc",
        "aarch64-unknown-linux-gnu",
        "amd64-linux",
    ] {
        assert_eq!(
            select_object_target(rejected).expect_err("near-match target must fail").code(),
            "ZRYNA-N3001"
        );
    }
}

#[test]
fn emits_deterministic_audited_linux_elf_object() {
    let module = add_module();
    let target = select_object_target(NATIVE_OBJECT_TARGET).expect("supported target");
    let first = emit_object(&module, target).expect("object emission");
    let second = emit_object(&module, target).expect("repeat object emission");
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(
        Sha256::digest(first.bytes()).as_slice(),
        &[
            0x0c, 0xeb, 0x5e, 0x55, 0x65, 0x2d, 0x36, 0xe0, 0xac, 0xe1, 0xec, 0x48, 0xd8, 0xe5,
            0x43, 0xb2, 0x49, 0x7e, 0x4f, 0xe8, 0xc8, 0x3e, 0x74, 0x48, 0xc4, 0x65, 0xe7, 0xdb,
            0x5c, 0x02, 0xf7, 0xe3,
        ]
    );

    let file = object::File::parse(first.bytes()).expect("audited object must parse");
    assert_eq!(file.format(), BinaryFormat::Elf);
    assert_eq!(file.architecture(), object::Architecture::X86_64);
    assert_eq!(file.endianness(), Endianness::Little);
    assert_eq!(file.kind(), ObjectKind::Relocatable);
    assert!(file.is_64());
    let exports = file
        .symbols()
        .filter(|symbol| symbol.is_global() && symbol.kind() == object::SymbolKind::Text)
        .map(|symbol| symbol.name().expect("audited UTF-8 symbol"))
        .collect::<Vec<_>>();
    assert_eq!(exports, ["zryna_v1_e_add"]);
    assert!(file.symbols().all(|symbol| !symbol.is_undefined()));
    assert!(file.sections().all(|section| section.relocations().next().is_none()));
}

#[test]
fn object_audit_rejects_corrupt_and_mismatched_bytes_stably() {
    let module = add_module();
    assert_eq!(
        audit_object(b"not an object", &module).expect_err("corrupt bytes").code(),
        "ZRYNA-N3003"
    );
    assert_eq!(
        audit_object(&vec![0_u8; MAX_NATIVE_OBJECT_BYTES + 1], &module)
            .expect_err("oversized bytes")
            .code(),
        "ZRYNA-N3003"
    );
    let target = select_object_target(NATIVE_OBJECT_TARGET).expect("supported target");
    let artifact = emit_object(&module, target).expect("fixture object");
    let mut unexpected_section = artifact.bytes().to_vec();
    let marker = b".note.GNU-stack";
    let offset = unexpected_section
        .windows(marker.len())
        .position(|bytes| bytes == marker)
        .expect("fixture section name");
    unexpected_section[offset + 6] = b'B';
    assert_eq!(
        audit_object(&unexpected_section, &module)
            .expect_err("unexpected section must fail")
            .code(),
        "ZRYNA-N3003"
    );
    let empty = zryna_native_mir::verify(zryna_native_mir::raw::Module::new(Vec::new()))
        .expect("empty MIR");
    assert_eq!(
        audit_object(artifact.bytes(), &empty).expect_err("symbol mismatch must fail").code(),
        "ZRYNA-N3003"
    );
}

#[test]
fn codegen_failure_mapping_is_stable() {
    assert_eq!(codegen_error("controlled failure").code(), "ZRYNA-N3002");
}

#[test]
fn exact_parameter_limit_emits_for_system_v() {
    let parameter_count = zryna_native_mir::MAX_MIR_PARAMETERS_PER_FUNCTION;
    let parameters = vec![MirType::I32; parameter_count];
    let values = (0..parameter_count)
        .map(|index| {
            zryna_native_mir::raw::ValueDefinition::new(
                zryna_native_mir::raw::ValueId::new(u32::try_from(index).expect("parameter index")),
                MirType::I32,
                zryna_native_mir::raw::Operation::Parameter {
                    index: u32::try_from(index).expect("parameter operation index"),
                },
            )
        })
        .collect();
    let function = zryna_native_mir::raw::Function::new(
        "maximum".to_owned(),
        zryna_native_mir::raw::CallingConvention::ZRYNA_INTERNAL_I32_V1,
        zryna_native_mir::raw::Signature::new(parameters, MirType::I32),
        values,
        zryna_native_mir::raw::ValueId::new(
            u32::try_from(parameter_count - 1).expect("result index"),
        ),
    );
    let module = zryna_native_mir::verify(zryna_native_mir::raw::Module::new(vec![function]))
        .expect("maximum supported MIR must verify");
    let target = select_object_target(NATIVE_OBJECT_TARGET).expect("supported target");
    assert!(emit_object(&module, target).is_ok());
}
