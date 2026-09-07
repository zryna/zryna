//! A private scalar core invocation whose result becomes the WASI command discriminant.

use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Instruction, Module, TypeSection, ValType,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::{Type, VerifiedProgram};

const INVALID_INVOCATION: &str = "ZRYNA-W4010";
pub(super) const CORE_INSTANCE: &str = "scalar-core";
pub(super) const RUN_EXPORT: &str = "run";

/// Only an export in the matching verified scalar program can construct this request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CommandInvocation {
    export: String,
    arguments: Vec<i32>,
    expected: i32,
}

impl CommandInvocation {
    pub(super) fn new(
        program: &VerifiedProgram,
        export: &str,
        arguments: &[i32],
        expected: i32,
    ) -> Result<Self, Diagnostic> {
        let function = program
            .functions()
            .find(|function| function.export_name().as_str() == export)
            .ok_or_else(|| invalid("command scalar export is not in the verified program"))?;
        if arguments.len() > 32
            || function.abi_export().webassembly_name().as_str().len() > 256
            || function.parameters().len() != arguments.len()
            || function.parameters().iter().any(|ty| *ty != Type::I32)
            || function.return_type() != Type::I32
        {
            return Err(invalid("command invocation does not match its verified i32 signature"));
        }
        Ok(Self {
            export: function.abi_export().webassembly_name().as_str().to_owned(),
            arguments: arguments.to_vec(),
            expected,
        })
    }

    pub(super) fn export(&self) -> &str {
        &self.export
    }

    pub(super) fn arguments(&self) -> &[i32] {
        &self.arguments
    }

    pub(super) fn expected(&self) -> i32 {
        self.expected
    }
}

/// Produces a second core module; the compiler-produced scalar module is never rewritten.
pub(super) fn encode(invocation: &CommandInvocation) -> Vec<u8> {
    let mut types = TypeSection::new();
    types
        .ty()
        .function(std::iter::repeat_n(ValType::I32, invocation.arguments.len()), [ValType::I32]);
    types.ty().function([], [ValType::I32]);

    let mut imports = ImportSection::new();
    imports.import(CORE_INSTANCE, invocation.export(), EntityType::Function(0));
    let mut functions = FunctionSection::new();
    functions.function(1);
    let mut exports = ExportSection::new();
    exports.export(RUN_EXPORT, ExportKind::Func, 1);

    let mut run = Function::new([]);
    for argument in invocation.arguments() {
        run.instruction(&Instruction::I32Const(*argument));
    }
    run.instruction(&Instruction::Call(0));
    run.instruction(&Instruction::I32Const(invocation.expected()));
    // result<_, _> has no payload: canonical discriminant 0 is success, 1 is error.
    run.instruction(&Instruction::I32Ne);
    run.instruction(&Instruction::End);
    let mut code = CodeSection::new();
    code.function(&run);

    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&functions);
    module.section(&exports);
    module.section(&code);
    module.finish()
}

fn invalid(message: &str) -> Diagnostic {
    Diagnostic::error(
        INVALID_INVOCATION,
        None,
        message,
        "use an exact i32 export and invocation from the retained verified source",
    )
}
