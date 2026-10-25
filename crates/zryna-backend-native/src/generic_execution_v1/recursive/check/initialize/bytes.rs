//! Complete bounded physical-byte zero/copy loop; no hidden source or destination writes.
use super::{Cursor, World};
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{Opcode as O, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
pub(super) fn check<'f>(
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    source: Option<Value>,
    destination: Value,
    size: u32,
) -> Result<Cursor<'f>, Diagnostic> {
    if size == 0 {
        return Ok(reader);
    }
    let zero = reader.constant(types::I32, 0)?;
    let repeat = reader.jump(&[zero])?;
    let mut header = world.at(repeat, &[types::I32])?;
    let index = world.f.dfg.block_params(repeat)[0];
    let count = header.constant(types::I32, i64::from(size))?;
    let more = header.compare(IntCC::UnsignedLessThan, [index, count])?;
    let (body, done) = header.branch(more)?;
    let mut body = world.at(body, &[])?;
    let offset = body.unary(O::Uextend, types::I64, index)?;
    let value = if let Some(source) = source {
        let address = body.binary(O::Iadd, types::I64, [source, offset])?;
        body.load(types::I8, address, 0)?
    } else {
        body.constant(types::I8, 0)?
    };
    let target = body.binary(O::Iadd, types::I64, [destination, offset])?;
    body.store(target, 0, value)?;
    let one = body.constant(types::I32, 1)?;
    let next = body.binary(O::Iadd, types::I32, [index, one])?;
    if body.jump(&[next])? != repeat {
        return Err(invariant());
    }
    world.at(done, &[])
}
