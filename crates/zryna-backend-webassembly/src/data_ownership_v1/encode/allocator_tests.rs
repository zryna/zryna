use wasm_encoder::{
    CodeSection, ConstExpr, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    GlobalType, Instruction, Module, TypeSection, ValType,
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
    let bytes = serde_json::to_string(&module.finish()).expect("fixed test authority");
    let limit = super::MEMORY_PAGES * 65536;
    let script = format!(
        r"
const bytes = new Uint8Array({bytes});
const rows = [];
for (const size of [67108863, 67108864, 67108865, 2147483647, 2147483648, -1, {limit} - 65536, {limit} - 65535]) {{
  const {{instance}} = await WebAssembly.instantiate(bytes);
  const e = instance.exports;
  rows.push([e.allocate(size), e.status.value, e.arena.value]);
}}
const {{instance}} = await WebAssembly.instantiate(bytes);
const e = instance.exports;
const failed = [e.allocate(67108865), e.status.value, e.arena.value];
e.status.value = 0;
e.arena.value = 65536;
const recovered = [e.allocate(8), e.status.value, e.arena.value];
rows.push([failed, recovered]);
console.log(JSON.stringify(rows));
"
    );
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("fixed test authority");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let rows: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("fixed test authority");
    assert_eq!(
        rows,
        serde_json::json!([
            [0, 2, 65536],
            [0, 2, 65536],
            [0, 2, 65536],
            [0, 2, 65536],
            [0, 3, 65536],
            [0, 3, 65536],
            [65536, 0, limit],
            [0, 2, 65536],
            [[0, 2, 65536], [65536, 0, 65544]]
        ])
    );
}

#[test]
fn string_concat_size_check_includes_record_and_preserves_universal_limit() {
    let mut module = Module::new();
    let mut types = TypeSection::new();
    types.ty().function([ValType::I32, ValType::I32], [ValType::I32]);
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
    exports.export("stringSize", ExportKind::Func, 0);
    exports.export("status", ExportKind::Global, 1);
    module.section(&exports);
    let mut body = Function::new([(1, ValType::I32)]);
    body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    super::values::string_allocation_size(0, 1, 2, &mut body)
        .expect("fixed String allocation limits");
    body.instruction(&Instruction::End);
    body.instruction(&Instruction::LocalGet(2));
    body.instruction(&Instruction::End);
    let mut code = CodeSection::new();
    code.function(&body);
    module.section(&code);
    let bytes = serde_json::to_string(&module.finish()).expect("fixed test authority");
    let payload_limit = super::MEMORY_PAGES * 65_536
        - u64::try_from(super::observation::ARENA_START).expect("arena start")
        - 12;
    let script = format!(
        r"
const bytes = new Uint8Array({bytes});
const rows = [];
for (const pair of [[{payload_limit}, 0], [{payload_limit} + 1, 0], [67108865, 0], [2147483647, 0], [2147483647, 1], [-1, 1]]) {{
  const {{instance}} = await WebAssembly.instantiate(bytes);
  const e = instance.exports;
  rows.push([e.stringSize(...pair), e.status.value]);
}}
console.log(JSON.stringify(rows));
"
    );
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("fixed test authority");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let rows: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("fixed test authority");
    assert_eq!(
        rows,
        serde_json::json!([
            [payload_limit, 0],
            [payload_limit + 1, 2],
            [67_108_865, 2],
            [2_147_483_647_u64, 2],
            [-2_147_483_648_i64, 3],
            [0, 3]
        ])
    );
}
