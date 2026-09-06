//! Audited Linux x86-64 object emission for verified DataOwnershipV1 MIR.

mod clone;
mod control;
mod drop;
mod lower;
mod runtime;
mod state;
mod storage;

use std::collections::{BTreeMap, BTreeSet};

use cranelift_codegen::{
    Context,
    ir::{AbiParam, Function, Signature, UserFuncName, types},
    isa::CallConv,
    settings::{self, Configurable},
};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use object::{
    BinaryFormat, Endianness, Object, ObjectKind, ObjectSection, ObjectSymbol, RelocationTarget,
};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{VerifiedFunction, VerifiedMirModule};

use crate::{LinuxX8664ObjectTarget, MAX_NATIVE_OBJECT_BYTES, NATIVE_OBJECT_TARGET};

/// Audited DataOwnershipV1 ELF relocatable bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedDataOwnershipObjectArtifact {
    bytes: Vec<u8>,
}

impl ValidatedDataOwnershipObjectArtifact {
    /// Returns exact bytes accepted by the closed object audit.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Emits deterministic DataOwnershipV1 Linux x86-64 object bytes.
///
/// # Errors
/// Returns a stable diagnostic when code generation or the closed object audit fails.
pub fn emit_object(
    program: &VerifiedMirModule,
    _target: LinuxX8664ObjectTarget,
) -> Result<ValidatedDataOwnershipObjectArtifact, Diagnostic> {
    let mut flags = settings::builder();
    flags.set("opt_level", "none").map_err(codegen_error)?;
    flags.set("is_pic", "false").map_err(codegen_error)?;
    flags.set("unwind_info", "false").map_err(codegen_error)?;
    let triple = NATIVE_OBJECT_TARGET.parse::<target_lexicon::Triple>().map_err(codegen_error)?;
    let isa = cranelift_codegen::isa::lookup(triple)
        .map_err(codegen_error)?
        .finish(settings::Flags::new(flags))
        .map_err(codegen_error)?;
    let mut builder = ObjectBuilder::new(isa, b"zryna-m3".to_vec(), default_libcall_names())
        .map_err(codegen_error)?;
    builder.per_function_section(false);
    let mut object = ObjectModule::new(builder);
    let functions = program.functions().collect::<Vec<_>>();
    let mut function_ids = BTreeMap::new();
    for function in &functions {
        let id = object
            .declare_function(function.symbol(), Linkage::Export, &signature(program, *function)?)
            .map_err(codegen_error)?;
        function_ids.insert(function.identity(), id);
    }
    let runtime_ids = declare_runtime(program, &mut object)?;
    let clone_ids = declare_helpers(program, &mut object, "clone", 2)?;
    let drop_ids = declare_helpers(program, &mut object, "drop", 1)?;

    define_helpers(program, &mut object, &runtime_ids, &clone_ids, &drop_ids)?;

    let mut builder_context = FunctionBuilderContext::new();
    for (index, function) in functions.iter().copied().enumerate() {
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(3, u32::try_from(index).map_err(codegen_error)?),
            signature(program, function)?,
        ));
        let callees = function_ids
            .iter()
            .map(|(key, id)| (*key, object.declare_func_in_func(*id, &mut context.func)))
            .collect();
        let runtime = runtime_ids
            .iter()
            .map(|(symbol, id)| {
                (symbol.as_str(), object.declare_func_in_func(*id, &mut context.func))
            })
            .collect();
        let clones = clone_ids
            .iter()
            .map(|(ty, id)| (*ty, object.declare_func_in_func(*id, &mut context.func)))
            .collect();
        let drops = drop_ids
            .iter()
            .map(|(ty, id)| (*ty, object.declare_func_in_func(*id, &mut context.func)))
            .collect();
        lower::build_function(
            program,
            function,
            &mut context,
            &mut builder_context,
            &callees,
            &runtime,
            &clones,
            &drops,
            object.target_config(),
        )?;
        let id = *function_ids.get(&function.identity()).ok_or_else(invariant_error)?;
        object.define_function(id, &mut context).map_err(codegen_error)?;
    }
    let bytes = object.finish().emit().map_err(codegen_error)?;
    audit_object(&bytes, program)?;
    Ok(ValidatedDataOwnershipObjectArtifact { bytes })
}

