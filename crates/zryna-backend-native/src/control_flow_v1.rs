//! Linux x86-64 object emission for verified M2 scalar control flow.

use std::collections::{BTreeMap, BTreeSet};

use cranelift_codegen::{
    Context,
    ir::{
        AbiParam, Function, InstBuilder, Signature, TrapCode, UserFuncName, condcodes::IntCC, types,
    },
    isa::CallConv,
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use object::{
    BinaryFormat, Endianness, Object, ObjectKind, ObjectSection, ObjectSymbol, RelocationEncoding,
    RelocationFlags, RelocationKind, RelocationTarget, SectionFlags, SectionKind, SymbolFlags,
    SymbolKind, SymbolScope, SymbolSection,
};
use zryna_abi::{Invocation, InvocationError, VerifiedInvocation, VerifiedScalarAbiModule};
use zryna_diagnostics::Diagnostic;
use zryna_ir::Type;
use zryna_native_mir::control_flow_v1::{
    BlockIdentity, FunctionIdentity, ValueIdentity, VerifiedCallingConvention, VerifiedFunction,
    VerifiedInstructionKind, VerifiedProgram, VerifiedTerminatorKind,
};

use crate::{LinuxX8664ObjectTarget, MAX_NATIVE_OBJECT_BYTES, NATIVE_OBJECT_TARGET};

/// M2 object bytes bound to the exact scalar-ABI authority used during emission.
///
/// Callers cannot construct an unaudited artifact:
///
/// ```compile_fail
/// let _ = zryna_backend_native::control_flow_v1::ValidatedControlFlowNativeObjectArtifact {
///     bytes: Vec::new(),
///     abi: todo!(),
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedControlFlowNativeObjectArtifact {
    bytes: Vec<u8>,
    abi: VerifiedScalarAbiModule,
}

impl ValidatedControlFlowNativeObjectArtifact {
    /// Returns the independently audited ELF relocatable bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the scalar ABI sealed into this exact object artifact.
    #[must_use]
    pub const fn scalar_abi(&self) -> &VerifiedScalarAbiModule {
        &self.abi
    }

    /// Validates one typed invocation against this exact artifact.
    ///
    /// # Errors
    ///
    /// Rejects an unknown export, wrong arity, or mismatched typed argument.
    pub fn prepare_invocation(
        &self,
        invocation: Invocation,
    ) -> Result<VerifiedInvocation<'_>, InvocationError> {
        self.abi.prepare_invocation(invocation)
    }
}

/// Emits one deterministic M2 Linux x86-64 ELF relocatable object.
///
/// The public M1 entrypoint remains separate. This internal profile accepts only independently
/// verified M2 native MIR and emits local sealed body symbols plus scalar-ABI export wrappers.
///
/// # Errors
///
/// Returns stable code-generation or closed-object-audit diagnostics.
pub fn emit_object(
    program: &VerifiedProgram,
    _target: LinuxX8664ObjectTarget,
) -> Result<ValidatedControlFlowNativeObjectArtifact, Diagnostic> {
    let mut flags = settings::builder();
    flags.set("opt_level", "none").map_err(codegen_error)?;
    flags.set("is_pic", "false").map_err(codegen_error)?;
    flags.set("unwind_info", "false").map_err(codegen_error)?;
    let triple = NATIVE_OBJECT_TARGET.parse::<target_lexicon::Triple>().map_err(codegen_error)?;
    let isa = cranelift_codegen::isa::lookup(triple)
        .map_err(codegen_error)?
        .finish(settings::Flags::new(flags))
        .map_err(codegen_error)?;
    let mut object_builder = ObjectBuilder::new(isa, b"zryna".to_vec(), default_libcall_names())
        .map_err(codegen_error)?;
    object_builder.per_function_section(false);
    let mut object_module = ObjectModule::new(object_builder);

    let functions = program
        .modules()
        .flat_map(zryna_native_mir::control_flow_v1::VerifiedModule::functions)
        .collect::<Vec<_>>();
    let mut body_ids = BTreeMap::new();
    let mut export_ids = Vec::new();
    for function in &functions {
        let signature = body_signature(*function)?;
        let id = object_module
            .declare_function(function.internal_symbol(), Linkage::Local, &signature)
            .map_err(codegen_error)?;
        body_ids.insert(function_key(function.id()), id);
        if let Some(export) = function.public_export() {
            let wrapper_signature = wrapper_signature(*function)?;
            let wrapper = object_module
                .declare_function(
                    export.native_linux_x86_64_symbol().as_str(),
                    Linkage::Export,
                    &wrapper_signature,
                )
                .map_err(codegen_error)?;
            export_ids.push((*function, wrapper, id));
        }
    }

    let mut builder_context = FunctionBuilderContext::new();
    let mut user_index = 0_u32;
    for function in &functions {
        let signature = body_signature(*function)?;
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(0, user_index),
            signature,
        ));
        user_index = user_index.checked_add(1).ok_or_else(codegen_error_unit)?;
        let callee_keys = direct_callee_keys(*function)?;
        let callees = callee_keys
            .into_iter()
            .map(|key| {
                let id = *body_ids.get(&key).ok_or_else(native_invariant_error)?;
                Ok((key, object_module.declare_func_in_func(id, &mut context.func)))
            })
            .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?;
        let frontend_config = object_module.target_config();
        build_body(*function, &mut context, &mut builder_context, &callees, frontend_config)?;
        let id = *body_ids.get(&function_key(function.id())).ok_or_else(native_invariant_error)?;
        object_module.define_function(id, &mut context).map_err(codegen_error)?;
    }

    for (function, wrapper_id, body_id) in export_ids {
        let signature = wrapper_signature(function)?;
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(0, user_index),
            signature,
        ));
        user_index = user_index.checked_add(1).ok_or_else(codegen_error_unit)?;
        let body_ref = object_module.declare_func_in_func(body_id, &mut context.func);
        build_wrapper(
            function,
            &mut context,
            &mut builder_context,
            body_ref,
            object_module.target_config(),
        )?;
        object_module.define_function(wrapper_id, &mut context).map_err(codegen_error)?;
    }

    let bytes = object_module.finish().emit().map_err(codegen_error)?;
    audit_object(&bytes, program)?;
    Ok(ValidatedControlFlowNativeObjectArtifact { bytes, abi: program.scalar_abi().clone() })
}

