//! Raw runtime-only fixtures: bypass the fixed arrangement solely inside this test target.

use std::convert::Infallible;

use wasm_encoder::{
    BlockType, CodeSection, Component, Function, Instruction, MemArg,
    reencode::{Error, Reencode, ReencodeComponent},
};
use wasmparser::{CodeSectionReader, Parser};

struct Replace {
    modules: usize,
    bridge: bool,
    grow: bool,
    marked_cpu: bool,
}
impl Reencode for Replace {
    type Error = Infallible;
    fn memory_type(
        &mut self,
        memory_ty: wasmparser::MemoryType,
    ) -> Result<wasm_encoder::MemoryType, Error<Infallible>> {
        let mut memory = wasm_encoder::reencode::utils::memory_type(self, memory_ty);
        // The fixture permits two pages structurally. Only the independent one-page host limit
        // may reject its growth; a producer-declared maximum cannot substitute for this proof.
        if self.grow && memory.maximum == Some(1) {
            memory.maximum = Some(2);
        }
        Ok(memory)
    }
    fn parse_code_section(
        &mut self,
        code: &mut CodeSection,
        section: CodeSectionReader<'_>,
    ) -> Result<(), Error<Infallible>> {
        if !self.bridge {
            return wasm_encoder::reencode::utils::parse_code_section(self, code, section);
        }
        assert_eq!(section.count(), 1, "fixed bridge has one handle");
        let mut function = Function::new([]);
        if self.grow {
            function.instruction(&Instruction::I32Const(1));
            function.instruction(&Instruction::MemoryGrow(0));
            function.instruction(&Instruction::Drop);
        } else if self.marked_cpu {
            marked_cpu(&mut function);
        } else {
            function.instruction(&Instruction::Loop(BlockType::Empty));
            function.instruction(&Instruction::Br(0));
            function.instruction(&Instruction::End);
        }
        function.instruction(&Instruction::End);
        code.function(&function);
        Ok(())
    }
}
impl ReencodeComponent for Replace {
    fn parse_component_submodule(
        &mut self,
        component: &mut Component,
        parser: Parser,
        module: &[u8],
    ) -> Result<(), Error<Infallible>> {
        self.modules += 1;
        self.bridge = self.modules == 3;
        let result = wasm_encoder::reencode::component_utils::parse_component_submodule(
            self, component, parser, module,
        );
        self.bridge = false;
        result
    }
}

pub(crate) fn replace_handle(bytes: &[u8], grow: bool) -> Vec<u8> {
    let mut component = Component::new();
    Replace { modules: 0, bridge: false, grow, marked_cpu: false }
        .parse_component(&mut component, Parser::new(0), bytes)
        .expect("replace fixture handle");
    component.finish()
}

// Existing import0 is HTTP [constructor]fields: () -> i32. Exactly one call marks
// real guest entry; the Store owns its resource until teardown, under the same cap4.
// The following loop has no calls, sleep, yield or host-cooperative checks. Its
// division depends on mutable guest memory and has a nonzero divisor by construction.
fn marked_cpu(function: &mut Function) {
    let cell = MemArg { offset: 0, align: 2, memory_index: 0 };
    function.instruction(&Instruction::Call(0));
    function.instruction(&Instruction::Drop);
    function.instruction(&Instruction::I32Const(0));
    function.instruction(&Instruction::I32Const(0x1357_9bdf));
    function.instruction(&Instruction::I32Store(cell));
    function.instruction(&Instruction::Loop(BlockType::Empty));
    function.instruction(&Instruction::I32Const(0));
    function.instruction(&Instruction::I32Const(0));
    function.instruction(&Instruction::I32Load(cell));
    function.instruction(&Instruction::I32Const(1_664_525));
    function.instruction(&Instruction::I32Mul);
    function.instruction(&Instruction::I32Const(1_013_904_223));
    function.instruction(&Instruction::I32Add);
    function.instruction(&Instruction::I32Const(0));
    function.instruction(&Instruction::I32Load(cell));
    function.instruction(&Instruction::I32Const(65_535));
    function.instruction(&Instruction::I32And);
    function.instruction(&Instruction::I32Const(1));
    function.instruction(&Instruction::I32Or);
    function.instruction(&Instruction::I32DivU);
    function.instruction(&Instruction::I32Const(-1_640_531_527));
    function.instruction(&Instruction::I32Xor);
    function.instruction(&Instruction::I32Store(cell));
    function.instruction(&Instruction::Br(0));
    function.instruction(&Instruction::End);
}

pub(crate) fn replace_cpu_handle(bytes: &[u8]) -> Vec<u8> {
    let mut component = Component::new();
    Replace { modules: 0, bridge: false, grow: false, marked_cpu: true }
        .parse_component(&mut component, Parser::new(0), bytes)
        .expect("replace marked CPU fixture handle");
    component.finish()
}