fn declare_helpers(
    program: &VerifiedMirModule,
    object: &mut ObjectModule,
    kind: &str,
    parameters: usize,
) -> Result<BTreeMap<u32, FuncId>, Diagnostic> {
    program
        .types()
        .map(|ty| {
            let mut signature = Signature::new(CallConv::SystemV);
            signature.params.extend((0..parameters).map(|_| AbiParam::new(types::I64)));
            signature.returns.push(AbiParam::new(types::I32));
            let symbol = format!("zryna_m3_{kind}_t{}", ty.id());
            let id = object
                .declare_function(&symbol, Linkage::Local, &signature)
                .map_err(codegen_error)?;
            Ok((ty.id(), id))
        })
        .collect()
}

fn define_helpers(
    program: &VerifiedMirModule,
    object: &mut ObjectModule,
    runtime_ids: &BTreeMap<String, FuncId>,
    clone_ids: &BTreeMap<u32, FuncId>,
    drop_ids: &BTreeMap<u32, FuncId>,
) -> Result<(), Diagnostic> {
    let mut builder_context = FunctionBuilderContext::new();
    for ty in program.types() {
        let clone_id = *clone_ids.get(&ty.id()).ok_or_else(invariant_error)?;
        let mut clone_context = object.make_context();
        let runtime = runtime_ids
            .iter()
            .map(|(symbol, id)| {
                (symbol.as_str(), object.declare_func_in_func(*id, &mut clone_context.func))
            })
            .collect();
        let clones = clone_ids
            .iter()
            .map(|(id, function)| {
                (*id, object.declare_func_in_func(*function, &mut clone_context.func))
            })
            .collect();
        clone::build_helper(
            program,
            ty.id(),
            &mut clone_context,
            &mut builder_context,
            &runtime,
            &clones,
            object.target_config(),
        )?;
        object.define_function(clone_id, &mut clone_context).map_err(codegen_error)?;

        let drop_id = *drop_ids.get(&ty.id()).ok_or_else(invariant_error)?;
        let mut drop_context = object.make_context();
        let runtime = runtime_ids
            .iter()
            .map(|(symbol, id)| {
                (symbol.as_str(), object.declare_func_in_func(*id, &mut drop_context.func))
            })
            .collect();
        let drops = drop_ids
            .iter()
            .map(|(id, function)| {
                (*id, object.declare_func_in_func(*function, &mut drop_context.func))
            })
            .collect();
        drop::build_helper(
            program,
            ty.id(),
            &mut drop_context,
            &mut builder_context,
            &runtime,
            &drops,
            object.target_config(),
        )?;
        object.define_function(drop_id, &mut drop_context).map_err(codegen_error)?;
    }
    Ok(())
}

fn signature(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
) -> Result<Signature, Diagnostic> {
    let mut signature = Signature::new(CallConv::SystemV);
    for parameter in function.parameters() {
        signature.params.push(AbiParam::new(lower::native_type(program, parameter.ty())?));
    }
    for _ in function.borrow_parameters() {
        signature.params.push(AbiParam::new(types::I64));
    }
    signature.returns.push(AbiParam::new(lower::native_type(program, function.result_type())?));
    Ok(signature)
}

fn declare_runtime(
    program: &VerifiedMirModule,
    object: &mut ObjectModule,
) -> Result<BTreeMap<String, FuncId>, Diagnostic> {
    program
        .runtime_symbols()
        .map(|symbol| {
            let id = object
                .declare_function(symbol, Linkage::Import, &runtime_signature(symbol)?)
                .map_err(codegen_error)?;
            Ok((symbol.to_owned(), id))
        })
        .collect()
}

