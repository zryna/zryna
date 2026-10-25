//! Test-only exports preserve every original body/type/function byte; they confer no production seal.
pub(super) use super::super::private_controls::harness::transport;
use crate::generic_execution_v1::{
    artifact, code,
    model::{Model, containers as C},
};
use wasm_encoder::{Encode, ExportKind, ExportSection};
mod session;
pub(super) use session::Session;
pub(super) struct Harness {
    engine: wasmtime::Engine,
    module: wasmtime::Module,
    pub(super) directory: std::path::PathBuf,
    pub(super) frame: u32,
    pub(super) out: u32,
    pub(super) scratch: u32,
    pub(super) control_size: u32,
    pub(super) control_alignment: u32,
}
impl Harness {
    pub(super) fn new(m: &Model<'_, '_, '_>, wire: &[u8], label: &str) -> Self {
        Self::build(m, wire, label, None)
    }
    pub(super) fn with_shim(
        m: &Model<'_, '_, '_>,
        wire: &[u8],
        label: &str,
        index: usize,
        status: i32,
        packed: bool,
    ) -> Self {
        Self::build(m, wire, label, Some((index, status, packed)))
    }
    fn build(
        m: &Model<'_, '_, '_>,
        wire: &[u8],
        label: &str,
        shim: Option<(usize, i32, bool)>,
    ) -> Self {
        assert!(m.accounted(), "actual control profile must own A/S/H before private exports");
        let code = code::build(m).expect("actual same-source full code table");
        let original = artifact::encode(m, &code).expect("actual ordinary module");
        artifact::audit(&original, m, &code).expect("independent complete ordinary audit");
        let steps = m
            .functions
            .iter()
            .filter(|f| {
                f.parameters().len() == 0
                    && f.result_type()
                        == zryna_ir::generic_v1::execution_v1::raw::ValueType::Stored(1)
                    && !m.program.export_functions().contains(&f.index())
            })
            .collect::<Vec<_>>();
        assert_eq!(steps.len(), 1, "unique retained zero-source-parameter i32 step");
        let step =
            m.source_base + u32::try_from(steps[0].index()).expect("bounded actual function");
        let control_base =
            m.vector_base() + if m.has_accounted_vec() { 2 + u32::from(m.continuation) } else { 0 };
        let transported = if let Some((index, status, wide)) = shim {
            transport::body_replacement(&original, index, |_| {
                let mut body = crate::generic_execution_v1::bytes::Bytes::new();
                body.extend(&[0]).expect("shim locals");
                let result = if wide {
                    wasm_encoder::Instruction::I64Const(i64::from(status) << 32)
                } else {
                    wasm_encoder::Instruction::I32Const(status)
                };
                body.op(&result).expect("synthetic role status");
                body.op(&wasm_encoder::Instruction::End).expect("end");
                body.bytes
            })
        } else {
            original.clone()
        };
        let exported =
            transport::replace(&transported, 7, |_| export_section(m, step, control_base));
        assert!(
            artifact::audit(&exported, m, &code).is_err(),
            "test extra exports are never ordinary authority"
        );
        let original_bodies = transport::bodies(&original);
        let actual_bodies = transport::bodies(&exported);
        assert_eq!(original_bodies.len(), actual_bodies.len());
        for (index, (before, after)) in original_bodies.iter().zip(&actual_bodies).enumerate() {
            if shim.is_none_or(|(changed, _, _)| changed != index) {
                assert_eq!(before, after, "every unchanged actual body byte preserved");
            }
        }
        assert_eq!(
            transport::type_map(&original),
            transport::type_map(&exported),
            "type/function sections preserved"
        );
        let directory = std::env::temp_dir()
            .join(format!("m416-wasm-control-private-{}-{label}", std::process::id()));
        std::fs::create_dir(&directory).expect("fresh actual harness directory");
        for (name, bytes) in [
            ("original-audited.wasm", original.as_slice()),
            ("harness-untrusted.wasm", exported.as_slice()),
            ("program.zir", wire),
        ] {
            std::fs::write(directory.join(name), bytes).expect("retain actual input");
        }
        std::fs::write(directory.join("body-preservation.json"),serde_json::to_vec_pretty(&serde_json::json!({"original":transport::hash(&original),"harness":transport::hash(&exported),"bodies":transport::bodies(&original),"type_function_map":transport::type_map(&original),"step":step,"control_base":control_base,"untrusted_exports":true,"synthetic_status_shim":shim})).expect("mapping receipt")).expect("retain before execution");
        let mut config = wasmtime::Config::new();
        config.epoch_interruption(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let module =
            wasmtime::Module::new(&engine, &exported).expect("valid untrusted export harness");
        let shared = m
            .types
            .iter()
            .find(|ty| ty.category() == zryna_layout::TypeCategory::Shared)
            .expect("actual Shared type");
        let layout =
            m.control(shared.id().index()).expect("same-program Linear32 control geometry");
        Self {
            engine,
            module,
            directory,
            frame: m.memory.claims().frames_start,
            out: m.memory.claims().scratch_start + 16,
            scratch: m.memory.claims().scratch_start,
            control_size: u32::try_from(layout.size()).expect("Linear32 size"),
            control_alignment: u32::try_from(layout.alignment()).expect("Linear32 alignment"),
        }
    }
    pub(super) fn engine(&self) -> wasmtime::Engine {
        self.engine.clone()
    }
    pub(super) fn session(&self) -> Session {
        let mut store = wasmtime::Store::new(&self.engine, ());
        store.set_epoch_deadline(1);
        let instance = wasmtime::Instance::new(&mut store, &self.module, &[])
            .expect("untrusted body-preserving instance");
        let memory = instance.get_memory(&mut store, "memory").expect("test-only memory export");
        Session { store, instance, memory }
    }
    pub(super) fn retain(&self, name: &str, actual: &serde_json::Value) {
        std::fs::write(
            self.directory.join(format!("{name}.before-checks.json")),
            serde_json::to_vec_pretty(actual).expect("bounded actual observation"),
        )
        .expect("retain before oracle checks");
    }
}

fn export_section(m: &Model<'_, '_, '_>, step: u32, control_base: u32) -> Vec<u8> {
    let mut exports = ExportSection::new();
    for (index, export) in m.program.scalar_abi().exports().enumerate() {
        exports.export(
            export.webassembly_name().as_str(),
            ExportKind::Func,
            m.wrapper_base + u32::try_from(index).expect("bounded exports"),
        );
    }
    for (name, index) in
        [("A", 10), ("L", 8), ("S", 11), ("H", 12), ("cursor", 0), ("reason", 1), ("attempts", 5)]
    {
        exports.export(name, ExportKind::Global, index);
    }
    exports.export("sourceStep", ExportKind::Func, step);
    exports.export("controlAllocate", ExportKind::Func, control_base);
    exports.export("rawControlAllocate", ExportKind::Func, m.container_base + C::ALLOCATE_RAW);
    for (name, role) in [
        ("strongClone", C::STRONG_CLONE),
        ("downgrade", C::DOWNGRADE),
        ("weakClone", C::WEAK_CLONE),
        ("begin", C::STRONG_BEGIN),
        ("finish", C::STRONG_FINISH),
        ("weakRelease", C::WEAK_RELEASE),
    ] {
        exports.export(name, ExportKind::Func, m.container(role));
    }
    if m.continuation {
        exports.export("upgrade", ExportKind::Func, m.container(C::WEAK_UPGRADE));
    }
    exports.export("memory", ExportKind::Memory, 0);
    let mut bytes = Vec::new();
    exports.encode(&mut bytes);
    let mut reader = wasmparser::BinaryReader::new(&bytes, 0);
    let size = reader.read_var_u32().expect("section size");
    let start = usize::try_from(reader.original_position()).expect("bounded section cursor");
    assert_eq!(usize::try_from(size).expect("bounded section"), bytes.len() - start);
    bytes[start..].to_vec()
}
