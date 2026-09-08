//! Independent core modules used only by denied-call and interruption component fixtures.

use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Instruction, MemorySection, MemoryType, Module, TypeSection, ValType,
};

#[derive(Clone, Copy)]
pub(super) enum Action {
    DeniedCall,
    Loop,
    InvalidResult,
}

pub(super) fn memory() -> Module {
    memory_with_pages(1)
}

pub(super) fn memory_with_pages(pages: u64) -> Module {
    let mut module = Module::new();
    let mut types = TypeSection::new();
    types.ty().function([ValType::I32; 4], [ValType::I32]);
    module.section(&types);
    let mut functions = FunctionSection::new();
    functions.function(0);
    module.section(&functions);
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: pages,
        maximum: Some(pages),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memories);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("realloc", ExportKind::Func, 0);
    module.section(&exports);
    // Any attempt to construct an output allocation traps independently of the host denial.
    let mut realloc = Function::new([]);
    realloc.instruction(&Instruction::Unreachable);
    realloc.instruction(&Instruction::End);
    let mut code = CodeSection::new();
    code.function(&realloc);
    module.section(&code);
    module
}

pub(super) fn caller(action: Action) -> Module {
    let mut module = Module::new();
    let mut types = TypeSection::new();
    types.ty().function([ValType::I32], []);
    types.ty().function([], [ValType::I32]);
    module.section(&types);
    let mut imports = ImportSection::new();
    imports.import("host", "deny", EntityType::Function(0));
    module.section(&imports);
    let mut functions = FunctionSection::new();
    functions.function(1);
    module.section(&functions);
    let mut exports = ExportSection::new();
    exports.export("run", ExportKind::Func, 1);
    module.section(&exports);
    let mut run = Function::new([]);
    match action {
        Action::DeniedCall => {
            // The flattened list result is indirect and receives a caller-provided result pointer.
            run.instruction(&Instruction::I32Const(0));
            run.instruction(&Instruction::Call(0));
            run.instruction(&Instruction::I32Const(0));
        }
        Action::Loop => {
            run.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            run.instruction(&Instruction::Br(0));
            run.instruction(&Instruction::End);
            run.instruction(&Instruction::I32Const(0));
        }
        Action::InvalidResult => {
            run.instruction(&Instruction::I32Const(2));
        }
    }
    run.instruction(&Instruction::End);
    let mut code = CodeSection::new();
    code.function(&run);
    module.section(&code);
    module
}