fn runtime_signature(symbol: &str) -> Result<Signature, Diagnostic> {
    let mut signature = Signature::new(CallConv::SystemV);
    let i32s = |signature: &mut Signature, parameters: &[cranelift_codegen::ir::Type]| {
        signature.params.extend(parameters.iter().copied().map(AbiParam::new));
        signature.returns.push(AbiParam::new(types::I32));
    };
    let p = types::I64;
    match symbol.strip_prefix("zryna_rt_o1_").ok_or_else(invariant_error)? {
        "allocate" => i32s(&mut signature, &[types::I64, types::I32, p]),
        "grow" => i32s(&mut signature, &[p, types::I64, types::I64, types::I32, p]),
        "release" => i32s(&mut signature, &[p, types::I64, types::I32]),
        "string_from_utf8_copy" => i32s(&mut signature, &[p, types::I64, p]),
        "string_clone" | "string_concat" => {
            let parameters = if symbol.ends_with("concat") { vec![p, p, p] } else { vec![p, p] };
            i32s(&mut signature, &parameters);
        }
        "string_release" => i32s(&mut signature, &[p]),
        "vec_allocate" => i32s(&mut signature, &[types::I32, types::I64, p]),
        "vec_reserve" => i32s(&mut signature, &[types::I32, p, types::I64, p]),
        "vec_release_storage" => i32s(&mut signature, &[types::I32, p]),
        "strong_clone"
        | "weak_downgrade"
        | "weak_clone"
        | "weak_upgrade"
        | "strong_release_finish" => i32s(&mut signature, &[p]),
        "strong_release_begin" | "weak_release" => i32s(&mut signature, &[p, p]),
        _ => return Err(invariant_error()),
    }
    Ok(signature)
}

fn audit_object(bytes: &[u8], program: &VerifiedMirModule) -> Result<(), Diagnostic> {
    if bytes.len() > MAX_NATIVE_OBJECT_BYTES {
        return Err(audit_error());
    }
    let file = object::File::parse(bytes).map_err(|_| audit_error())?;
    if file.format() != BinaryFormat::Elf
        || file.architecture() != object::Architecture::X86_64
        || file.endianness() != Endianness::Little
        || file.kind() != ObjectKind::Relocatable
        || !file.is_64()
    {
        return Err(audit_error());
    }
    let approved_runtime = program.runtime_symbols().collect::<BTreeSet<_>>();
    let expected_functions = program.functions().map(|function| function.symbol()).collect();
    let mut defined = BTreeSet::new();
    let mut all_defined = BTreeSet::new();
    let mut undefined = BTreeSet::new();
    for symbol in file.symbols() {
        if symbol.is_undefined() {
            let name = symbol.name().map_err(|_| audit_error())?;
            if !approved_runtime.contains(name) {
                return Err(audit_error());
            }
            undefined.insert(name);
        } else if symbol.kind() == object::SymbolKind::Text {
            if symbol.size() == 0 {
                return Err(audit_error());
            }
            let name = symbol.name().map_err(|_| audit_error())?;
            all_defined.insert(name);
            if symbol.is_global() {
                defined.insert(name);
            }
        }
    }
    if defined != expected_functions {
        return Err(audit_error());
    }
    for section in file.sections() {
        for (_, relocation) in section.relocations() {
            let RelocationTarget::Symbol(index) = relocation.target() else {
                return Err(audit_error());
            };
            let target = file.symbol_by_index(index).map_err(|_| audit_error())?;
            let name = target.name().map_err(|_| audit_error())?;
            if !all_defined.contains(name) && !undefined.contains(name) {
                return Err(audit_error());
            }
        }
    }
    Ok(())
}

pub(super) fn invariant_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N3301",
        None,
        "DataOwnershipV1 native code generation rejected a verified MIR invariant",
        "report this compiler invariant failure with the smallest reproducible source",
    )
}

fn codegen_error(error: impl std::fmt::Display) -> Diagnostic {
    let _ = error;
    Diagnostic::error(
        "ZRYNA-N3302",
        None,
        "DataOwnershipV1 native object code generation failed",
        "report this compiler failure with the smallest reproducible source",
    )
}

fn audit_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N3303",
        None,
        "DataOwnershipV1 object failed the closed Linux x86-64 ELF audit",
        "report this compiler failure with the smallest reproducible source",
    )
}
