//! Native code-generation boundary.

#![forbid(unsafe_code)]

use std::fmt::Write;

use cranelift_codegen::{
    Context,
    ir::{AbiParam, Function, InstBuilder, Signature, UserFuncName, types},
    isa::CallConv,
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use object::{
    BinaryFormat, Endianness, Object, ObjectKind, ObjectSection, ObjectSymbol, SectionFlags,
    SectionKind,
};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::{
    MirType, OperationView, ValueId, VerifiedCallingConvention, VerifiedMirFunction,
    VerifiedMirModule,
};

/// Internal M2 scalar control-flow object emission.
pub mod control_flow_v1;

/// The only native object target implemented by the M1 scalar profile.
pub const NATIVE_OBJECT_TARGET: &str = "x86_64-unknown-linux-gnu";
/// Maximum encoded object bytes accepted by the native object audit.
pub const MAX_NATIVE_OBJECT_BYTES: usize = 8 * 1024 * 1024;
const EMPTY_OBJECT_SECTIONS: [(&str, SectionKind, u64); 4] = [
    (".note.GNU-stack", SectionKind::Other, 0),
    (".symtab", SectionKind::Metadata, 0),
    (".strtab", SectionKind::Metadata, 0),
    (".shstrtab", SectionKind::Metadata, 0),
];
const FUNCTION_OBJECT_SECTIONS: [(&str, SectionKind, u64); 5] = [
    (".text", SectionKind::Text, 6),
    (".note.GNU-stack", SectionKind::Other, 0),
    (".symtab", SectionKind::Metadata, 0),
    (".strtab", SectionKind::Metadata, 0),
    (".shstrtab", SectionKind::Metadata, 0),
];

/// Capability proving that an object request selected the exact supported target.
///
/// ```compile_fail
/// let _ = zryna_backend_native::LinuxX8664ObjectTarget { private: () };
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxX8664ObjectTarget {
    private: (),
}

/// Selects the exact supported native object target.
///
/// # Errors
///
/// Returns `ZRYNA-N3001` for every empty, malformed, aliased, or unsupported request.
pub fn select_object_target(requested: &str) -> Result<LinuxX8664ObjectTarget, Diagnostic> {
    if requested == NATIVE_OBJECT_TARGET {
        Ok(LinuxX8664ObjectTarget { private: () })
    } else {
        Err(Diagnostic::error(
            "ZRYNA-N3001",
            None,
            "native object target is unsupported",
            format!("use the exact supported target '{NATIVE_OBJECT_TARGET}'"),
        ))
    }
}

/// ELF object bytes that passed the closed native-object audit.
///
/// ```compile_fail
/// let _ = zryna_backend_native::ValidatedNativeObjectArtifact { bytes: Vec::new() };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedNativeObjectArtifact {
    bytes: Vec<u8>,
}

impl ValidatedNativeObjectArtifact {
    /// Returns the independently audited ELF relocatable bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Emits one deterministic audited Linux x86-64 ELF relocatable object.
///
/// Only verified native MIR and an exact target capability can enter this boundary. The backend
/// never invokes a system compiler, assembler, linker, loader, or generated executable.
///
/// # Errors
///
/// Returns stable target, code-generation, or post-encode audit diagnostics.
pub fn emit_object(
    module: &VerifiedMirModule,
    _target: LinuxX8664ObjectTarget,
) -> Result<ValidatedNativeObjectArtifact, Diagnostic> {
    let mut flags = settings::builder();
    flags.set("opt_level", "none").map_err(codegen_error)?;
    flags.set("is_pic", "false").map_err(codegen_error)?;
    let triple = NATIVE_OBJECT_TARGET.parse::<target_lexicon::Triple>().map_err(codegen_error)?;
    let isa = cranelift_codegen::isa::lookup(triple)
        .map_err(codegen_error)?
        .finish(settings::Flags::new(flags))
        .map_err(codegen_error)?;
    let mut object_builder = ObjectBuilder::new(isa, b"zryna".to_vec(), default_libcall_names())
        .map_err(codegen_error)?;
    object_builder.per_function_section(false);
    let mut object_module = ObjectModule::new(object_builder);
    let mut function_builder_context = FunctionBuilderContext::new();

    for (function_index, function) in module.functions().enumerate() {
        let frontend_config = object_module.target_config();
        let signature = signature(function)?;
        let function_id = object_module
            .declare_function(function.symbol(), Linkage::Export, &signature)
            .map_err(codegen_error)?;
        let user_index = u32::try_from(function_index).map_err(codegen_error)?;
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(0, user_index),
            signature,
        ));
        build_function(function, &mut context, &mut function_builder_context, frontend_config)?;
        object_module.define_function(function_id, &mut context).map_err(codegen_error)?;
    }

    let bytes = object_module.finish().emit().map_err(codegen_error)?;
    audit_object(&bytes, module)?;
    Ok(ValidatedNativeObjectArtifact { bytes })
}

