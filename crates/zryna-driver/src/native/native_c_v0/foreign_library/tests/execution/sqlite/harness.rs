//! Real generated copied entry plus separate adapter, `SQLite` provider and private runtime.
use super::*;
use crate::native::{self, NativeStage, ProcessPhase};
use std::{ffi::OsString, path::PathBuf};
use zryna_semantics::native_c_v0::LibraryMaterial;
mod adapter;
mod link;
use adapter::{adapter, capture, library_source};
use link::link_arguments;

#[derive(Clone, Copy, Debug)]
pub(super) enum Variant {
    Pristine,
    WrongCount(usize),
    MissingRelease,
}
const HEADER: &[u8] =
    include_bytes!("../../../../../../../../../tests/native-c-sqlite-v0/library.h");
const LIBRARY: &str = include_str!("../../../../../../../../../tests/native-c-sqlite-v0/library.c");
const ORACLE: &str =
    include_str!("../../../../../../../../../tests/native-c-sqlite-v0/generated-oracle.c");
const CLIENT: &str =
    include_str!("../../../../../../../../../tests/native-c-sqlite-v0/generated-client.c");

pub(super) fn observe(
    variant: Variant,
    scenario: u8,
    sanitized: bool,
) -> native::BoundedProcessOutput {
    let fixture = Fixture::new();
    let (requirements, material) = super::super::super::sqlite::execution_fixture();
    let provider = provider::Provider::get(sanitized);
    let library = library_source(variant);
    let ordinary = adapter(&fixture, provider, &library, false);
    let captured =
        capture(&requirements, &material, &ordinary).expect("exact two-import adapter audit");
    assert_eq!(
        captured.definitions().collect::<Vec<_>>(),
        ["sqlite_fixture_copy_bytes", "sqlite_fixture_release_bytes"]
    );
    assert_eq!(captured.dependencies().collect::<Vec<_>>(), ["sqlite3_free", "sqlite3_malloc"]);
    captured.check_binding(&requirements).expect("same original SQLite source issuer");
    let foreign = if sanitized {
        let bytes = adapter(&fixture, provider, &library, true);
        assert!(
            capture(&requirements, &material, &bytes).is_err(),
            "no sanitizer production adapter seal"
        );
        bytes
    } else {
        captured.bytes().to_vec()
    };
    let (runtime_stage, runtime) =
        private_byte_runtime::compile(&fixture, &requirements, sanitized);
    let runtime_stage = stage::Stage::retain(runtime_stage);
    let runtime_object =
        runtime_stage.capability_file_path("runtime.o").expect("private object capability");
    let stage = stage::Stage::new(&fixture);
    let provider_stage = stage::Stage::new(&fixture);
    provider_stage
        .write_input(&provider_stage.object, provider.bytes())
        .expect("private provider snapshot");
    let provider_object =
        provider_stage.capability_file_path("program.o").expect("provider object capability");
    let client = stage_inputs(&stage, &requirements, &foreign, scenario);
    let args = link_arguments(&stage, runtime_object, provider_object, sanitized);
    runtime_stage.revalidate().expect("private stage before link");
    provider_stage.revalidate().expect("provider stage before link");
    provider.compile(&stage, &args, "link-generated-sqlite");
    let executable = stage.capability_file_path("invocation.elf").expect("executable capability");
    let executable_bytes =
        source_input::bounded_read(&executable, native::MAX_NATIVE_EXECUTABLE_BYTES);
    if !sanitized {
        native::audit_staged_executable(&executable, "zryna_c_v0_i_dispatch")
            .expect("ordinary bounded ELF audit");
    }
    eprintln!(
        "417-sqlite-generated {}",
        serde_json::json!({
        "variant":format!("{variant:?}"),"scenario":scenario,"sanitized_c_only":sanitized,
        "original_generated_sha256":hex(requirements.object_sha256()),
        "adapter_source_sha256":hex(&sha(library.as_bytes())),
        "ordinary_adapter_sha256":hex(captured.object_sha256()),
        "observed_adapter_sha256":hex(&sha(&foreign)),"provider_sha256":hex(&sha(provider.bytes())),
        "private_source_sha256":hex(requirements.private_runtime_source_sha256().expect("private source")),
        "private_object_sha256":hex(&sha(&runtime)),"client_sha256":hex(&sha(client.as_bytes())),
        "executable_sha256":hex(&sha(&executable_bytes)),"executable_bytes":executable_bytes.len(),
        "production_sanitizer_seal":false,"recipe_admission":false})
    );
    run(&fixture, stage, runtime_stage, provider_stage, &executable, scenario, sanitized)
}

fn client_source(requirements: &HandleLinkRequirements, scenario: u8) -> String {
    let private_header =
        requirements.object().program().source().runtime_abi().native_linux_x86_64_header();
    assert_eq!(Some(&sha(private_header)), requirements.private_runtime_header_sha256());
    format!(
        "{}\n{}\n#include <assert.h>\n#include <stdlib.h>\n#include <string.h>\n#define SCENARIO {scenario}\n{ORACLE}\n{}",
        std::str::from_utf8(private_header).expect("original private ABI"),
        requirements.object().header(),
        CLIENT.replace("ENTRY", &entry(requirements, "copied"))
    )
}

fn run(
    fixture: &Fixture,
    stage: stage::Stage,
    runtime_stage: stage::Stage,
    provider_stage: stage::Stage,
    executable: &std::path::Path,
    scenario: u8,
    sanitized: bool,
) -> native::BoundedProcessOutput {
    native::prepare_executable_mode(executable).expect("owned bounded executable mode");
    stage.revalidate().expect("owned job stage before run");
    runtime_stage.revalidate().expect("private source stage before run");
    provider_stage.revalidate().expect("provider snapshot stage before run");
    let directory = stage.capability_directory_path();
    let result = native::run_bounded_process(
        executable,
        &[],
        &directory,
        native::MAX_NATIVE_RUN_TIMEOUT,
        4096,
        native::MAX_NATIVE_RUN_STDERR_BYTES,
        ProcessPhase::Run,
        Some(&directory),
    );
    drop(stage);
    drop(runtime_stage);
    drop(provider_stage);
    fixture.empty();
    let output = result.unwrap_or_else(|error| {
        panic!("run helper Diagnostic {error:?}; raw Output unavailable; no control credit")
    });
    eprintln!(
        "417-sqlite-run {}",
        serde_json::json!({"scenario":scenario,"sanitized":sanitized,
        "status":output.status.code(),"signal":output.status.signal(),"stdout":output.stdout,"stderr":output.stderr})
    );
    assert!(output.stdout.is_empty());
    if output.status.success() {
        assert!(output.stderr.is_empty());
    }
    output
}

fn stage_inputs(
    stage: &NativeStage,
    requirements: &HandleLinkRequirements,
    foreign: &[u8],
    scenario: u8,
) -> String {
    stage
        .write_input(&stage.object, requirements.object().bytes())
        .expect("original generated object");
    stage
        .write_input(&stage.directory.join("runtime.o"), foreign)
        .expect("captured separate adapter");
    let client = client_source(requirements, scenario);
    stage.write_input(&stage.harness, client.as_bytes()).expect("fixed independent oracle client");
    client
}
