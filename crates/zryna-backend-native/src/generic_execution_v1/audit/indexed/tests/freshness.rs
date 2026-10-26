//! Valid actual SSA from a genuine indexed body may not borrow stale preflight loads.
use super::*;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_codegen::settings;
mod fixture;
#[test]
fn actual_indexed_guard_rejects_earlier_vec_data_length_index_or_address_authority() {
    for mutation in 0..4 {
        fixture::with_graph(|c, f, view, source, status, published| {
            check(c, f, view, source, status, published).expect("genuine pristine current guard");
            let source = stale(c, f, view, source, status, published, mutation);
            cranelift_codegen::verify_function(f, &settings::Flags::new(settings::builder()))
                .expect("hostile substitution remains valid SSA before audit assertion");
            assert!(
                check(c, f, view, source, status, published).is_err(),
                "current guard must not trust earlier Vec data/length, index load or address"
            );
        });
    }
}
fn stale(
    c: &Frame<'_, '_, '_>,
    f: &mut Function,
    view: VerifiedIndexedLoan<'_, '_>,
    source: Value,
    status: Value,
    published: Inst,
    mutation: u8,
) -> Value {
    let old_bounds = bounds(f, status).expect("actual predicate");
    let compare =
        f.dfg.value_def(args(f, status, O::Select, 3).expect("status")[0]).inst().expect("compare");
    let extend = f.dfg.value_def(old_bounds[0]).inst().expect("actual unsigned extension");
    let D::Store { args: stored, .. } = f.dfg.insts[published] else { panic!("publication") };
    let pointer = f.dfg.value_def(stored[0]).inst().expect("pointer add");
    let pointer_data = f.dfg.inst_args(pointer)[0];
    let data_load = f.dfg.value_def(pointer_data).inst().expect("actual data load");
    let length_load = f.dfg.value_def(old_bounds[1]).inst().expect("actual initialized length");
    let entry = f.layout.entry_block().expect("entry");
    let before = f.layout.last_inst(entry).expect("entry jump before Vec initialization");
    let record = if mutation == 2 {
        c.f.value(view.index().id()).expect("index record")
    } else {
        c.f.place(view.base_place().expect("genuine rooted Vec").id()).expect("actual Vec record")
    };
    let (old_address, old_load) = {
        let mut cursor = FuncCursor::new(f);
        cursor.goto_inst(before);
        let offset = cursor.ins().iconst(types::I64, i64::from(record.offset()));
        let address = cursor.ins().iadd(c.base, offset);
        let load = cursor.ins().load(
            if mutation == 2 { types::I32 } else { types::I64 },
            MemFlagsData::new(),
            address,
            if mutation == 1 { 8 } else { 0 },
        );
        (address, load)
    };
    if mutation == 2 {
        f.dfg.inst_args_mut(extend)[0] = old_load;
        return source;
    }
    if let D::Load { arg, .. } = &mut f.dfg.insts[data_load] {
        *arg = old_address;
    }
    if let D::Load { arg, .. } = &mut f.dfg.insts[length_load] {
        *arg = old_address;
    }
    if mutation == 0 {
        f.dfg.inst_args_mut(pointer)[0] = old_load;
    }
    if mutation == 1 {
        let D::IntCompare { args, .. } = &mut f.dfg.insts[compare] else { panic!("compare") };
        args[1] = old_load;
    }
    old_address
}