fn body_signature(function: VerifiedFunction<'_>) -> Result<Signature, Diagnostic> {
    match function.calling_convention() {
        VerifiedCallingConvention::ControlFlowV1 => {}
    }
    let mut signature = Signature::new(CallConv::SystemV);
    for (_, ty) in function.parameters() {
        signature.params.push(AbiParam::new(native_type(ty)?));
    }
    signature.returns.push(AbiParam::new(native_type(function.result())?));
    Ok(signature)
}

fn wrapper_signature(function: VerifiedFunction<'_>) -> Result<Signature, Diagnostic> {
    let mut signature = Signature::new(CallConv::SystemV);
    for (_, ty) in function.parameters() {
        verify_scalar_type(ty)?;
        signature.params.push(AbiParam::new(types::I32));
    }
    verify_scalar_type(function.result())?;
    signature.returns.push(AbiParam::new(types::I32));
    Ok(signature)
}

#[allow(clippy::too_many_lines, reason = "exhaustive M2 MIR lowering remains one closed match")]
fn build_body(
    function: VerifiedFunction<'_>,
    context: &mut Context,
    builder_context: &mut FunctionBuilderContext,
    callees: &BTreeMap<(u32, u32), cranelift_codegen::ir::FuncRef>,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
) -> Result<(), Diagnostic> {
    let mut builder = FunctionBuilder::new(&mut context.func, builder_context);
    let blocks = function.blocks().collect::<Vec<_>>();
    let encoded_blocks = blocks.iter().map(|_| builder.create_block()).collect::<Vec<_>>();
    let mut values = vec![None; value_capacity(function)?];

    let entry = *encoded_blocks.first().ok_or_else(native_invariant_error)?;
    builder.append_block_params_for_function_params(entry);
    for ((id, ty), value) in function.parameters().zip(builder.block_params(entry).to_vec()) {
        verify_scalar_type(ty)?;
        store_value(&mut values, id, value)?;
    }
    for (block, encoded) in blocks.iter().zip(&encoded_blocks) {
        for (id, ty) in block.parameters() {
            verify_scalar_type(ty)?;
            builder.append_block_param(*encoded, native_type(ty)?);
            let value =
                *builder.block_params(*encoded).last().ok_or_else(native_invariant_error)?;
            store_value(&mut values, id, value)?;
        }
    }

    for block_index in reverse_postorder(&blocks)? {
        let block = blocks[block_index];
        let encoded = encoded_blocks[block_index];
        builder.switch_to_block(encoded);
        for instruction in block.instructions() {
            verify_scalar_type(instruction.ty())?;
            let value = match instruction.kind() {
                VerifiedInstructionKind::BoolLiteral(value) => {
                    builder.ins().iconst(types::I8, i64::from(value))
                }
                VerifiedInstructionKind::I32Literal(value) => {
                    builder.ins().iconst(types::I32, i64::from(value))
                }
                VerifiedInstructionKind::I32Add(lhs, rhs) => {
                    builder.ins().iadd(load_value(&values, lhs)?, load_value(&values, rhs)?)
                }
                VerifiedInstructionKind::I32Sub(lhs, rhs) => {
                    builder.ins().isub(load_value(&values, lhs)?, load_value(&values, rhs)?)
                }
                VerifiedInstructionKind::I32Mul(lhs, rhs) => {
                    builder.ins().imul(load_value(&values, lhs)?, load_value(&values, rhs)?)
                }
                VerifiedInstructionKind::I32Neg(operand) => {
                    builder.ins().ineg(load_value(&values, operand)?)
                }
                VerifiedInstructionKind::Eq(lhs, rhs) => compare(
                    &mut builder,
                    IntCC::Equal,
                    load_value(&values, lhs)?,
                    load_value(&values, rhs)?,
                ),
                VerifiedInstructionKind::Ne(lhs, rhs) => compare(
                    &mut builder,
                    IntCC::NotEqual,
                    load_value(&values, lhs)?,
                    load_value(&values, rhs)?,
                ),
                VerifiedInstructionKind::I32LtS(lhs, rhs) => compare(
                    &mut builder,
                    IntCC::SignedLessThan,
                    load_value(&values, lhs)?,
                    load_value(&values, rhs)?,
                ),
                VerifiedInstructionKind::I32LeS(lhs, rhs) => compare(
                    &mut builder,
                    IntCC::SignedLessThanOrEqual,
                    load_value(&values, lhs)?,
                    load_value(&values, rhs)?,
                ),
                VerifiedInstructionKind::I32GtS(lhs, rhs) => compare(
                    &mut builder,
                    IntCC::SignedGreaterThan,
                    load_value(&values, lhs)?,
                    load_value(&values, rhs)?,
                ),
                VerifiedInstructionKind::I32GeS(lhs, rhs) => compare(
                    &mut builder,
                    IntCC::SignedGreaterThanOrEqual,
                    load_value(&values, lhs)?,
                    load_value(&values, rhs)?,
                ),
                VerifiedInstructionKind::DirectCall { callee, arguments } => {
                    let callee =
                        *callees.get(&function_key(callee)).ok_or_else(native_invariant_error)?;
                    let arguments = arguments
                        .iter()
                        .map(|id| load_value(&values, id))
                        .collect::<Result<Vec<_>, _>>()?;
                    let call = builder.ins().call(callee, &arguments);
                    *builder.inst_results(call).first().ok_or_else(native_invariant_error)?
                }
            };
            store_value(&mut values, instruction.result(), value)?;
        }
        match block.terminator().kind() {
            VerifiedTerminatorKind::Return(value) => {
                builder.ins().return_(&[load_value(&values, value)?]);
            }
            VerifiedTerminatorKind::Jump { target, arguments } => {
                let target = encoded_block(&encoded_blocks, target)?;
                let arguments = edge_arguments(&values, arguments)?;
                builder.ins().jump(target, &arguments);
            }
            VerifiedTerminatorKind::Branch {
                condition,
                true_target,
                true_arguments,
                false_target,
                false_arguments,
            } => {
                let condition =
                    builder.ins().icmp_imm_u(IntCC::Equal, load_value(&values, condition)?, 1);
                let true_target = encoded_block(&encoded_blocks, true_target)?;
                let false_target = encoded_block(&encoded_blocks, false_target)?;
                let true_arguments = edge_arguments(&values, true_arguments)?;
                let false_arguments = edge_arguments(&values, false_arguments)?;
                builder.ins().brif(
                    condition,
                    true_target,
                    &true_arguments,
                    false_target,
                    &false_arguments,
                );
            }
        }
    }
    builder.seal_all_blocks();
    builder.finalize(frontend_config);
    Ok(())
}

