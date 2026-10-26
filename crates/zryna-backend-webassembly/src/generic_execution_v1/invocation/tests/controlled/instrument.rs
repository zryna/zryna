//! Explicitly untrusted, tests-only prefix instrumentation of canonical roles and source Bounds.
use super::{Model, Value, transport};
use crate::generic_execution_v1::model::{self, containers as C};
use wasm_encoder::{BlockType, Encode, Instruction as I, MemArg};
use wasmparser::{BinaryReader, FunctionBody, Operator};
const GLOBALS: u32 = 24;
fn ops(out: &mut Vec<u8>, items: impl IntoIterator<Item = I<'static>>) {
    for item in items {
        item.encode(out);
    }
}
fn get(out: &mut Vec<u8>, base: u32, offset: u32) {
    ops(out, [I::GlobalGet(base + offset)]);
}
fn set(out: &mut Vec<u8>, base: u32, offset: u32) {
    ops(out, [I::GlobalSet(base + offset)]);
}
fn zero(out: &mut Vec<u8>, global: u32) {
    ops(out, [I::I32Const(0), I::GlobalSet(global)]);
}
fn increment(out: &mut Vec<u8>, global: u32) {
    ops(out, [I::GlobalGet(global), I::I32Const(1), I::I32Add, I::GlobalSet(global)]);
}
fn snapshot(out: &mut Vec<u8>, base: u32) {
    increment(out, base + 2);
    ops(out, [I::GlobalGet(8)]);
    set(out, base, 22);
    ops(out, [I::GlobalGet(12)]);
    set(out, base, 23);
}
fn failure(base: u32, canonical: i32, logical: i32, second: bool) -> Vec<u8> {
    let mut out = Vec::new();
    if second {
        increment(&mut out, base + 1);
    }
    get(&mut out, base, 0);
    if second {
        get(&mut out, base, 1);
        ops(&mut out, [I::I32Const(2), I::I32Eq, I::I32And]);
    }
    ops(&mut out, [I::If(BlockType::Empty)]);
    snapshot(&mut out, base);
    ops(
        &mut out,
        [I::I32Const(logical), I::GlobalSet(1), I::I32Const(canonical), I::Return, I::End],
    );
    out
}
fn capture(out: &mut Vec<u8>, base: u32, length: u32, bytes: u32) {
    ops(out, [I::LocalGet(1)]);
    set(out, base, length);
    for n in 0..8 {
        zero(out, base + bytes + n);
        ops(
            out,
            [
                I::LocalGet(1),
                I::I32Const(i32::try_from(n).expect("eight bytes")),
                I::I32GtU,
                I::If(BlockType::Empty),
                I::LocalGet(0),
                I::I32Load8U(MemArg { offset: u64::from(n), align: 0, memory_index: 0 }),
            ],
        );
        set(out, base, bytes + n);
        ops(out, [I::End]);
    }
}
fn releases(base: u32) -> Vec<u8> {
    let mut out = Vec::new();
    ops(
        &mut out,
        [
            I::LocalGet(1),
            I::I32Eqz,
            I::I32Eqz,
            I::If(BlockType::Empty),
            I::LocalGet(1),
            I::I32Const(8),
            I::I32GtU,
            I::If(BlockType::Empty),
            I::Unreachable,
            I::End,
        ],
    );
    get(&mut out, base, 3);
    ops(&mut out, [I::I32Eqz, I::If(BlockType::Empty)]);
    capture(&mut out, base, 4, 5);
    ops(&mut out, [I::End]);
    capture(&mut out, base, 13, 14);
    increment(&mut out, base + 3);
    ops(&mut out, [I::End]);
    out
}
fn prefix(body: &[u8], prefix: &[u8]) -> Vec<u8> {
    let reader = FunctionBody::new(BinaryReader::new(body, 0))
        .get_operators_reader()
        .expect("original locals");
    let at = transport::offset(reader.original_position());
    let mut changed = body[..at].to_vec();
    changed.extend_from_slice(prefix);
    changed.extend_from_slice(&body[at..]);
    changed
}
fn bounds(body: &[u8], base: u32) -> Vec<u8> {
    let mut reader =
        FunctionBody::new(BinaryReader::new(body, 0)).get_operators_reader().expect("source body");
    let mut matches = Vec::new();
    while !reader.eof() {
        if matches!(reader.read().expect("actual source operator"), Operator::I32GeU) {
            matches.push(transport::offset(reader.original_position()));
        }
    }
    assert_eq!(matches.len(), 1, "unique authenticated indexed Bounds comparison in frozen source");
    let at = matches[0];
    let mut add = Vec::new();
    get(&mut add, base, 0);
    ops(&mut add, [I::If(BlockType::Empty)]);
    snapshot(&mut add, base);
    ops(&mut add, [I::End]);
    get(&mut add, base, 0);
    ops(&mut add, [I::I32Or]);
    let mut changed = body[..at].to_vec();
    changed.extend_from_slice(&add);
    changed.extend_from_slice(&body[at..]);
    changed
}
fn configure(base: u32, used: Option<u32>) -> Vec<u8> {
    let mut out = vec![0];
    for global in [Some(8), Some(12), Some(2), used].into_iter().flatten() {
        ops(&mut out, [I::GlobalGet(global), I::If(BlockType::Empty), I::Unreachable, I::End]);
    }
    ops(
        &mut out,
        [
            I::LocalGet(0),
            I::I32Const(1),
            I::I32GtU,
            I::If(BlockType::Empty),
            I::Unreachable,
            I::End,
        ],
    );
    for n in 0..GLOBALS {
        zero(&mut out, base + n);
    }
    ops(&mut out, [I::LocalGet(0), I::GlobalSet(base), I::I32Const(0), I::End]);
    out
}
fn observer(base: u32, used: Option<u32>) -> Vec<u8> {
    let mut out = vec![0];
    for n in 0..GLOBALS {
        ops(
            &mut out,
            [
                I::LocalGet(0),
                I::I32Const(i32::try_from(n).expect("small selector")),
                I::I32Eq,
                I::If(BlockType::Empty),
                I::GlobalGet(base + n),
                I::Return,
                I::End,
            ],
        );
    }
    for (n, global) in [Some(8), Some(12), Some(2), used].into_iter().enumerate() {
        ops(
            &mut out,
            [
                I::LocalGet(0),
                I::I32Const(i32::try_from(n + 24).expect("small selector")),
                I::I32Eq,
                I::If(BlockType::Empty),
            ],
        );
        ops(&mut out, [global.map_or(I::I32Const(0), I::GlobalGet), I::Return, I::End]);
    }
    ops(&mut out, [I::Unreachable, I::End]);
    out
}
pub(super) fn build(original: &[u8], m: &Model<'_, '_, '_>, row: &Value) -> Vec<u8> {
    let mut sections = transport::sections(original);
    let base =
        transport::counted(&sections.iter().find(|(id, _)| *id == 6).expect("defined globals").1).0;
    let ty = transport::counted(&sections.iter().find(|(id, _)| *id == 1).expect("types").1).0;
    let mut bodies = transport::bodies(original);
    let original_bodies = bodies.clone();
    let count = u32::try_from(bodies.len()).expect("bounded original functions");
    let changed = change(&mut bodies, m, row, base);
    let release = usize::try_from(model::STRING_RELEASE).expect("role");
    bodies[release] = prefix(&bodies[release], &releases(base));
    for (index, (before, after)) in original_bodies.iter().zip(&bodies).enumerate() {
        if index != changed && index != release {
            assert_eq!(before, after, "all other original function bodies remain byte-exact");
        }
    }
    assert_eq!(
        original_bodies.last(),
        bodies.last(),
        "original audited packed invocation body is exact"
    );
    let used = (m.recursive_count() != 0).then(|| m.recursive_used_global());
    bodies.push(configure(base, used));
    bodies.push(observer(base, used));
    for (id, data) in &mut sections {
        match id {
            1 => *data = transport::append(data, 1, &[0x60, 1, 0x7f, 1, 0x7f]),
            3 => {
                let mut tail = Vec::new();
                ty.encode(&mut tail);
                ty.encode(&mut tail);
                *data = transport::append(data, 2, &tail);
            }
            6 => {
                let tail =
                    [0x7f, 1, 0x41, 0, 0x0b].repeat(usize::try_from(GLOBALS).expect("24 globals"));
                *data = transport::append(data, GLOBALS, &tail);
            }
            7 => {
                let mut tail = Vec::new();
                for (name, index) in [("test$configure", count), ("test$observe", count + 1)] {
                    name.encode(&mut tail);
                    tail.push(0);
                    index.encode(&mut tail);
                }
                *data = transport::append(data, 2, &tail);
            }
            10 => *data = transport::code(&bodies),
            _ => {}
        }
    }
    let changed = transport::module(sections);
    // Fixed memory, original data, imports and every noninstrumented section stay exact.
    for (id, data) in transport::sections(original) {
        if ![1, 3, 6, 7, 10].contains(&id) {
            assert_eq!(
                Some(&(id, data)),
                transport::sections(&changed).iter().find(|(other, _)| *other == id)
            );
        }
    }
    changed
}
fn change(bodies: &mut [Vec<u8>], m: &Model<'_, '_, '_>, row: &Value, base: u32) -> usize {
    let name = row["name"].as_str().expect("frozen row");
    if name == "bounds" {
        let op = m
            .program
            .operations()
            .find(|op| op.indexed_loan().is_some())
            .expect("source-authenticated indexed operation");
        let target = usize::try_from(m.source_base).expect("source index") + op.site().function;
        bodies[target] = bounds(&bodies[target], base);
        target
    } else {
        let (target, canonical, logical, second) = match name {
            "allocation" => (model::STRING_CLONE, 1, 2, false),
            "capacity" => (model::STRING_CLONE, 2, 3, false),
            "utf8" => (model::STRING_FROM, 4, 5, true),
            "refcount" => (m.container_base + C::STRONG_CLONE, 3, 4, false),
            _ => panic!("frozen role"),
        };
        let target = usize::try_from(target).expect("source-bound raw runtime role");
        bodies[target] = prefix(&bodies[target], &failure(base, canonical, logical, second));
        target
    }
}
