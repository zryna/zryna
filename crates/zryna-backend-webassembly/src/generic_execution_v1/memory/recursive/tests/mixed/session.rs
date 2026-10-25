//! Test-only whole-case fuel; no process/OS dependency and no product source quota.
use wasmtime::{Config, Engine, Instance, Module, Store, TypedFunc};
const WHOLE_CASE_FUEL: u64 = 100_000_000;
pub(super) struct Session {
    store: Store<()>,
    ordinary: TypedFunc<(), i32>,
    entry: TypedFunc<(), i32>,
    getter: TypedFunc<i32, i32>,
}
fn module(engine: &Engine, bytes: &[u8], observed: bool) -> Module {
    let module = Module::new(engine, bytes).expect("actual independently audited bytes");
    assert_eq!(module.imports().len(), 0);
    let names = module
        .exports()
        .map(|export| {
            assert!(matches!(export.ty(), wasmtime::ExternType::Func(_)));
            export.name().to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(names, if observed { vec!["entry", "$zryna$observation"] } else { vec!["entry"] });
    module
}
impl Session {
    pub(super) fn new(ordinary: &[u8], observed: &[u8]) -> Self {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).expect("portable host containment engine");
        let ordinary = module(&engine, ordinary, false);
        let observed = module(&engine, observed, true);
        let mut store = Store::new(&engine, ());
        // Frozen before any invocation; shared across BOTH instances and all35 faults/retries.
        // Never refilled. Exhaustion fails this test, not a language resource result.
        store.set_fuel(WHOLE_CASE_FUEL).expect("one whole-case test guard");
        let plain = Instance::new(&mut store, &ordinary, &[]).expect("ordinary owned memory");
        let marked = Instance::new(&mut store, &observed, &[]).expect("observed owned memory");
        Self {
            ordinary: plain.get_typed_func(&mut store, "entry").expect("ordinary scalar ABI"),
            entry: marked.get_typed_func(&mut store, "entry").expect("observed scalar ABI"),
            getter: marked
                .get_typed_func(&mut store, "$zryna$observation")
                .expect("explicit test-only observer ABI"),
            store,
        }
    }
    pub(super) fn ordinary(&mut self) -> wasmtime::Result<i32> {
        self.ordinary.call(&mut self.store, ())
    }
    pub(super) fn entry(&mut self) -> wasmtime::Result<i32> {
        self.entry.call(&mut self.store, ())
    }
    pub(super) fn get(&mut self, argument: u32) -> i32 {
        self.getter
            .call(&mut self.store, argument.cast_signed())
            .expect("getter succeeds; host containment failure fails test")
    }
    pub(super) fn funded(&self) {
        assert!(
            self.store.get_fuel().expect("test guard remains enabled") > 0,
            "entire mixed case must finish within frozen test-only fuel"
        );
    }
}