fn build_wrapper(
    function: VerifiedFunction<'_>,
    context: &mut Context,
    builder_context: &mut FunctionBuilderContext,
    body: cranelift_codegen::ir::FuncRef,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
) -> Result<(), Diagnostic> {
    let mut builder = FunctionBuilder::new(&mut context.func, builder_context);
    let entry = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    builder.seal_block(entry);
    let mut arguments = Vec::new();
    for ((_, ty), value) in function.parameters().zip(builder.block_params(entry).to_vec()) {
        arguments.push(match ty {
            Type::I32 => value,
            Type::Bool => {
                let zero = builder.ins().icmp_imm_u(IntCC::Equal, value, 0);
                let one = builder.ins().icmp_imm_u(IntCC::Equal, value, 1);
                let canonical = builder.ins().bor(zero, one);
                builder.ins().trapz(canonical, TrapCode::unwrap_user(1));
                builder.ins().ireduce(types::I8, value)
            }
            Type::Unit => return Err(native_invariant_error()),
        });
    }
    let call = builder.ins().call(body, &arguments);
    let body_result = *builder.inst_results(call).first().ok_or_else(native_invariant_error)?;
    let result = match function.result() {
        Type::I32 => body_result,
        Type::Bool => builder.ins().uextend(types::I32, body_result),
        Type::Unit => return Err(native_invariant_error()),
    };
    builder.ins().return_(&[result]);
    builder.finalize(frontend_config);
    Ok(())
}

fn compare(
    builder: &mut FunctionBuilder<'_>,
    condition: IntCC,
    lhs: cranelift_codegen::ir::Value,
    rhs: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    builder.ins().icmp(condition, lhs, rhs)
}

