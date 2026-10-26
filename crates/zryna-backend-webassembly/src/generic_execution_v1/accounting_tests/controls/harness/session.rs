//! Test-only exact32-bit global and memory transport; every actual call result is retained first.
pub(in crate::generic_execution_v1::accounting_tests::controls) struct Session {
    pub(in crate::generic_execution_v1::accounting_tests::controls) store: wasmtime::Store<()>,
    pub(in crate::generic_execution_v1::accounting_tests::controls) instance: wasmtime::Instance,
    pub(in crate::generic_execution_v1::accounting_tests::controls) memory: wasmtime::Memory,
}
impl Session {
    pub(in crate::generic_execution_v1::accounting_tests::controls) fn set(
        &mut self,
        name: &str,
        value: u32,
    ) {
        self.instance
            .get_global(&mut self.store, name)
            .expect("private global")
            .set(&mut self.store, wasmtime::Val::I32(value.cast_signed()))
            .expect("exact u32 seed");
    }
    pub(in crate::generic_execution_v1::accounting_tests::controls) fn get(
        &mut self,
        name: &str,
    ) -> u32 {
        self.instance
            .get_global(&mut self.store, name)
            .expect("private global")
            .get(&mut self.store)
            .i32()
            .expect("u32 global")
            .cast_unsigned()
    }
    pub(in crate::generic_execution_v1::accounting_tests::controls) fn state(
        &mut self,
    ) -> [u32; 4] {
        [self.get("A"), self.get("L"), self.get("S"), self.get("H")]
    }
    pub(in crate::generic_execution_v1::accounting_tests::controls) fn word(
        &mut self,
        address: u32,
    ) -> u32 {
        let mut bytes = [0; 4];
        self.memory
            .read(&self.store, usize::try_from(address).expect("memory address"), &mut bytes)
            .expect("actual memory word");
        u32::from_le_bytes(bytes)
    }
    pub(in crate::generic_execution_v1::accounting_tests::controls) fn put(
        &mut self,
        address: u32,
        value: u32,
    ) {
        self.memory
            .write(
                &mut self.store,
                usize::try_from(address).expect("memory address"),
                &value.to_le_bytes(),
            )
            .expect("private caller word");
    }
    pub(in crate::generic_execution_v1::accounting_tests::controls) fn call1(
        &mut self,
        name: &str,
        arg: u32,
    ) -> u32 {
        let result = self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, name)
            .expect("actual one-lane role")
            .call(&mut self.store, arg.cast_signed());
        self.retain_call(
            name,
            &[arg],
            result.as_ref().ok().map(|v| u64::from(v.cast_unsigned())),
            result.as_ref().err().map(ToString::to_string).as_deref(),
        );
        result.expect("actual role must return; engine error already retained").cast_unsigned()
    }
    pub(in crate::generic_execution_v1::accounting_tests::controls) fn call2(
        &mut self,
        name: &str,
        args: (u32, u32),
    ) -> u32 {
        let result = self
            .instance
            .get_typed_func::<(i32, i32), i32>(&mut self.store, name)
            .expect("actual two-lane role")
            .call(&mut self.store, (args.0.cast_signed(), args.1.cast_signed()));
        self.retain_call(
            name,
            &[args.0, args.1],
            result.as_ref().ok().map(|v| u64::from(v.cast_unsigned())),
            result.as_ref().err().map(ToString::to_string).as_deref(),
        );
        result.expect("actual role must return; engine error already retained").cast_unsigned()
    }
    pub(in crate::generic_execution_v1::accounting_tests::controls) fn wide(
        &mut self,
        name: &str,
        args: (u32, u32),
    ) -> u64 {
        let result = self
            .instance
            .get_typed_func::<(i32, i32), i64>(&mut self.store, name)
            .expect("actual packed-result role")
            .call(&mut self.store, (args.0.cast_signed(), args.1.cast_signed()));
        self.retain_call(
            name,
            &[args.0, args.1],
            result.as_ref().ok().map(|v| v.cast_unsigned()),
            result.as_ref().err().map(ToString::to_string).as_deref(),
        );
        result
            .expect("actual packed role must return; engine error already retained")
            .cast_unsigned()
    }
    fn retain_call(
        &mut self,
        role: &str,
        args: &[u32],
        returned: Option<u64>,
        error: Option<&str>,
    ) {
        static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("m416-wasm-control-call-{}-{sequence}.json", std::process::id()));
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"role":role,"arguments":args,"returned":returned,"engine_error":error,"post":self.state(),"reason":self.get("reason"),"cursor":self.get("cursor"),"attempts":self.get("attempts"),"untrusted_private_harness":true})).expect("actual call receipt")).expect("retain actual before asserting result");
    }
}
