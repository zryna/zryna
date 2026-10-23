//! Complete canonical envelope, in the independently frozen field order.
use super::{Failure, HEADER, VERSION, raw, writer::Writer};
impl Writer {
    pub(super) fn span(&mut self, span: &zryna_source::UntrustedSpan) -> Result<(), Failure> {
        self.u32(span.file)?;
        self.u32(span.start)?;
        self.u32(span.end)
    }
    fn instruction(&mut self, instruction: &raw::Instruction) -> Result<(), Failure> {
        self.span(&instruction.site_span)?;
        self.option(instruction.result.as_ref(), Self::value_definition)?;
        self.operation(&instruction.operation)
    }
    fn block(&mut self, block: &raw::Block) -> Result<(), Failure> {
        self.u32(block.id)?;
        self.vector(&block.value_parameters, Self::value_definition)?;
        self.vector(&block.instructions, Self::instruction)?;
        self.terminator(&block.terminator)
    }
    fn function(&mut self, function: &raw::Function) -> Result<(), Failure> {
        self.blob(&function.key)?;
        self.span(&function.original_span)?;
        self.option(function.executable_export.as_ref(), |writer, name| {
            writer.blob(name.as_bytes())
        })?;
        self.vector(&function.parameters, Self::parameter)?;
        self.ty(function.result_type)?;
        self.vector(&function.places, Self::place)?;
        self.vector(&function.loans, Self::loan)?;
        self.vector(&function.blocks, Self::block)?;
        self.vector(&function.cleanup_plans, Self::cleanup_plan)
    }
}
pub(super) fn encode(program: &raw::Program) -> Result<Vec<u8>, Failure> {
    let mut writer = Writer::new();
    write(program, &mut writer)?;
    Ok(writer.bytes)
}

// The exact serializer performs its census without allocating an output buffer.
pub(super) fn census(program: &raw::Program) -> Result<usize, Failure> {
    let mut writer = Writer::counter();
    write(program, &mut writer)?;
    Ok(writer.length())
}

fn write(program: &raw::Program, writer: &mut Writer) -> Result<(), Failure> {
    writer.bytes(HEADER)?;
    writer.u32(VERSION)?;
    writer.blob(program.language_contract.as_bytes())?;
    writer.vector(&program.modules, |writer, module| {
        writer.u32(module.id)?;
        writer.u32(module.functions)
    })?;
    writer.vector(&program.declarations, |writer, declaration| {
        writer.u32(declaration.module)?;
        writer.u32(declaration.function)?;
        writer.u32(declaration.parameters)?;
        writer.span(&declaration.span)
    })?;
    writer.vector(&program.type_keys, |writer, key| writer.blob(key))?;
    writer.bytes(&program.universe)?;
    writer.bytes(&program.linear32)?;
    writer.bytes(&program.linux_x86_64)?;
    writer.u32(program.entry_module)?;
    writer.vector(&program.functions, Writer::function)?;
    Ok(())
}