fn reverse_postorder(
    blocks: &[zryna_native_mir::control_flow_v1::VerifiedBlock<'_>],
) -> Result<Vec<usize>, Diagnostic> {
    let mut visited = vec![false; blocks.len()];
    let mut postorder = Vec::with_capacity(blocks.len());
    let mut stack = vec![(0_usize, false)];
    while let Some((index, expanded)) = stack.pop() {
        if expanded {
            postorder.push(index);
            continue;
        }
        if *visited.get(index).ok_or_else(native_invariant_error)? {
            continue;
        }
        visited[index] = true;
        stack.push((index, true));
        let successors = match blocks[index].terminator().kind() {
            VerifiedTerminatorKind::Return(_) => Vec::new(),
            VerifiedTerminatorKind::Jump { target, .. } => {
                vec![usize::try_from(target.index()).map_err(codegen_error)?]
            }
            VerifiedTerminatorKind::Branch { true_target, false_target, .. } => vec![
                usize::try_from(true_target.index()).map_err(codegen_error)?,
                usize::try_from(false_target.index()).map_err(codegen_error)?,
            ],
        };
        for successor in successors.into_iter().rev() {
            if !*visited.get(successor).ok_or_else(native_invariant_error)? {
                stack.push((successor, false));
            }
        }
    }
    if visited.iter().any(|visited| !visited) {
        return Err(native_invariant_error());
    }
    postorder.reverse();
    Ok(postorder)
}

fn value_capacity(function: VerifiedFunction<'_>) -> Result<usize, Diagnostic> {
    let maximum = function
        .parameters()
        .map(|(id, _)| id.index())
        .chain(function.blocks().flat_map(|block| {
            block
                .parameters()
                .map(|(id, _)| id.index())
                .chain(block.instructions().map(|instruction| instruction.result().index()))
        }))
        .max()
        .unwrap_or(0);
    usize::try_from(maximum)
        .ok()
        .and_then(|maximum| maximum.checked_add(1))
        .ok_or_else(native_invariant_error)
}

fn store_value(
    values: &mut [Option<cranelift_codegen::ir::Value>],
    id: ValueIdentity,
    value: cranelift_codegen::ir::Value,
) -> Result<(), Diagnostic> {
    let slot = values
        .get_mut(usize::try_from(id.index()).map_err(codegen_error)?)
        .ok_or_else(native_invariant_error)?;
    if slot.replace(value).is_some() {
        return Err(native_invariant_error());
    }
    Ok(())
}

fn load_value(
    values: &[Option<cranelift_codegen::ir::Value>],
    id: ValueIdentity,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    values
        .get(usize::try_from(id.index()).map_err(codegen_error)?)
        .and_then(|value| *value)
        .ok_or_else(native_invariant_error)
}

fn encoded_block(
    blocks: &[cranelift_codegen::ir::Block],
    id: BlockIdentity,
) -> Result<cranelift_codegen::ir::Block, Diagnostic> {
    blocks
        .get(usize::try_from(id.index()).map_err(codegen_error)?)
        .copied()
        .ok_or_else(native_invariant_error)
}

fn edge_arguments(
    values: &[Option<cranelift_codegen::ir::Value>],
    ids: zryna_native_mir::control_flow_v1::VerifiedValueList<'_>,
) -> Result<Vec<cranelift_codegen::ir::BlockArg>, Diagnostic> {
    ids.iter().map(|id| load_value(values, id).map(Into::into)).collect()
}

fn function_key(id: FunctionIdentity) -> (u32, u32) {
    (id.module().index(), id.declaration())
}

fn direct_callee_keys(function: VerifiedFunction<'_>) -> Result<BTreeSet<(u32, u32)>, Diagnostic> {
    let blocks = function.blocks().collect::<Vec<_>>();
    let mut callees = BTreeSet::new();
    for index in reverse_postorder(&blocks)? {
        for instruction in blocks[index].instructions() {
            if let VerifiedInstructionKind::DirectCall { callee, .. } = instruction.kind() {
                callees.insert(function_key(callee));
            }
        }
    }
    Ok(callees)
}

fn verify_scalar_type(ty: Type) -> Result<(), Diagnostic> {
    match ty {
        Type::I32 | Type::Bool => Ok(()),
        Type::Unit => Err(native_invariant_error()),
    }
}

fn native_type(ty: Type) -> Result<cranelift_codegen::ir::Type, Diagnostic> {
    verify_scalar_type(ty)?;
    Ok(match ty {
        Type::I32 => types::I32,
        Type::Bool => types::I8,
        Type::Unit => return Err(native_invariant_error()),
    })
}

#[allow(clippy::too_many_lines, reason = "the closed ELF inventory is audited in one boundary")]
fn audit_object(bytes: &[u8], program: &VerifiedProgram) -> Result<(), Diagnostic> {
    if bytes.len() > MAX_NATIVE_OBJECT_BYTES {
        return Err(object_audit_error());
    }
    let file = object::File::parse(bytes).map_err(|_| object_audit_error())?;
    if file.format() != BinaryFormat::Elf
        || file.architecture() != object::Architecture::X86_64
        || file.endianness() != Endianness::Little
        || file.kind() != ObjectKind::Relocatable
        || !file.is_64()
        || file.dynamic_symbols().next().is_some()
        || file.dynamic_relocations().is_some_and(|mut relocations| relocations.next().is_some())
    {
        return Err(object_audit_error());
    }

    let functions = program
        .modules()
        .flat_map(zryna_native_mir::control_flow_v1::VerifiedModule::functions)
        .collect::<Vec<_>>();
    let function_symbols = functions
        .iter()
        .map(|function| (function_key(function.id()), function.internal_symbol()))
        .collect::<BTreeMap<_, _>>();
    let mut expected_relocations = Vec::new();
    for function in &functions {
        let blocks = function.blocks().collect::<Vec<_>>();
        for index in reverse_postorder(&blocks)? {
            for instruction in blocks[index].instructions() {
                if let VerifiedInstructionKind::DirectCall { callee, .. } = instruction.kind() {
                    let target = function_symbols
                        .get(&function_key(callee))
                        .copied()
                        .ok_or_else(native_invariant_error)?;
                    expected_relocations.push((function.internal_symbol(), target));
                }
            }
        }
    }
    for function in &functions {
        if function.public_export().is_some() {
            expected_relocations.push((
                function
                    .public_export()
                    .ok_or_else(native_invariant_error)?
                    .native_linux_x86_64_symbol()
                    .as_str(),
                function.internal_symbol(),
            ));
        }
    }

    let expected_sections: Vec<(&str, SectionKind, u64)> = if functions.is_empty() {
        vec![
            (".note.GNU-stack", SectionKind::Other, 0),
            (".symtab", SectionKind::Metadata, 0),
            (".strtab", SectionKind::Metadata, 0),
            (".shstrtab", SectionKind::Metadata, 0),
        ]
    } else if expected_relocations.is_empty() {
        vec![
            (".text", SectionKind::Text, 6),
            (".note.GNU-stack", SectionKind::Other, 0),
            (".symtab", SectionKind::Metadata, 0),
            (".strtab", SectionKind::Metadata, 0),
            (".shstrtab", SectionKind::Metadata, 0),
        ]
    } else {
        vec![
            (".text", SectionKind::Text, 6),
            (".rela.text", SectionKind::Metadata, 64),
            (".note.GNU-stack", SectionKind::Other, 0),
            (".symtab", SectionKind::Metadata, 0),
            (".strtab", SectionKind::Metadata, 0),
            (".shstrtab", SectionKind::Metadata, 0),
        ]
    };
    let sections = file.sections().collect::<Vec<_>>();
    if sections.len() != expected_sections.len() {
        return Err(object_audit_error());
    }
    for (section, (expected_name, expected_kind, expected_flags)) in
        sections.iter().zip(&expected_sections)
    {
        let SectionFlags::Elf { sh_flags } = section.flags() else {
            return Err(object_audit_error());
        };
        if section.name().map_err(|_| object_audit_error())? != *expected_name
            || section.kind() != *expected_kind
            || sh_flags != *expected_flags
            || (*expected_name != ".text" && section.relocations().next().is_some())
        {
            return Err(object_audit_error());
        }
    }

    let text = sections.iter().find(|section| section.name().ok() == Some(".text"));
    if functions.is_empty() && text.is_some() {
        return Err(object_audit_error());
    }
    let text_index = text.map(ObjectSection::index);
    let text_size = text.map_or(0, ObjectSection::size);

    let mut expected_symbols =
        Vec::with_capacity(functions.len() + program.scalar_abi().exports().len() + 1);
    expected_symbols.push(("zryna", SymbolKind::File, SymbolScope::Compilation, false));
    expected_symbols.extend(functions.iter().map(|function| {
        (function.internal_symbol(), SymbolKind::Text, SymbolScope::Compilation, true)
    }));
    expected_symbols.extend(program.scalar_abi().exports().map(|export| {
        (export.native_linux_x86_64_symbol().as_str(), SymbolKind::Text, SymbolScope::Dynamic, true)
    }));
    let symbols = file.symbols().collect::<Vec<_>>();
    if symbols.len() != expected_symbols.len() {
        return Err(object_audit_error());
    }
    let mut symbol_by_name = BTreeMap::new();
    let mut previous_end = 0_u64;
    for (symbol, (expected_name, expected_kind, expected_scope, is_text)) in
        symbols.iter().zip(expected_symbols)
    {
        let expected_info = if expected_kind == SymbolKind::File {
            object::elf::STT_FILE
        } else if expected_scope == SymbolScope::Dynamic {
            (object::elf::STB_GLOBAL << 4) | object::elf::STT_FUNC
        } else {
            object::elf::STT_FUNC
        };
        if symbol.name().map_err(|_| object_audit_error())? != expected_name
            || symbol.kind() != expected_kind
            || symbol.scope() != expected_scope
            || symbol.is_undefined()
            || symbol.is_weak()
            || (symbol.is_global() != (expected_scope == SymbolScope::Dynamic))
            || symbol.flags()
                != (SymbolFlags::Elf { st_info: expected_info, st_other: object::elf::STV_DEFAULT })
        {
            return Err(object_audit_error());
        }
        if is_text {
            let end = symbol.address().checked_add(symbol.size()).ok_or_else(object_audit_error)?;
            if symbol.size() == 0
                || symbol.address() < previous_end
                || end > text_size
                || symbol.section()
                    != text_index.map_or(SymbolSection::None, SymbolSection::Section)
            {
                return Err(object_audit_error());
            }
            previous_end = end;
        } else if symbol.address() != 0
            || symbol.size() != 0
            || symbol.section() != SymbolSection::None
        {
            return Err(object_audit_error());
        }
        if symbol_by_name
            .insert(expected_name, (symbol.address(), symbol.size(), symbol.index()))
            .is_some()
        {
            return Err(object_audit_error());
        }
    }

    let text_bytes = text
        .map(|section| section.data().map_err(|_| object_audit_error()))
        .transpose()?
        .unwrap_or_default();
    let observed_relocations =
        text.map(|section| section.relocations().collect::<Vec<_>>()).unwrap_or_default();
    if observed_relocations.len() != expected_relocations.len() {
        return Err(object_audit_error());
    }
    let mut previous_offset = None;
    for ((offset, relocation), (caller_name, target_name)) in
        observed_relocations.into_iter().zip(expected_relocations)
    {
        let caller = symbol_by_name.get(caller_name).ok_or_else(object_audit_error)?;
        let target = symbol_by_name.get(target_name).ok_or_else(object_audit_error)?;
        let caller_end = caller.0.checked_add(caller.1).ok_or_else(object_audit_error)?;
        let displacement_end = offset.checked_add(4).ok_or_else(object_audit_error)?;
        let opcode_offset = offset.checked_sub(1).ok_or_else(object_audit_error)?;
        let opcode = text_bytes
            .get(usize::try_from(opcode_offset).map_err(|_| object_audit_error())?)
            .copied();
        if offset <= caller.0
            || displacement_end > caller_end
            || opcode != Some(0xe8)
            || previous_offset.is_some_and(|previous| offset <= previous)
            || relocation.kind() != RelocationKind::PltRelative
            || relocation.encoding() != RelocationEncoding::X86Branch
            || relocation.size() != 32
            || relocation.addend() != -4
            || relocation.has_implicit_addend()
            || relocation.subtractor().is_some()
            || relocation.flags() != (RelocationFlags::Elf { r_type: object::elf::R_X86_64_PLT32 })
            || relocation.target() != RelocationTarget::Symbol(target.2)
        {
            return Err(object_audit_error());
        }
        previous_offset = Some(offset);
    }
    Ok(())
}

fn native_invariant_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N3102",
        None,
        "M2 native object code generation rejected a verified MIR invariant",
        "report this compiler invariant failure with the smallest reproducible source",
    )
}