fn signature(function: VerifiedMirFunction<'_>) -> Result<Signature, Diagnostic> {
    match function.calling_convention() {
        VerifiedCallingConvention::ScalarAbiV1LinuxX8664SystemV => {}
    }
    verify_codegen_type(function.result_type())?;
    let mut signature = Signature::new(CallConv::SystemV);
    for ty in function.parameter_types() {
        verify_codegen_type(*ty)?;
        signature.params.push(AbiParam::new(types::I32));
    }
    signature.returns.push(AbiParam::new(types::I32));
    Ok(signature)
}

fn build_function(
    function: VerifiedMirFunction<'_>,
    context: &mut Context,
    builder_context: &mut FunctionBuilderContext,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
) -> Result<(), Diagnostic> {
    let mut builder = FunctionBuilder::new(&mut context.func, builder_context);
    let entry = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    builder.seal_block(entry);
    let parameters = builder.block_params(entry).to_vec();
    let mut values = Vec::with_capacity(function.values().len());
    for value in function.values() {
        verify_codegen_type(value.ty())?;
        let encoded = match value.operation() {
            OperationView::Parameter { index } => parameters
                .get(usize::try_from(index).map_err(codegen_error)?)
                .copied()
                .ok_or_else(native_invariant_error)?,
            OperationView::I32Literal { value } => {
                builder.ins().iconst(types::I32, i64::from(value))
            }
            OperationView::I32Add { lhs, rhs } => {
                let lhs = encoded_value(&values, lhs)?;
                let rhs = encoded_value(&values, rhs)?;
                builder.ins().iadd(lhs, rhs)
            }
        };
        values.push(encoded);
    }
    let result = encoded_value(&values, function.result())?;
    builder.ins().return_(&[result]);
    builder.finalize(frontend_config);
    Ok(())
}

fn encoded_value(
    values: &[cranelift_codegen::ir::Value],
    id: ValueId,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    values
        .get(usize::try_from(id.index()).map_err(codegen_error)?)
        .copied()
        .ok_or_else(native_invariant_error)
}

fn audit_object(bytes: &[u8], module: &VerifiedMirModule) -> Result<(), Diagnostic> {
    if bytes.len() > MAX_NATIVE_OBJECT_BYTES {
        return Err(object_audit_error());
    }
    let file = object::File::parse(bytes).map_err(|_| object_audit_error())?;
    if file.format() != BinaryFormat::Elf
        || file.architecture() != object::Architecture::X86_64
        || file.endianness() != Endianness::Little
        || file.kind() != ObjectKind::Relocatable
        || !file.is_64()
    {
        return Err(object_audit_error());
    }
    let expected_sections = if module.functions().len() == 0 {
        &EMPTY_OBJECT_SECTIONS[..]
    } else {
        &FUNCTION_OBJECT_SECTIONS[..]
    };
    let sections = file.sections().collect::<Vec<_>>();
    if sections.len() != expected_sections.len() {
        return Err(object_audit_error());
    }
    for (section, (expected_name, expected_kind, expected_flags)) in
        sections.into_iter().zip(expected_sections.iter().copied())
    {
        let SectionFlags::Elf { sh_flags } = section.flags() else {
            return Err(object_audit_error());
        };
        if section.name().map_err(|_| object_audit_error())? != expected_name
            || section.kind() != expected_kind
            || sh_flags != expected_flags
            || section.relocations().next().is_some()
        {
            return Err(object_audit_error());
        }
    }
    let expected = module.functions().map(VerifiedMirFunction::symbol).collect::<Vec<_>>();
    let mut observed = Vec::new();
    for symbol in file.symbols() {
        if symbol.is_undefined() {
            return Err(object_audit_error());
        }
        if symbol.is_global() {
            if symbol.kind() != object::SymbolKind::Text {
                return Err(object_audit_error());
            }
            let name = symbol.name().map_err(|_| object_audit_error())?;
            if symbol.size() == 0 {
                return Err(object_audit_error());
            }
            observed.push(name);
        }
    }
    if observed != expected {
        return Err(object_audit_error());
    }
    Ok(())
}

