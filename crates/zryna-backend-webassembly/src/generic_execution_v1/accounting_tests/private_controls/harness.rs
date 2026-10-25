//! Explicit untrusted export harness, preserving every original code body/type/function map byte.
use crate::generic_execution_v1::{artifact, code, model::Model};
use wasm_encoder::{Encode, ExportKind, ExportSection};
mod session;
pub(in crate::generic_execution_v1::accounting_tests) mod transport;
use session::Session;
pub(in crate::generic_execution_v1::accounting_tests) use transport::body_replacement;
use transport::{bodies, hash, replace, type_map};
pub(in crate::generic_execution_v1::accounting_tests) struct Harness {
    pub(in crate::generic_execution_v1::accounting_tests) engine: wasmtime::Engine,
    pub(in crate::generic_execution_v1::accounting_tests) module: wasmtime::Module,
    pub(in crate::generic_execution_v1::accounting_tests) directory: std::path::PathBuf,
}
impl Harness {
    pub(in crate::generic_execution_v1::accounting_tests) fn new(
        m: &Model<'_, '_, '_>,
        wire: &[u8],
        label: &str,
        shim: Option<(usize, i32)>,
    ) -> Self {
        let code = code::build(m).expect("actual recipe table");
        let original = artifact::encode(m, &code).expect("actual bytes");
        artifact::audit(&original, m, &code).expect("independently audited original");
        let original_bodies = bodies(&original);
        let mutated = if let Some((index, status)) = shim {
            body_replacement(&original, index, |_| {
                let mut bytes = crate::generic_execution_v1::bytes::Bytes::new();
                bytes.extend(&[0]).expect("locals");
                bytes.op(&wasm_encoder::Instruction::I32Const(status)).expect("synthetic status");
                bytes.op(&wasm_encoder::Instruction::End).expect("end");
                bytes.bytes
            })
        } else {
            original.clone()
        };
        let exported = replace(&mutated, 7, |_| export_section(m));
        assert!(
            artifact::audit(&exported, m, &code).is_err(),
            "harness extra exports cannot be production authority"
        );
        let original_type_map = type_map(&original);
        let harness_type_map = type_map(&exported);
        assert_eq!(
            original_type_map, harness_type_map,
            "exact type/function-index section bytes preserved"
        );
        let final_bodies = bodies(&exported);
        assert_eq!(original_bodies.len(), final_bodies.len());
        for (index, (before, after)) in original_bodies.iter().zip(&final_bodies).enumerate() {
            if shim.is_none_or(|(changed, _)| changed != index) {
                assert_eq!(before, after, "actual body {index} preserved");
            }
        }
        let directory = std::env::temp_dir()
            .join(format!("m416-wasm-string-private-{}-{label}", std::process::id()));
        std::fs::create_dir(&directory).expect("new private evidence directory");
        std::fs::write(directory.join("original-audited.wasm"), &original)
            .expect("retain original");
        std::fs::write(directory.join("harness-untrusted.wasm"), &exported)
            .expect("retain untrusted harness");
        std::fs::write(directory.join("program.zir"), wire).expect("genuine source seal wire");
        let receipt = serde_json::json!({"original_sha256":hash(&original),"harness_sha256":hash(&exported),"original_type_function_map":original_type_map,"harness_type_function_map":harness_type_map,"accounting_base":m.accounting_base(),"global_count":m.global_count(),"original_body_sha256":original_bodies,"harness_body_sha256":final_bodies,"synthetic_raw_status_shim":shim,"production_audit_accepts_harness":false,"seeded_private_evidence":true});
        std::fs::write(
            directory.join("body-preservation.json"),
            serde_json::to_vec_pretty(&receipt).expect("receipt"),
        )
        .expect("retain mapping");
        let mut config = wasmtime::Config::new();
        config.epoch_interruption(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let module =
            wasmtime::Module::new(&engine, &exported).expect("valid untrusted private harness");
        Self { engine, module, directory }
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn session(&self) -> Session {
        let mut store = wasmtime::Store::new(&self.engine, ());
        store.set_epoch_deadline(1);
        let instance = wasmtime::Instance::new(&mut store, &self.module, &[])
            .expect("untrusted private harness instance");
        let memory = instance.get_memory(&mut store, "memory").expect("test-only memory view");
        Session { store, instance, memory }
    }
    pub(in crate::generic_execution_v1::accounting_tests) fn retain(
        &self,
        name: &str,
        value: &serde_json::Value,
    ) {
        std::fs::write(
            self.directory.join(format!("{name}.json")),
            serde_json::to_vec_pretty(value).expect("private result summary"),
        )
        .expect("retain before assertions");
    }
}

fn export_section(m: &Model<'_, '_, '_>) -> Vec<u8> {
    let mut exports = ExportSection::new();
    for (index, export) in m.program.scalar_abi().exports().enumerate() {
        exports.export(
            export.webassembly_name().as_str(),
            ExportKind::Func,
            m.wrapper_base + u32::try_from(index).expect("fixed export"),
        );
    }
    for (name, index) in
        [("A", 10), ("L", 8), ("S", 11), ("H", 12), ("cursor", 0), ("reason", 1), ("attempts", 5)]
    {
        exports.export(name, ExportKind::Global, index);
    }
    for (name, offset) in [("from", 0), ("clone", 1), ("release", 2)] {
        exports.export(name, ExportKind::Func, m.accounting_base() + offset);
    }
    if m.concat {
        exports.export("concat", ExportKind::Func, m.accounting_base() + 3);
    }
    if m.types.iter().any(|ty| ty.category() == zryna_layout::TypeCategory::Vec) {
        use crate::generic_execution_v1::model::containers as C;
        for (name, role) in [("vecAllocate", C::VEC_ALLOCATE), ("vecRelease", C::VEC_RELEASE)] {
            exports.export(name, ExportKind::Func, m.container(role));
        }
        if m.continuation {
            exports.export("vecReserve", ExportKind::Func, m.container(C::VEC_RESERVE));
        }
        for (index, ty) in m
            .types
            .iter()
            .enumerate()
            .filter(|(_, ty)| ty.category() == zryna_layout::TypeCategory::Vec)
        {
            let index = u32::try_from(index).expect("retained type bound");
            exports.export(&format!("vecDrop{index}"), ExportKind::Func, m.drop_index(index));
            if m.clones[usize::try_from(index).expect("checked index")].is_some() {
                exports.export(&format!("vecClone{index}"), ExportKind::Func, m.clone_index(index));
            }
            assert_eq!(index, ty.id().index());
        }
    }
    exports.export("memory", ExportKind::Memory, 0);
    let mut bytes = Vec::new();
    exports.encode(&mut bytes);
    let mut reader = wasmparser::BinaryReader::new(&bytes, 0);
    let length = reader.read_var_u32().expect("section size");
    let start = usize::try_from(reader.original_position()).expect("bounded export section offset");
    assert_eq!(usize::try_from(length).expect("fixed length"), bytes.len() - start);
    bytes[start..].to_vec()
}
