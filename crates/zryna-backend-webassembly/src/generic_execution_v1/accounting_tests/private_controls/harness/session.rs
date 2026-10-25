//! Actual untrusted harness instance with explicit private global/record access.
pub(in crate::generic_execution_v1::accounting_tests) struct Session {
    pub(in crate::generic_execution_v1::accounting_tests) store: wasmtime::Store<()>,
    pub(in crate::generic_execution_v1::accounting_tests) instance: wasmtime::Instance,
    pub(in crate::generic_execution_v1::accounting_tests) memory: wasmtime::Memory,
}
static CALL_RECEIPT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
impl Session {
    pub(in crate::generic_execution_v1::accounting_tests) fn set(
        &mut self,
        name: &str,
        value: u32,
    ) {
        self.instance
            .get_global(&mut self.store, name)
            .expect("private global")
            .set(&mut self.store, wasmtime::Val::I32(value.cast_signed()))
            .expect("seed exact i32 context");
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn get(&mut self, name: &str) -> u32 {
        self.instance
            .get_global(&mut self.store, name)
            .expect("private global")
            .get(&mut self.store)
            .i32()
            .expect("i32 global")
            .cast_unsigned()
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn state(&mut self) -> serde_json::Value {
        serde_json::json!({"A":self.get("A"),"L":self.get("L"),"S":self.get("S"),"H":self.get("H")})
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn call3(
        &mut self,
        name: &str,
        args: (i32, i32, i32),
    ) -> i32 {
        let result = self
            .instance
            .get_typed_func::<(i32, i32, i32), i32>(&mut self.store, name)
            .expect("actual three-lane role")
            .call(&mut self.store, args);
        self.returned(name, &[args.0, args.1, args.2], result)
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn call4(
        &mut self,
        args: (i32, i32, i32, i32),
    ) -> i32 {
        let result = self
            .instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&mut self.store, "clone")
            .expect("actual clone role")
            .call(&mut self.store, args);
        self.returned("clone", &[args.0, args.1, args.2, args.3], result)
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn call4_named(
        &mut self,
        name: &str,
        args: (i32, i32, i32, i32),
    ) -> i32 {
        let result = self
            .instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&mut self.store, name)
            .expect("actual four-lane role")
            .call(&mut self.store, args);
        self.returned(name, &[args.0, args.1, args.2, args.3], result)
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn call2(
        &mut self,
        name: &str,
        args: (i32, i32),
    ) -> i32 {
        let result = self
            .instance
            .get_typed_func::<(i32, i32), i32>(&mut self.store, name)
            .expect("actual retained clone helper")
            .call(&mut self.store, args);
        self.returned(name, &[args.0, args.1], result)
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn drop_at(
        &mut self,
        name: &str,
        address: i32,
    ) {
        let result = self
            .instance
            .get_typed_func::<i32, ()>(&mut self.store, name)
            .expect("actual retained drop helper")
            .call(&mut self.store, address)
            .map(|()| 0);
        assert_eq!(self.returned(name, &[address], result), 0);
    }
    fn returned(&mut self, role: &str, args: &[i32], result: wasmtime::Result<i32>) -> i32 {
        let sequence = CALL_RECEIPT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("m416-wasm-private-string-call-{}-{sequence}.json", std::process::id()));
        let receipt = serde_json::json!({"role":role,"arguments":args,"returned_status":result.as_ref().ok().copied(),"engine_error":result.as_ref().err().map(ToString::to_string),"engine_trap":result.as_ref().err().and_then(|error|error.downcast_ref::<wasmtime::Trap>()).map(|trap|format!("{trap:?}")),"post":self.state(),"test":std::thread::current().name(),"untrusted_private_harness":true,"source_semantic_result":false});
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&receipt).expect("bounded private call receipt"),
        )
        .expect("retain actual status/trap before assertions");
        result.unwrap_or_else(|error| {
            panic!("private helper engine failure; retained {path:?}: {error:#}")
        })
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn record(
        &mut self,
        address: usize,
    ) -> (i32, i32, i32) {
        let mut bytes = [0; 12];
        self.memory.read(&self.store, address, &mut bytes).expect("checked record");
        (
            i32::from_le_bytes(bytes[0..4].try_into().expect("word")),
            i32::from_le_bytes(bytes[4..8].try_into().expect("word")),
            i32::from_le_bytes(bytes[8..12].try_into().expect("word")),
        )
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn poison(&mut self, address: usize) {
        self.memory.write(&mut self.store, address, &[0xa5; 12]).expect("test output poison");
    }
}
