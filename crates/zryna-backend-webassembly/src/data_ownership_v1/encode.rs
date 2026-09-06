use wasm_encoder::{
    CodeSection, ConstExpr, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    GlobalType, Instruction, MemorySection, MemoryType, Module, TypeSection, ValType,
};
use zryna_ir::data_ownership_v1::{FunctionIdentity, VerifiedFunction, VerifiedProgram};

use super::error;

mod control;
mod memory;
mod operations;
mod values;

const MEMORY_PAGES: u64 = 256;

pub(super) fn module(program: &VerifiedProgram) -> Result<Vec<u8>, zryna_diagnostics::Diagnostic> {
    let functions = program
        .modules()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
        .collect::<Vec<_>>();
    let layouts = program.linear32_layouts();
    let type_count = u32::try_from(layouts.types().len()).map_err(|_| index_error())?;
    let mut arities = vec![1_usize];
    for function in &functions {
        let arity = function.parameters().len() + function.borrow_parameters().len();
        if !arities.contains(&arity) {
            arities.push(arity);
        }
    }
    let mut module = Module::new();
    let mut types = TypeSection::new();
    for arity in &arities {
        types.ty().function(vec![ValType::I32; *arity], [ValType::I32]);
    }
    let drop_type = u32::try_from(arities.len()).map_err(|_| index_error())?;
    types.ty().function([ValType::I32], []);
    let copy_type = drop_type + 1;
    types.ty().function([ValType::I32, ValType::I32, ValType::I32], []);
    module.section(&types);

    let mut declarations = FunctionSection::new();
    declarations.function(0);
    declarations.function(copy_type);
    for _ in 0..type_count {
        declarations.function(0);
    }
    for _ in 0..type_count {
        declarations.function(drop_type);
    }
    for function in &functions {
        let arity = function.parameters().len() + function.borrow_parameters().len();
        declarations.function(
            u32::try_from(arities.iter().position(|candidate| *candidate == arity).unwrap_or(0))
                .map_err(|_| index_error())?,
        );
    }
    for function in functions.iter().filter(|function| function.public_export().is_some()) {
        let arity = function.parameters().len() + function.borrow_parameters().len();
        declarations.function(
            u32::try_from(arities.iter().position(|candidate| *candidate == arity).unwrap_or(0))
                .map_err(|_| index_error())?,
        );
    }
    module.section(&declarations);

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: MEMORY_PAGES,
        maximum: Some(MEMORY_PAGES),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memories);
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType { val_type: ValType::I32, mutable: true, shared: false },
        &ConstExpr::i32_const(1024),
    );
    module.section(&globals);

    let mut exports = ExportSection::new();
    let program_base = 2 + type_count * 2;
    let mut wrapper_index =
        program_base + u32::try_from(functions.len()).map_err(|_| index_error())?;
    for function in &functions {
        if let Some(export) = function.public_export() {
            exports.export(export.webassembly_name().as_str(), ExportKind::Func, wrapper_index);
            wrapper_index += 1;
        }
    }
    module.section(&exports);

    let context = Context { functions: &functions, layouts, type_count, program_base };
    let mut code = CodeSection::new();
    code.function(&allocator());
    code.function(&copy_memory());
    for layout in layouts.types() {
        code.function(&memory::clone_helper(layout, &context)?);
    }
    for layout in layouts.types() {
        code.function(&memory::drop_helper(layout, &context)?);
    }
    for function in &functions {
        code.function(&encode_function(*function, &context)?);
    }
    for (index, function) in functions.iter().enumerate() {
        if function.public_export().is_some() {
            code.function(&export_wrapper(
                *function,
                program_base + u32::try_from(index).map_err(|_| index_error())?,
            )?);
        }
    }
    module.section(&code);
    Ok(module.finish())
}

