//! Source-derived exact operator grammar independent of encoder buffers/instruction objects.
use super::{Diagnostic, Model, VerifiedInvocation, invariant};
use wasmparser::{BlockType, FunctionBody, Operator, OperatorsReader, ValType};
use zryna_abi::{ScalarType, ScalarValue};
use zryna_ir::generic_v1::execution_v1::VerifiedParameter;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Token {
    I32(i32),
    I64(i64),
    Get(u32),
    Set(u32),
    Global(u32),
    GlobalSet(u32),
    Call(u32),
    Store,
    Store8,
    Load,
    Load8,
    If,
    End,
    Trap,
    Ne,
    Gt,
    Eqz,
    Wide,
    Shift,
    Or,
}
struct Cursor<'b, 'm, 'p, 'a, 's> {
    ops: OperatorsReader<'b>,
    m: &'m Model<'p, 'a, 's>,
}
impl Cursor<'_, '_, '_, '_, '_> {
    fn need(&mut self, want: Token) -> Result<(), Diagnostic> {
        self.m.take(1)?;
        let actual = match self.ops.read().map_err(|_| invariant())? {
            Operator::I32Const { value } => Token::I32(value),
            Operator::I64Const { value } => Token::I64(value),
            Operator::LocalGet { local_index } => Token::Get(local_index),
            Operator::LocalSet { local_index } => Token::Set(local_index),
            Operator::GlobalGet { global_index } => Token::Global(global_index),
            Operator::GlobalSet { global_index } => Token::GlobalSet(global_index),
            Operator::Call { function_index } => Token::Call(function_index),
            Operator::I32Store { memarg }
                if memarg.memory == 0 && memarg.align == 2 && memarg.offset == 0 =>
            {
                Token::Store
            }
            Operator::I32Store8 { memarg }
                if memarg.memory == 0 && memarg.align == 0 && memarg.offset == 0 =>
            {
                Token::Store8
            }
            Operator::I32Load { memarg }
                if memarg.memory == 0 && memarg.align == 2 && memarg.offset == 0 =>
            {
                Token::Load
            }
            Operator::I32Load8U { memarg }
                if memarg.memory == 0 && memarg.align == 0 && memarg.offset == 0 =>
            {
                Token::Load8
            }
            Operator::If { blockty: BlockType::Empty } => Token::If,
            Operator::End => Token::End,
            Operator::Unreachable => Token::Trap,
            Operator::I32Ne => Token::Ne,
            Operator::I32GtU => Token::Gt,
            Operator::I32Eqz => Token::Eqz,
            Operator::I64ExtendI32U => Token::Wide,
            Operator::I64Shl => Token::Shift,
            Operator::I64Or => Token::Or,
            _ => return Err(invariant()),
        };
        if actual != want {
            return Err(invariant());
        }
        Ok(())
    }
    fn series(&mut self, tokens: impl IntoIterator<Item = Token>) -> Result<(), Diagnostic> {
        for token in tokens {
            self.need(token)?;
        }
        Ok(())
    }
    fn guard(&mut self) -> Result<(), Diagnostic> {
        self.series([Token::If, Token::Trap, Token::End])
    }
    fn zero_global(&mut self, global: u32) -> Result<(), Diagnostic> {
        self.need(Token::Global(global))?;
        self.guard()
    }
    fn reset(&mut self) -> Result<(), Diagnostic> {
        for global in [1, 5, 6, 7, 9] {
            self.series([Token::I32(0), Token::GlobalSet(global)])?;
        }
        if self.m.accounted() {
            for global in [10, 11, 12] {
                self.series([Token::I32(0), Token::GlobalSet(global)])?;
            }
        }
        self.series([
            Token::I32(self.m.memory.claims().heap_start.cast_signed()),
            Token::GlobalSet(0),
        ])
    }
}
pub(super) fn check(
    body: &FunctionBody<'_>,
    m: &Model<'_, '_, '_>,
    request: &VerifiedInvocation<'_>,
) -> Result<(), Diagnostic> {
    let mut locals = body.get_locals_reader().map_err(|_| invariant())?;
    if locals.get_count() != 1 || locals.read().map_err(|_| invariant())? != (2, ValType::I32) {
        return Err(invariant());
    }
    let mut c = Cursor { ops: body.get_operators_reader().map_err(|_| invariant())?, m };
    let out = m.memory.claims().scratch_start.checked_add(16).ok_or_else(invariant)?;
    for global in [2, 8] {
        c.zero_global(global)?;
    }
    if m.accounted() {
        c.zero_global(12)?;
    }
    if m.recursive_count() != 0 {
        c.zero_global(m.recursive_used_global())?;
    }
    c.series([
        Token::I32(0),
        Token::Set(0),
        Token::I32(0),
        Token::Set(1),
        Token::I32(out.cast_signed()),
        Token::I32(0),
        Token::Store,
        Token::I32(1),
        Token::GlobalSet(2),
    ])?;
    c.reset()?;
    let target =
        *m.program.export_functions().get(request.export().index()).ok_or_else(invariant)?;
    arguments(&mut c, m, request, target)?;
    c.series([
        Token::I32(m.memory.claims().frames_start.cast_signed()),
        Token::I32(out.cast_signed()),
        Token::Call(m.source_base + u32::try_from(target).map_err(|_| invariant())?),
        Token::Set(0),
        Token::Get(0),
        Token::Global(1),
        Token::Ne,
    ])?;
    c.guard()?;
    c.series([Token::Get(0), Token::I32(5), Token::Gt])?;
    c.guard()?;
    c.zero_global(8)?;
    if m.accounted() {
        c.zero_global(12)?;
        for (global, max) in [(10, 1_048_576), (11, 4_194_304)] {
            c.series([Token::Global(global), Token::I32(max), Token::Gt])?;
            c.guard()?;
        }
    }
    if m.recursive_count() != 0 {
        c.zero_global(m.recursive_used_global())?;
    }
    c.series([Token::Global(2), Token::I32(1), Token::Ne])?;
    c.guard()?;
    c.series([
        Token::Get(0),
        Token::Eqz,
        Token::If,
        Token::I32(out.cast_signed()),
        if request.export().result() == ScalarType::Bool { Token::Load8 } else { Token::Load },
        Token::Set(1),
    ])?;
    if request.export().result() == ScalarType::Bool {
        c.series([Token::Get(1), Token::I32(1), Token::Gt])?;
        c.guard()?;
    }
    c.series([Token::End, Token::I32(out.cast_signed()), Token::I32(0), Token::Store])?;
    c.reset()?;
    c.series([
        Token::I32(0),
        Token::GlobalSet(2),
        Token::Get(0),
        Token::Wide,
        Token::I64(32),
        Token::Shift,
        Token::Get(1),
        Token::Wide,
        Token::Or,
        Token::End,
    ])?;
    if !c.ops.eof() {
        return Err(invariant());
    }
    Ok(())
}

// The independent cursor consumes the exact same-source baked scalar argument stores.
fn arguments(
    c: &mut Cursor<'_, '_, '_, '_, '_>,
    m: &Model<'_, '_, '_>,
    request: &VerifiedInvocation<'_>,
    target: usize,
) -> Result<(), Diagnostic> {
    for (parameter, value) in m.functions[target].parameters().zip(request.arguments()) {
        let VerifiedParameter::Value(parameter) = parameter else { return Err(invariant()) };
        let slot = &m.memory.claims().functions[target].values[parameter.id() as usize];
        let address =
            m.memory.claims().frames_start.checked_add(slot.offset).ok_or_else(invariant)?;
        let value = match value {
            ScalarValue::Bool(value) => i32::from(*value),
            ScalarValue::I32(value) => *value,
        };
        c.series([
            Token::I32(address.cast_signed()),
            Token::I32(value),
            match slot.bytes {
                1 => Token::Store8,
                4 => Token::Store,
                _ => return Err(invariant()),
            },
        ])?;
    }
    Ok(())
}
