//! Real Wasm metering under the caller's unchanged production engine and fuel budget.
//! This raw core prerequisite bypasses public artifact admission.

use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction,
    Module as EncodedModule, TypeSection,
};
use wasmtime::{Engine, Instance, Module, Store, StoreLimits, Trap};

pub(super) fn measured_production_exhaustion(engine: &Engine, limits: StoreLimits, budget: u64) {
    assert_eq!(budget, 100_000, "bind the advertised unchanged production ceiling");
    let mut encoded = EncodedModule::new();
    let mut types = TypeSection::new();
    types.ty().function([], []);
    encoded.section(&types);
    let mut functions = FunctionSection::new();
    functions.function(0);
    encoded.section(&functions);
    let mut exports = ExportSection::new();
    exports.export("unit", ExportKind::Func, 0);
    encoded.section(&exports);
    let mut function = Function::new([]);
    function.instruction(&Instruction::End);
    let mut code = CodeSection::new();
    code.function(&function);
    encoded.section(&code);
    let module = Module::new(engine, encoded.finish()).expect("independent tiny core fixture");
    let mut store = Store::new(engine, limits);
    store.limiter(|limits| limits);
    store.set_fuel(budget).expect("initial production fuel, never replenished");
    store.set_epoch_deadline(1);
    let instance = Instance::new(&mut store, &module, &[]).expect("real core instantiation");
    let unit = instance.get_typed_func::<(), ()>(&mut store, "unit").expect("real typed call");
    let before = store.get_fuel().expect("meter before first call");
    assert_eq!(before, budget, "fixture instantiation must not spend guest execution fuel");
    unit.call(&mut store, ()).expect("first measured call");
    let after = store.get_fuel().expect("meter after first call");
    let charged = before.checked_sub(after).expect("fuel cannot increase");
    assert_eq!(charged, 1, "measure one charged unit; source opcode count is not an oracle");
    let mut completed = 1;
    for remaining in (1..after).rev() {
        unit.call(&mut store, ()).expect("measured call leaves positive fuel");
        completed += 1;
        assert_eq!(store.get_fuel().expect("actual remaining meter"), remaining);
    }
    assert_eq!(completed, budget - 1);
    assert_eq!(store.get_fuel().expect("meter before final charged call"), 1);
    // Pinned Wasmtime checks exhaustion when a charged block reaches zero.
    // Spending the final unit traps; it does not complete another unit call.
    let error = unit.call(&mut store, ()).expect_err("final charged unit reaches exhaustion");
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
    assert_eq!(store.get_fuel().expect("exact exhaustion meter"), 0);
    let error = unit.call(&mut store, ()).expect_err("next call with exhausted budget rejects");
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
    assert_eq!(store.get_fuel().expect("rejected call does not replenish quota"), 0);
    println!(
        "production fuel={budget}; measured unit={charged}; completed={completed}; final charged call=OutOfFuel; next call=OutOfFuel; remaining=0"
    );
    drop(store);
}