fn allocator() -> Function {
    let mut function = Function::new([(2, ValType::I32)]);
    function.instruction(&Instruction::GlobalGet(0));
    function.instruction(&Instruction::LocalTee(1));
    function.instruction(&Instruction::LocalGet(0));
    function.instruction(&Instruction::I32Add);
    function.instruction(&Instruction::I32Const(7));
    function.instruction(&Instruction::I32Add);
    function.instruction(&Instruction::I32Const(-8));
    function.instruction(&Instruction::I32And);
    function.instruction(&Instruction::LocalTee(2));
    function.instruction(&Instruction::LocalGet(1));
    function.instruction(&Instruction::I32LtU);
    function.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    function.instruction(&Instruction::Unreachable);
    function.instruction(&Instruction::End);
    function.instruction(&Instruction::LocalGet(2));
    function.instruction(&Instruction::I32Const(
        i32::try_from(MEMORY_PAGES * 65_536).unwrap_or(i32::MAX),
    ));
    function.instruction(&Instruction::I32GtU);
    function.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    function.instruction(&Instruction::Unreachable);
    function.instruction(&Instruction::End);
    function.instruction(&Instruction::LocalGet(2));
    function.instruction(&Instruction::GlobalSet(0));
    function.instruction(&Instruction::LocalGet(1));
    function.instruction(&Instruction::End);
    function
}

pub(super) struct Context<'a> {
    functions: &'a [VerifiedFunction<'a>],
    pub(super) layouts: &'a zryna_layout::VerifiedLayouts,
    type_count: u32,
    program_base: u32,
}

impl Context<'_> {
    pub(super) fn function_index(
        &self,
        id: FunctionIdentity,
    ) -> Result<u32, zryna_diagnostics::Diagnostic> {
        self.functions
            .iter()
            .position(|function| function.id() == id)
            .and_then(|index| u32::try_from(index).ok())
            .and_then(|index| self.program_base.checked_add(index))
            .ok_or_else(index_error)
    }

    pub(super) const fn clone_index(ty: zryna_layout::TypeId) -> u32 {
        2 + ty.index()
    }

    pub(super) fn drop_index(&self, ty: zryna_layout::TypeId) -> u32 {
        2 + self.type_count + ty.index()
    }
}

fn export_wrapper(
    function: VerifiedFunction<'_>,
    target: u32,
) -> Result<Function, zryna_diagnostics::Diagnostic> {
    let parameters = function.parameters().len() + function.borrow_parameters().len();
    let result = u32::try_from(parameters).map_err(|_| index_error())?;
    let mut body = Function::new([(1, ValType::I32)]);
    body.instruction(&Instruction::I32Const(1024));
    body.instruction(&Instruction::GlobalSet(0));
    for index in 0..parameters {
        body.instruction(&Instruction::LocalGet(u32::try_from(index).map_err(|_| index_error())?));
    }
    body.instruction(&Instruction::Call(target));
    body.instruction(&Instruction::LocalSet(result));
    body.instruction(&Instruction::I32Const(1024));
    body.instruction(&Instruction::GlobalSet(0));
    body.instruction(&Instruction::LocalGet(result));
    body.instruction(&Instruction::End);
    Ok(body)
}

fn copy_memory() -> Function {
    let mut body = Function::new([(1, ValType::I32)]);
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::LocalSet(3));
    body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::LocalGet(2));
    body.instruction(&Instruction::I32GeU);
    body.instruction(&Instruction::BrIf(1));
    body.instruction(&Instruction::LocalGet(0));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalGet(1));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    body.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    body.instruction(&Instruction::LocalGet(3));
    body.instruction(&Instruction::I32Const(1));
    body.instruction(&Instruction::I32Add);
    body.instruction(&Instruction::LocalSet(3));
    body.instruction(&Instruction::Br(0));
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::End);
    body
}

#[derive(Clone, Copy)]
pub(super) struct Locals {
    pub(super) frame: u32,
    pub(super) borrows: u32,
    pub(super) scratch: u32,
    pub(super) heap: u32,
    pub(super) state: u32,
}

