use super::Context;
use wasm_encoder::{BlockType, Function, Instruction as I, MemArg, ValType};

const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
pub(super) const ARENA_START: i32 = 65_536;

fn emit(body: &mut Function, instructions: impl IntoIterator<Item = I<'static>>) {
    for instruction in instructions {
        body.instruction(&instruction);
    }
}
fn mode(body: &mut Function, value: i32) {
    emit(
        body,
        [
            I::LocalGet(0),
            I::I32Const(-268435456),
            I::I32And,
            I::I32Const(value),
            I::I32Eq,
            I::If(BlockType::Empty),
        ],
    );
}

pub(super) fn getter() -> Function {
    let mut body = Function::new([(1, ValType::I32)]);
    mode(&mut body, 0x20000000);
    emit(
        &mut body,
        [
            I::LocalGet(0),
            I::I32Const(24),
            I::I32ShrU,
            I::I32Const(15),
            I::I32And,
            I::GlobalSet(3),
            I::LocalGet(0),
            I::I32Const(0xffffff),
            I::I32And,
            I::GlobalSet(4),
            I::I32Const(0),
            I::GlobalSet(5),
            I::I32Const(0),
            I::Return,
            I::End,
        ],
    );
    mode(&mut body, 0x10000000);
    emit(
        &mut body,
        [
            I::LocalGet(0),
            I::I32Const(0xfffffff),
            I::I32And,
            I::GlobalGet(3),
            I::I32Eq,
            I::If(BlockType::Empty),
            I::GlobalGet(5),
            I::I32Const(1),
            I::I32Add,
            I::GlobalSet(5),
            I::GlobalGet(5),
            I::GlobalGet(4),
            I::I32Eq,
            I::If(BlockType::Empty),
            I::GlobalGet(3),
            I::GlobalSet(1),
            I::End,
            I::End,
            I::GlobalGet(1),
            I::Return,
            I::End,
        ],
    );
    mode(&mut body, 0x40000000);
    emit(
        &mut body,
        [
            I::LocalGet(0),
            I::I32Const(0xfffffff),
            I::I32And,
            I::LocalTee(1),
            I::I32Eqz,
            I::If(BlockType::Empty),
            I::GlobalGet(2),
            I::Return,
            I::End,
            I::LocalGet(1),
            I::GlobalGet(2),
            I::I32GtU,
            I::If(BlockType::Empty),
            I::I32Const(-1),
            I::Return,
            I::End,
            I::LocalGet(1),
            I::I32Const(1),
            I::I32Sub,
            I::I32Const(4),
            I::I32Mul,
            I::I32Load(WORD),
            I::Return,
            I::End,
            I::GlobalGet(1),
            I::End,
        ],
    );
    body
}

pub(super) fn recorder() -> Function {
    let mut body = Function::new([]);
    emit(
        &mut body,
        [
            I::GlobalGet(4),
            I::I32Eqz,
            I::If(BlockType::Empty),
            I::Return,
            I::End,
            I::GlobalGet(2),
            I::I32Const(4096),
            I::I32GeU,
            I::If(BlockType::Empty),
            I::I32Const(4097),
            I::GlobalSet(2),
            I::Return,
            I::End,
            I::GlobalGet(2),
            I::I32Const(4),
            I::I32Mul,
            I::LocalGet(0),
            I::I32Store(WORD),
            I::GlobalGet(2),
            I::I32Const(1),
            I::I32Add,
            I::GlobalSet(2),
            I::End,
        ],
    );
    body
}

pub(super) fn record(word: u32, context: &Context<'_>, body: &mut Function) {
    body.instruction(&I::I32Const(i32::try_from(word).expect("bounded logical trace word")));
    body.instruction(&I::Call(context.observation + 1));
}

pub(super) fn root(
    function: zryna_ir::data_ownership_v1::VerifiedFunction<'_>,
    place: u32,
    context: &Context<'_>,
    body: &mut Function,
) {
    for word in [0x20000000 + function.id().module(), function.id().declaration(), place] {
        record(word, context, body);
    }
}

pub(super) fn probe(code: i32, context: &Context<'_>, body: &mut Function) {
    emit(body, [I::I32Const(0x10000000 + code), I::Call(context.observation), I::Drop]);
}

pub(super) fn value_kind(category: zryna_layout::TypeCategory) -> Option<u32> {
    use zryna_layout::TypeCategory as T;
    match category {
        T::Bool | T::I32 => None,
        T::String => Some(1),
        T::Struct | T::FixedArray | T::Vec => Some(2),
        T::Enum => Some(3),
        T::Shared => Some(4),
        T::Weak => Some(5),
    }
}