fn native_invariant_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N3002",
        None,
        "native object code generation rejected a verified MIR invariant",
        "report this compiler invariant failure with the smallest reproducible source",
    )
}

fn codegen_error(error: impl std::fmt::Display) -> Diagnostic {
    let _ = error;
    Diagnostic::error(
        "ZRYNA-N3002",
        None,
        "native object code generation failed",
        "report this compiler failure with the smallest reproducible source",
    )
}

fn object_audit_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N3003",
        None,
        "native object failed the closed Linux x86-64 ELF audit",
        "report this compiler failure with the smallest reproducible source",
    )
}

/// Textual LLVM IR artifact used to validate the initial native boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlvmIrArtifact {
    /// LLVM IR module text.
    pub source: String,
}

/// Emits the supported verified MIR slice as LLVM IR text.
///
/// This compatibility proof is independent of the implemented Cranelift object path.
/// Raw MIR is not accepted by this boundary:
///
/// ```compile_fail
/// let raw = zryna_native_mir::raw::Module::new(Vec::new());
/// let _ = zryna_backend_native::emit_llvm_ir(&raw);
/// ```
///
/// # Errors
///
/// Returns a compiler diagnostic when an internal verified-MIR invariant or formatting fails.
pub fn emit_llvm_ir(module: &VerifiedMirModule) -> Result<LlvmIrArtifact, Diagnostic> {
    let mut output = String::new();
    for function in module.functions() {
        emit_function(function, &mut output)?;
    }
    Ok(LlvmIrArtifact { source: output })
}

fn emit_function(function: VerifiedMirFunction<'_>, output: &mut String) -> Result<(), Diagnostic> {
    match function.calling_convention() {
        VerifiedCallingConvention::ScalarAbiV1LinuxX8664SystemV => {}
    }
    verify_codegen_type(function.result_type())?;
    for ty in function.parameter_types() {
        verify_codegen_type(*ty)?;
    }
    write!(output, "define i32 @{}(", function.symbol()).map_err(native_format_error)?;
    for index in 0..function.parameter_types().len() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "i32 %p{index}").map_err(native_format_error)?;
    }
    output.push_str(") {\nentry:\n");
    for value in function.values() {
        verify_codegen_type(value.ty())?;
        let id = value.id().index();
        match value.operation() {
            OperationView::Parameter { .. } => {}
            OperationView::I32Literal { value } => {
                writeln!(output, "  %v{id} = add i32 0, {value}").map_err(native_format_error)?;
            }
            OperationView::I32Add { lhs, rhs } => {
                let left = llvm_value(function, lhs)?;
                let right = llvm_value(function, rhs)?;
                writeln!(output, "  %v{id} = add i32 {left}, {right}")
                    .map_err(native_format_error)?;
            }
        }
    }
    let result = llvm_value(function, function.result())?;
    write!(output, "  ret i32 {result}\n}}\n").map_err(native_format_error)?;
    Ok(())
}

fn llvm_value(function: VerifiedMirFunction<'_>, id: ValueId) -> Result<String, Diagnostic> {
    let value = function.value(id).ok_or_else(|| {
        Diagnostic::error(
            "ZRYNA-N2002",
            None,
            format!("verified native function '{}' references a missing value", function.symbol()),
            "report this compiler invariant failure with the smallest reproducible source",
        )
    })?;
    match value.operation() {
        OperationView::Parameter { index } => Ok(format!("%p{index}")),
        OperationView::I32Literal { .. } | OperationView::I32Add { .. } => {
            Ok(format!("%v{}", id.index()))
        }
    }
}

fn verify_codegen_type(ty: MirType) -> Result<(), Diagnostic> {
    match ty {
        MirType::I32 => Ok(()),
        MirType::Unit | MirType::Bool => Err(Diagnostic::error(
            "ZRYNA-N2001",
            None,
            "verified native MIR contains a type outside the LLVM proof profile",
            "report this compiler invariant failure with the smallest reproducible source",
        )),
    }
}

fn native_format_error(error: std::fmt::Error) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N2003",
        None,
        format!("native IR formatting failed: {error}"),
        "report this compiler failure with the smallest reproducible Zryna source",
    )
}

#[cfg(test)]
mod tests {
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
                    zryna_native_mir::raw::ValueId::new(
                        u32::try_from(index).expect("parameter index"),
                    ),
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
}