fn codegen_error(error: impl std::fmt::Display) -> Diagnostic {
    let _ = error;
    Diagnostic::error(
        "ZRYNA-N3102",
        None,
        "M2 native object code generation failed",
        "report this compiler failure with the smallest reproducible source",
    )
}

fn codegen_error_unit() -> Diagnostic {
    codegen_error("bounded native code-generation identity overflow")
}

fn object_audit_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N3103",
        None,
        "M2 native object failed the closed Linux x86-64 ELF audit",
        "report this compiler failure with the smallest reproducible source",
    )
}

#[cfg(test)]
mod tests {
    use object::{Object, ObjectSection, ObjectSymbol};
    use zryna_native_mir::control_flow_v1::{self, raw};

    use super::*;

    fn assert_closed_audit_rejects(bytes: &[u8], program: &VerifiedProgram, attack: &str) {
        assert_eq!(
            audit_object(bytes, program).expect_err(attack).code(),
            "ZRYNA-N3103",
            "attack must fail closed: {attack}"
        );
    }

    fn elf_section_header(bytes: &[u8], section_index: usize) -> usize {
        let section_table = u64::from_le_bytes(bytes[40..48].try_into().expect("ELF shoff"));
        let entry_size = u16::from_le_bytes(bytes[58..60].try_into().expect("ELF shentsize"));
        usize::try_from(section_table).expect("section table offset")
            + section_index * usize::from(entry_size)
    }

    fn value(id: u32, ty: Type) -> raw::ValueDefinition {
        raw::ValueDefinition { id: raw::ValueId(id), ty }
    }

    fn instruction(id: u32, ty: Type, kind: raw::InstructionKind) -> raw::Instruction {
        raw::Instruction { result: value(id, ty), kind }
    }

