//! Private actual separately compiled `SQLite` snapshot; not the 8 MiB adapter seal.
use super::*;
use crate::native::{self, NativeProcessLimits, NativeStage, ProcessPhase};
use object::{Object as _, ObjectSymbol as _};
use std::{
    ffi::OsString,
    sync::{Arc, OnceLock},
};

pub(super) struct Provider {
    object: Arc<[u8]>,
    header: Arc<[u8]>,
    object_sha256: [u8; 32],
    tools: native::LinuxX8664LinkToolchain,
}
static ORDINARY: OnceLock<Provider> = OnceLock::new();
static SANITIZED: OnceLock<Provider> = OnceLock::new();

impl Provider {
    pub(super) fn get(sanitized: bool) -> &'static Self {
        let slot = if sanitized { &SANITIZED } else { &ORDINARY };
        slot.get_or_init(|| Self::build(sanitized))
    }
    fn build(sanitized: bool) -> Self {
        let sources = source_input::capture();
        let fixture = Fixture::new();
        let stage = stage::Stage::new(&fixture);
        let tools = native::discover_linux_native_toolchain(NativeProcessLimits::default())
            .expect("supported pinned C toolchain");
        let source =
            stage.capability_file_path("invocation.c").expect("provider source capability");
        let object = stage.capability_file_path("runtime.o").expect("provider object capability");
        // The authenticated amalgamation includes the upstream header inline.
        stage.write_input(&stage.harness, &sources.source).expect("bounded amalgamation snapshot");
        let mut args: Vec<OsString> = [
            "-std=c11",
            "-O0",
            "-g0",
            "-fno-ident",
            "-fno-pie",
            "-fno-pic",
            "-fcf-protection=none",
            "-fno-unwind-tables",
            "-fno-asynchronous-unwind-tables",
            "-DSQLITE_THREADSAFE=0",
            "-DSQLITE_OMIT_LOAD_EXTENSION",
            "-c",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        if sanitized {
            args.extend(SANITIZERS.map(Into::into));
        }
        args.extend([source.into_os_string(), "-o".into(), object.clone().into_os_string()]);
        compile(&tools, &stage, &args, "compile-sqlite");
        // A separate private observation cap reuses the existing 32 MiB numeric limit.
        // The provider is never submitted to the unchanged 8 MiB foreign adapter audit.
        let bytes = source_input::bounded_read(&object, native::MAX_NATIVE_EXECUTABLE_BYTES);
        check_provider(&bytes);
        let digest = sha(&bytes);
        eprintln!(
            "417-sqlite-provider {}",
            serde_json::json!({
            "sanitized":sanitized,"source_sha256":hex(&sha(&sources.source)),
            "header_sha256":hex(&sha(&sources.header)),"object_sha256":hex(&digest),
            "object_bytes":bytes.len(),"gcc_version":tools.gcc_version(),
            "linker_version":tools.linker_version(),
            "driver_path_bytes":tools.driver.as_os_str().as_encoded_bytes(),
            "linker_path_bytes":tools.linker.as_os_str().as_encoded_bytes(),
            "driver_metadata_identity":format!("{:?}",tools.driver_identity),
            "linker_metadata_identity":format!("{:?}",tools.linker_identity),
            "production_admission":false,
            "aggregate_material_qualification":false})
        );
        drop(stage);
        fixture.empty();
        Self {
            object: Arc::from(bytes),
            header: Arc::from(sources.header),
            object_sha256: digest,
            tools,
        }
    }
    pub(super) fn bytes(&self) -> &[u8] {
        assert_eq!(sha(&self.object), self.object_sha256);
        self.revalidate();
        &self.object
    }
    pub(super) fn header(&self) -> &[u8] {
        &self.header
    }
    pub(super) fn compile(&self, stage: &NativeStage, args: &[OsString], phase: &str) {
        self.revalidate();
        compile(&self.tools, stage, args, phase);
    }
    fn revalidate(&self) {
        native::revalidate_tool(&self.tools.driver, &self.tools.driver_identity)
            .expect("retained compiler identity");
        native::revalidate_tool(&self.tools.linker, &self.tools.linker_identity)
            .expect("retained linker identity");
    }
}

fn compile(
    tools: &native::LinuxX8664LinkToolchain,
    stage: &NativeStage,
    args: &[OsString],
    phase: &str,
) {
    native::revalidate_tool(&tools.driver, &tools.driver_identity)
        .expect("actual selected compiler");
    native::revalidate_tool(&tools.linker, &tools.linker_identity).expect("actual selected linker");
    stage.revalidate().expect("owned stage before compiler");
    let limits = NativeProcessLimits::default();
    let directory = stage.capability_directory_path();
    let actual = native::run_bounded_process(
        &tools.driver,
        args,
        &directory,
        limits.link_timeout(),
        limits.tool_output_bytes(),
        limits.tool_output_bytes(),
        ProcessPhase::Link,
        Some(&directory),
    );
    stage.revalidate().expect("owned stage after compiler");
    native::revalidate_tool(&tools.driver, &tools.driver_identity)
        .expect("compiler identity after use");
    native::revalidate_tool(&tools.linker, &tools.linker_identity)
        .expect("linker identity after use");
    eprintln!(
        "417-sqlite-phase {}",
        serde_json::json!({"phase":phase,
        "argv_bytes":args.iter().map(|value|value.as_encoded_bytes()).collect::<Vec<_>>(),
        "raw_output_available":actual.is_ok()})
    );
    let output = actual.unwrap_or_else(|error| {
        panic!("{phase}: helper Diagnostic {error:?}; raw Output unavailable")
    });
    eprintln!(
        "417-sqlite-process {}",
        serde_json::json!({"phase":phase,
        "status":output.status.code(),"signal":output.status.signal(),
        "stdout":output.stdout,"stderr":output.stderr})
    );
    assert!(output.status.success(), "{phase}: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
}

fn check_provider(bytes: &[u8]) {
    let file = object::File::parse(bytes).expect("actual provider ELF");
    assert_eq!(file.format(), BinaryFormat::Elf);
    assert_eq!(file.architecture(), Architecture::X86_64);
    assert_eq!(file.endianness(), Endianness::Little);
    assert_eq!(file.kind(), object::ObjectKind::Relocatable);
    assert!(file.is_64());
    for name in ["sqlite3_malloc", "sqlite3_free"] {
        let definitions = file
            .symbols()
            .filter(|symbol| symbol.name() == Ok(name))
            .filter(|symbol| !symbol.is_undefined())
            .collect::<Vec<_>>();
        assert_eq!(definitions.len(), 1, "exact selected real SQLite definition");
        let symbol = &definitions[0];
        assert!(symbol.is_global() && !symbol.is_weak() && symbol.kind() == SymbolKind::Text);
        assert!(symbol.section_index().is_some() && symbol.size() != 0);
    }
}