#[allow(clippy::too_many_lines)]
fn encode_function(
    function: VerifiedFunction<'_>,
    context: &Context<'_>,
) -> Result<Function, zryna_diagnostics::Diagnostic> {
    let parameter_count = function.parameters().len() + function.borrow_parameters().len();
    let value_count = function
        .blocks()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .filter_map(zryna_ir::data_ownership_v1::VerifiedInstruction::result)
        .map(|id| id.index() + 1)
        .chain(function.parameters().map(|value| value.id().index() + 1))
        .chain(
            function
                .blocks()
                .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::parameters)
                .map(|value| value.id().index() + 1),
        )
        .max()
        .unwrap_or(0);
    let borrow_count = function
        .borrow_parameters()
        .map(|borrow| borrow.id().index() + 1)
        .chain(
            function
                .blocks()
                .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
                .filter_map(zryna_ir::data_ownership_v1::VerifiedInstruction::borrow)
                .map(|id| id.index() + 1),
        )
        .max()
        .unwrap_or(0);
    let max_edge = function
        .blocks()
        .map(|block| {
            block.terminator().edges().map(|edge| edge.arguments().len()).max().unwrap_or(0) + 1
        })
        .max()
        .unwrap_or(1);
    let local_base = value_count.max(u32::try_from(parameter_count).map_err(|_| index_error())?);
    let locals = Locals {
        frame: local_base,
        borrows: local_base + 1,
        scratch: local_base + 1 + borrow_count,
        heap: local_base + 1 + borrow_count + u32::try_from(max_edge).map_err(|_| index_error())?,
        state: local_base
            + 5
            + borrow_count
            + u32::try_from(max_edge).map_err(|_| index_error())?,
    };
    let declared = locals.state + 1 - u32::try_from(parameter_count).map_err(|_| index_error())?;
    let mut body = Function::new((declared > 0).then_some((declared, ValType::I32)));
    let frame_bytes = function
        .places()
        .len()
        .checked_mul(4)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(index_error)?;
    body.instruction(&Instruction::I32Const(frame_bytes));
    body.instruction(&Instruction::Call(0));
    body.instruction(&Instruction::LocalSet(locals.frame));
    for place in function.places() {
        if let zryna_ir::data_ownership_v1::VerifiedPlaceKind::Parameter(ordinal) = place.kind() {
            operations::place_address(
                function,
                place.id().index(),
                locals,
                context.layouts,
                &mut body,
            )?;
            body.instruction(&Instruction::LocalGet(ordinal));
            operations::store(&mut body);
        }
    }
    for (ordinal, borrow) in function.borrow_parameters().enumerate() {
        body.instruction(&Instruction::LocalGet(
            u32::try_from(function.parameters().len() + ordinal).map_err(|_| index_error())?,
        ));
        body.instruction(&Instruction::LocalSet(locals.borrows + borrow.id().index()));
    }
    body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
    let block_parameters =
        function.blocks().map(|block| block.parameters().collect::<Vec<_>>()).collect::<Vec<_>>();
    for block in function.blocks() {
        body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::LocalGet(locals.state));
        body.instruction(&Instruction::I32Const(
            i32::try_from(block.id().index()).map_err(|_| index_error())?,
        ));
        body.instruction(&Instruction::I32Ne);
        body.instruction(&Instruction::BrIf(0));
        for instruction in block.instructions() {
            operations::instruction(function, instruction, locals, context, &mut body)?;
        }
        control::terminator(
            function,
            block.terminator(),
            &block_parameters,
            locals,
            context,
            &mut body,
        )?;
        body.instruction(&Instruction::End);
    }
    body.instruction(&Instruction::Unreachable);
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::I32Const(0));
    body.instruction(&Instruction::End);
    Ok(body)
}

pub(super) fn index_error() -> zryna_diagnostics::Diagnostic {
    error("ZRYNA-W3001", "verified DataOwnershipV1 index exceeds core WebAssembly limits")
}