    fn call_program() -> VerifiedProgram {
        let helper = raw::Function {
            id: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
            internal_symbol: "zryna_m2_i_m0_f0".to_owned(),
            entry_export: None,
            convention: raw::CallingConvention::ZRYNA_INTERNAL_CONTROL_FLOW_V1,
            parameters: vec![value(0, Type::I32)],
            result: Type::I32,
            blocks: vec![raw::Block {
                id: raw::BlockId(0),
                parameters: Vec::new(),
                instructions: vec![
                    raw::Instruction {
                        result: value(1, Type::I32),
                        kind: raw::InstructionKind::I32Literal(1),
                    },
                    raw::Instruction {
                        result: value(2, Type::I32),
                        kind: raw::InstructionKind::I32Add {
                            lhs: raw::ValueId(0),
                            rhs: raw::ValueId(1),
                        },
                    },
                ],
                terminators: vec![raw::Terminator::Return(raw::ValueId(2))],
            }],
        };
        let exported = raw::Function {
            id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
            internal_symbol: "zryna_m2_i_m0_f1".to_owned(),
            entry_export: Some("run".to_owned()),
            convention: raw::CallingConvention::ZRYNA_INTERNAL_CONTROL_FLOW_V1,
            parameters: vec![value(0, Type::I32)],
            result: Type::I32,
            blocks: vec![raw::Block {
                id: raw::BlockId(0),
                parameters: Vec::new(),
                instructions: vec![raw::Instruction {
                    result: value(1, Type::I32),
                    kind: raw::InstructionKind::DirectCall {
                        callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
                        arguments: vec![raw::ValueId(0)],
                    },
                }],
                terminators: vec![raw::Terminator::Return(raw::ValueId(1))],
            }],
        };
        control_flow_v1::verify(raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions: vec![helper, exported] }],
        })
        .expect("fixture M2 MIR must verify")
    }

    #[allow(clippy::too_many_lines, reason = "one fixture enumerates every M2 operation")]
    fn operations_and_control_flow_program() -> VerifiedProgram {
        let compute = raw::Function {
            id: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
            internal_symbol: "zryna_m2_i_m0_f0".to_owned(),
            entry_export: Some("compute".to_owned()),
            convention: raw::CallingConvention::ZRYNA_INTERNAL_CONTROL_FLOW_V1,
            parameters: vec![value(0, Type::I32), value(1, Type::Bool)],
            result: Type::I32,
            blocks: vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        instruction(2, Type::Bool, raw::InstructionKind::BoolLiteral(true)),
                        instruction(3, Type::I32, raw::InstructionKind::I32Literal(2)),
                        instruction(
                            4,
                            Type::I32,
                            raw::InstructionKind::I32Add {
                                lhs: raw::ValueId(0),
                                rhs: raw::ValueId(3),
                            },
                        ),
                        instruction(
                            5,
                            Type::I32,
                            raw::InstructionKind::I32Sub {
                                lhs: raw::ValueId(4),
                                rhs: raw::ValueId(3),
                            },
                        ),
                        instruction(
                            6,
                            Type::I32,
                            raw::InstructionKind::I32Mul {
                                lhs: raw::ValueId(5),
                                rhs: raw::ValueId(3),
                            },
                        ),
                        instruction(
                            7,
                            Type::I32,
                            raw::InstructionKind::I32Neg { operand: raw::ValueId(6) },
                        ),
                        instruction(
                            8,
                            Type::Bool,
                            raw::InstructionKind::Eq { lhs: raw::ValueId(1), rhs: raw::ValueId(2) },
                        ),
                        instruction(
                            9,
                            Type::Bool,
                            raw::InstructionKind::Ne { lhs: raw::ValueId(1), rhs: raw::ValueId(2) },
                        ),
                        instruction(
                            10,
                            Type::Bool,
                            raw::InstructionKind::I32LtS {
                                lhs: raw::ValueId(7),
                                rhs: raw::ValueId(6),
                            },
                        ),
                        instruction(
                            11,
                            Type::Bool,
                            raw::InstructionKind::I32LeS {
                                lhs: raw::ValueId(7),
                                rhs: raw::ValueId(6),
                            },
                        ),
                        instruction(
                            12,
                            Type::Bool,
                            raw::InstructionKind::I32GtS {
                                lhs: raw::ValueId(6),
                                rhs: raw::ValueId(7),
                            },
                        ),
                        instruction(
                            13,
                            Type::Bool,
                            raw::InstructionKind::I32GeS {
                                lhs: raw::ValueId(6),
                                rhs: raw::ValueId(7),
                            },
                        ),
                    ],
                    terminators: vec![raw::Terminator::Branch {
                        condition: raw::ValueId(1),
                        true_target: raw::BlockId(1),
                        true_arguments: vec![raw::ValueId(6)],
                        false_target: raw::BlockId(2),
                        false_arguments: vec![raw::ValueId(7)],
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: vec![value(14, Type::I32)],
                    instructions: vec![instruction(
                        15,
                        Type::I32,
                        raw::InstructionKind::I32Add {
                            lhs: raw::ValueId(14),
                            rhs: raw::ValueId(3),
                        },
                    )],
                    terminators: vec![raw::Terminator::Return(raw::ValueId(15))],
                },
                raw::Block {
                    id: raw::BlockId(2),
                    parameters: vec![value(16, Type::I32)],
                    instructions: vec![instruction(
                        17,
                        Type::I32,
                        raw::InstructionKind::I32Sub {
                            lhs: raw::ValueId(16),
                            rhs: raw::ValueId(3),
                        },
                    )],
                    terminators: vec![raw::Terminator::Return(raw::ValueId(17))],
                },
            ],
        };
        let countdown = raw::Function {
            id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
            internal_symbol: "zryna_m2_i_m0_f1".to_owned(),
            entry_export: Some("countdown".to_owned()),
            convention: raw::CallingConvention::ZRYNA_INTERNAL_CONTROL_FLOW_V1,
            parameters: vec![value(0, Type::I32)],
            result: Type::I32,
            blocks: vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: Vec::new(),
                    instructions: Vec::new(),
                    terminators: vec![raw::Terminator::Jump {
                        target: raw::BlockId(1),
                        arguments: vec![raw::ValueId(0)],
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: vec![value(1, Type::I32)],
                    instructions: vec![
                        instruction(2, Type::I32, raw::InstructionKind::I32Literal(0)),
                        instruction(
                            3,
                            Type::Bool,
                            raw::InstructionKind::I32GtS {
                                lhs: raw::ValueId(1),
                                rhs: raw::ValueId(2),
                            },
                        ),
                    ],
                    terminators: vec![raw::Terminator::Branch {
                        condition: raw::ValueId(3),
                        true_target: raw::BlockId(2),
                        true_arguments: vec![raw::ValueId(1)],
                        false_target: raw::BlockId(3),
                        false_arguments: vec![raw::ValueId(1)],
                    }],
                },
                raw::Block {
                    id: raw::BlockId(2),
                    parameters: vec![value(4, Type::I32)],
                    instructions: vec![
                        instruction(5, Type::I32, raw::InstructionKind::I32Literal(1)),
                        instruction(
                            6,
                            Type::I32,
                            raw::InstructionKind::I32Sub {
                                lhs: raw::ValueId(4),
                                rhs: raw::ValueId(5),
                            },
                        ),
                    ],
                    terminators: vec![raw::Terminator::Jump {
                        target: raw::BlockId(1),
                        arguments: vec![raw::ValueId(6)],
                    }],
                },
                raw::Block {
                    id: raw::BlockId(3),
                    parameters: vec![value(7, Type::I32)],
                    instructions: Vec::new(),
                    terminators: vec![raw::Terminator::Return(raw::ValueId(7))],
                },
            ],
        };
        control_flow_v1::verify(raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module {
                id: raw::ModuleId(0),
                functions: vec![compute, countdown],
            }],
        })
        .expect("operation and CFG fixture must verify")
    }

    #[test]
    fn emits_only_pinned_internal_call_relocations_deterministically() {
        let program = call_program();
        let target = crate::select_object_target(NATIVE_OBJECT_TARGET).expect("target");
        let artifact = emit_object(&program, target).expect("M2 object");
        let repeated = emit_object(&program, target).expect("repeated M2 object");
        assert_eq!(artifact, repeated);
        assert_eq!(artifact.scalar_abi(), program.scalar_abi());
        let file = object::File::parse(artifact.bytes()).expect("ELF object");
        let sections = file
            .sections()
            .map(|section| section.name().expect("section name"))
            .collect::<Vec<_>>();
        assert_eq!(
            sections,
            [".text", ".rela.text", ".note.GNU-stack", ".symtab", ".strtab", ".shstrtab"]
        );
        let symbols =
            file.symbols().map(|symbol| symbol.name().expect("symbol name")).collect::<Vec<_>>();
        assert_eq!(symbols, ["zryna", "zryna_m2_i_m0_f0", "zryna_m2_i_m0_f1", "zryna_v1_e_run"]);
        let relocations =
            file.section_by_name(".text").expect("text").relocations().collect::<Vec<_>>();
        assert_eq!(relocations.len(), 2);
        assert!(relocations.iter().all(|(_, relocation)| {
            relocation.kind() == RelocationKind::PltRelative
                && relocation.encoding() == RelocationEncoding::X86Branch
                && relocation.size() == 32
                && relocation.addend() == -4
                && relocation.flags()
                    == (RelocationFlags::Elf { r_type: object::elf::R_X86_64_PLT32 })
        }));
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the closed audit matrix keeps every ELF attack explicit"
    )]
    fn closed_audit_rejects_corrupt_oversized_and_forged_relocations() {
        let program = call_program();
        let target = crate::select_object_target(NATIVE_OBJECT_TARGET).expect("target");
        let artifact = emit_object(&program, target).expect("M2 object");
        assert_closed_audit_rejects(b"not an object", &program, "corrupt object");
        assert_closed_audit_rejects(
            &vec![0; MAX_NATIVE_OBJECT_BYTES + 1],
            &program,
            "oversized object",
        );

        let file = object::File::parse(artifact.bytes()).expect("ELF object");
        let relocation_section = file.section_by_name(".rela.text").expect("relocation section");
        let (file_offset, _) = relocation_section.file_range().expect("relocation file range");
        let first = usize::try_from(file_offset).expect("relocation offset");

        let mut wrong_type = artifact.bytes().to_vec();
        wrong_type[first + 8] = u8::try_from(object::elf::R_X86_64_PC32).expect("ELF type");
        assert_closed_audit_rejects(&wrong_type, &program, "wrong relocation type");

        let mut wrong_addend = artifact.bytes().to_vec();
        wrong_addend[first + 16..first + 24].copy_from_slice(&0_i64.to_le_bytes());
        assert_closed_audit_rejects(&wrong_addend, &program, "wrong relocation addend");

        let mut wrong_target = artifact.bytes().to_vec();
        let wrapper_index =
            file.symbol_by_name("zryna_v1_e_run").expect("wrapper symbol").index().0;
        wrong_target[first + 12..first + 16]
            .copy_from_slice(&u32::try_from(wrapper_index).expect("symbol index").to_le_bytes());
        assert_closed_audit_rejects(&wrong_target, &program, "wrong relocation target");

        let relocation_count =
            file.section_by_name(".text").expect("text section").relocations().count();
        let caller = file.symbol_by_name("zryna_m2_i_m0_f1").expect("caller symbol");
        let mut crossing_caller = artifact.bytes().to_vec();
        let crossing_offset = caller.address() + caller.size() - 1;
        crossing_caller[first..first + 8].copy_from_slice(&crossing_offset.to_le_bytes());
        assert_closed_audit_rejects(
            &crossing_caller,
            &program,
            "relocation displacement crossing caller boundary",
        );

        let mut non_call_site = artifact.bytes().to_vec();
        let non_call_offset = caller.address() + 1;
        non_call_site[first..first + 8].copy_from_slice(&non_call_offset.to_le_bytes());
        assert_closed_audit_rejects(&non_call_site, &program, "relocation at non-call opcode");

        let second = first + 24;
        assert_eq!(relocation_count, 2, "fixture relocation count");
        let mut duplicate_relocation = artifact.bytes().to_vec();
        duplicate_relocation.copy_within(first..first + 24, second);
        assert_closed_audit_rejects(
            &duplicate_relocation,
            &program,
            "duplicated and reordered relocation",
        );

        let relocation_header = elf_section_header(artifact.bytes(), relocation_section.index().0);
        let relocation_size = relocation_section.size();
        let mut missing_relocation = artifact.bytes().to_vec();
        missing_relocation[relocation_header + 32..relocation_header + 40]
            .copy_from_slice(&(relocation_size - 24).to_le_bytes());
        assert_closed_audit_rejects(&missing_relocation, &program, "missing relocation");

        let mut extra_relocation = artifact.bytes().to_vec();
        extra_relocation[relocation_header + 32..relocation_header + 40]
            .copy_from_slice(&(relocation_size + 24).to_le_bytes());
        assert_closed_audit_rejects(&extra_relocation, &program, "extra relocation");

        let text = file.section_by_name(".text").expect("text section");
        let text_header = elf_section_header(artifact.bytes(), text.index().0);
        let mut writable_text = artifact.bytes().to_vec();
        writable_text[text_header + 8..text_header + 16].copy_from_slice(&7_u64.to_le_bytes());
        assert_closed_audit_rejects(&writable_text, &program, "writable text section");

        let names = file.section_by_name(".shstrtab").expect("section names");
        let (names_offset, _) = names.file_range().expect("section names range");
        let names_offset = usize::try_from(names_offset).expect("section names offset");
        let marker = names
            .data()
            .expect("section name bytes")
            .windows(b".text\0".len())
            .position(|window| window == b".text\0")
            .expect("text section name");
        let mut renamed_text = artifact.bytes().to_vec();
        renamed_text[names_offset + marker] = b'_';
        assert_closed_audit_rejects(&renamed_text, &program, "renamed text section");

        let symbols = file.section_by_name(".symtab").expect("symbol table");
        let (symbols_offset, _) = symbols.file_range().expect("symbol table range");
        let symbols_offset = usize::try_from(symbols_offset).expect("symbol table offset");
        let body = file.symbol_by_name("zryna_m2_i_m0_f0").expect("body symbol");
        let body_entry = symbols_offset + body.index().0 * 24;

        let mut global_body = artifact.bytes().to_vec();
        global_body[body_entry + 4] = (object::elf::STB_GLOBAL << 4) | object::elf::STT_FUNC;
        assert_closed_audit_rejects(&global_body, &program, "global internal body");

        let mut hidden_body = artifact.bytes().to_vec();
        hidden_body[body_entry + 5] = object::elf::STV_HIDDEN;
        assert_closed_audit_rejects(&hidden_body, &program, "hidden internal body");

        let mut undefined_body = artifact.bytes().to_vec();
        undefined_body[body_entry + 6..body_entry + 8].copy_from_slice(&0_u16.to_le_bytes());
        assert_closed_audit_rejects(&undefined_body, &program, "undefined internal body");

        let mut zero_sized_body = artifact.bytes().to_vec();
        zero_sized_body[body_entry + 16..body_entry + 24].copy_from_slice(&0_u64.to_le_bytes());
        assert_closed_audit_rejects(&zero_sized_body, &program, "zero-sized internal body");

        let next_body = file.symbol_by_name("zryna_m2_i_m0_f1").expect("second body symbol");
        let next_entry = symbols_offset + next_body.index().0 * 24;
        let mut reordered_symbols = artifact.bytes().to_vec();
        for byte in 0..4 {
            reordered_symbols.swap(body_entry + byte, next_entry + byte);
        }
        assert_closed_audit_rejects(&reordered_symbols, &program, "reordered body symbols");
    }

    #[test]
    fn emits_every_operation_branch_jump_loop_and_boolean_wrapper() {
        let program = operations_and_control_flow_program();
        let target = crate::select_object_target(NATIVE_OBJECT_TARGET).expect("target");
        let artifact = emit_object(&program, target).expect("complete M2 object");
        assert_eq!(artifact.scalar_abi().exports().len(), 2);
        let file = object::File::parse(artifact.bytes()).expect("ELF object");
        let globals = file
            .symbols()
            .filter(object::ObjectSymbol::is_global)
            .map(|symbol| symbol.name().expect("global name"))
            .collect::<Vec<_>>();
        assert_eq!(globals, ["zryna_v1_e_compute", "zryna_v1_e_countdown"]);
    }

    #[test]
    fn empty_m2_program_emits_the_closed_empty_object() {
        let program = control_flow_v1::verify(raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions: Vec::new() }],
        })
        .expect("empty M2 program must verify");
        let target = crate::select_object_target(NATIVE_OBJECT_TARGET).expect("target");
        let artifact = emit_object(&program, target).expect("empty M2 object");
        let file = object::File::parse(artifact.bytes()).expect("ELF object");
        assert!(file.section_by_name(".text").is_none());
        assert_eq!(file.symbols().count(), 1);
    }

    #[test]
    fn many_call_free_functions_import_no_quadratic_callee_matrix() {
        let functions = (0..1_024_u32)
            .map(|declaration| raw::Function {
                id: raw::FunctionId { module: raw::ModuleId(0), declaration },
                internal_symbol: format!("zryna_m2_i_m0_f{declaration}"),
                entry_export: None,
                convention: raw::CallingConvention::ZRYNA_INTERNAL_CONTROL_FLOW_V1,
                parameters: Vec::new(),
                result: Type::I32,
                blocks: vec![raw::Block {
                    id: raw::BlockId(0),
                    parameters: Vec::new(),
                    instructions: vec![instruction(
                        0,
                        Type::I32,
                        raw::InstructionKind::I32Literal(0),
                    )],
                    terminators: vec![raw::Terminator::Return(raw::ValueId(0))],
                }],
            })
            .collect();
        let program = control_flow_v1::verify(raw::Program {
            entry_module: raw::ModuleId(0),
            modules: vec![raw::Module { id: raw::ModuleId(0), functions }],
        })
        .expect("bounded many-function program must verify");
        let target = crate::select_object_target(NATIVE_OBJECT_TARGET).expect("target");
        let artifact = emit_object(&program, target).expect("linear call-free object emission");
        let file = object::File::parse(artifact.bytes()).expect("ELF object");
        assert_eq!(file.symbols().count(), 1_025);
        assert_eq!(file.section_by_name(".text").expect("text section").relocations().count(), 0);
    }
}
