use wasm_encoder::{
    CodeSection, ConstExpr, ExportKind, ExportSection, FunctionSection, GlobalSection, GlobalType,
    Module, TypeSection, ValType,
};

#[test]
fn allocator_capacity_and_arena_exhaustion_keep_distinct_typed_statuses() {
    let mut module = Module::new();
    let mut types = TypeSection::new();
    types.ty().function([ValType::I32], [ValType::I32]);
    module.section(&types);
    let mut functions = FunctionSection::new();
    functions.function(0);
    module.section(&functions);
    let mut globals = GlobalSection::new();
    for value in [65536, 0] {
        globals.global(
            GlobalType { val_type: ValType::I32, mutable: true, shared: false },
            &ConstExpr::i32_const(value),
        );
    }
    module.section(&globals);
    let mut exports = ExportSection::new();
    exports.export("allocate", ExportKind::Func, 0);
    exports.export("arena", ExportKind::Global, 0);
    exports.export("status", ExportKind::Global, 1);
    module.section(&exports);
    let mut code = CodeSection::new();
    code.function(&super::allocator());
    module.section(&code);
    let bytes = serde_json::to_string(&module.finish()).unwrap();
    let limit = super::MEMORY_PAGES * 65536;
    let script = format!(
        r#"
const bytes = new Uint8Array({bytes});
const rows = [];
for (const size of [67108865, -1, 67108864, {limit} - 65536, {limit} - 65535]) {{
  const {{instance}} = await WebAssembly.instantiate(bytes);
  const e = instance.exports;
  rows.push([e.allocate(size), e.status.value, e.arena.value]);
}}
console.log(JSON.stringify(rows));
"#
    );
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let rows: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        rows,
        serde_json::json!([
            [0, 3, 65536],
            [0, 3, 65536],
            [0, 2, 65536],
            [65536, 0, limit],
            [0, 2, 65536]
        ])
    );
}
