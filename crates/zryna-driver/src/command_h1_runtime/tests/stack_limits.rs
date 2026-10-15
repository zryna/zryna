//! Private recursive core trap; no exact native-frame byte oracle or public recursion claim.

use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction,
    Module as EncodedModule, TypeSection,
};
use wasmtime::{Instance, Module, Store, Trap};

#[test]
fn private_h1_fixed_stack_traps_recursive_core_with_fuel_remaining_and_recovers() {
    let engine = super::super::envelope::engine().expect("unchanged H1 stack configuration");
    let mut encoded = EncodedModule::new();
    let mut types = TypeSection::new();
    types.ty().function([], []);
    encoded.section(&types);
    let mut functions = FunctionSection::new();
    functions.function(0);
    encoded.section(&functions);
    let mut exports = ExportSection::new();
    exports.export("recurse", ExportKind::Func, 0);
    encoded.section(&exports);
    let mut function = Function::new([]);
    function.instruction(&Instruction::Call(0));
    function.instruction(&Instruction::End);
    let mut code = CodeSection::new();
    code.function(&function);
    encoded.section(&code);
    let module = Module::new(&engine, encoded.finish()).expect("private recursive core fixture");
    let mut store = Store::new(&engine, super::super::envelope::limits());
    store.limiter(|limits| limits);
    store.set_fuel(super::super::envelope::FUEL).expect("unchanged production fuel");
    store.set_epoch_deadline(1);
    let instance = Instance::new(&mut store, &module, &[]).expect("private core instantiation");
    let recurse = instance.get_typed_func::<(), ()>(&mut store, "recurse").expect("recursive call");
    let error = recurse.call(&mut store, ()).expect_err("fixed stack prevents unbounded recursion");
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::StackOverflow));
    assert!(store.get_fuel().expect("remaining actual meter") > 0, "stack traps before fuel");
    drop(store);
    super::fresh_recovery();
}
