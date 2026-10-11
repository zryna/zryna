//! Backtrace frame cap is diagnostic output truncation, not guest call admission.

use super::super::envelope;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction,
    Module as EncodedModule, TypeSection,
};
use wasmtime::{Instance, Module, Store, Trap, WasmBacktrace};

#[test]
fn private_h1_one_frame_backtrace_retains_inner_trap_and_truncates_extra_caller() {
    let engine = envelope::engine().expect("unchanged one-frame H1 engine");
    let mut encoded = EncodedModule::new();
    let mut types = TypeSection::new();
    types.ty().function([], []);
    encoded.section(&types);
    let mut functions = FunctionSection::new();
    functions.function(0).function(0);
    encoded.section(&functions);
    let mut exports = ExportSection::new();
    exports.export("inner", ExportKind::Func, 0);
    exports.export("outer", ExportKind::Func, 1);
    encoded.section(&exports);
    let mut code = CodeSection::new();
    let mut inner = Function::new([]);
    inner.instruction(&Instruction::Unreachable);
    inner.instruction(&Instruction::End);
    code.function(&inner);
    let mut outer = Function::new([]);
    outer.instruction(&Instruction::Call(0));
    outer.instruction(&Instruction::End);
    code.function(&outer);
    encoded.section(&code);
    let module = Module::new(&engine, encoded.finish()).expect("private nested trap fixture");
    for export in ["inner", "outer"] {
        let mut store = Store::new(&engine, envelope::limits());
        store.limiter(|limits| limits);
        store.set_fuel(envelope::FUEL).expect("unchanged production fuel");
        store.set_epoch_deadline(1);
        let instance = Instance::new(&mut store, &module, &[]).expect("private core instance");
        let function = instance.get_typed_func::<(), ()>(&mut store, export).expect("real call");
        let error = function.call(&mut store, ()).expect_err("same actual inner trap");
        assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::UnreachableCodeReached));
        let trace = error.downcast_ref::<WasmBacktrace>().expect("actual bounded backtrace");
        assert_eq!(trace.frames().len(), 1, "additional caller never enlarges output past one");
        assert_eq!(trace.frames()[0].func_index(), 0, "retain the actual trapping inner function");
        drop(store);
    }
    super::fresh_recovery();
}
