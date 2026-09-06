use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use zryna_abi::{Invocation, ScalarOutcome, ScalarValue};
use zryna_semantics::data_ownership_v1::{SemanticInput, lower};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v4::{decode_snapshot, verify_snapshot};

use super::*;

const SOURCE: &str = include_str!("../../../../../tests/m3-fixtures/pair-score-v4.zry");
const SNAPSHOT: &str = include_str!("../../../../../tests/m3-fixtures/pair-score-v4.json");
static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot {
    workspace: PathBuf,
    output: ArtifactOutputRoot,
}

impl TestRoot {
    fn new() -> Self {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let workspace = std::env::temp_dir()
            .join(format!("zryna-ownership-native-{}-{sequence}", std::process::id()));
        fs::create_dir(&workspace).expect("test workspace");
        let output = ArtifactOutputRoot::prepare_for_workspace(&workspace).expect("output root");
        Self { workspace, output }
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.workspace).expect("test workspace cleanup");
    }
}

fn verified() -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    const FUNCTION_START: u64 = 62;
    const EXPORT_BYTES: u64 = 7;
    let source = format!(
        "{}export {}",
        &SOURCE[..FUNCTION_START as usize],
        &SOURCE[FUNCTION_START as usize..]
    );
    let mut snapshot: serde_json::Value = serde_json::from_str(SNAPSHOT).expect("JSON snapshot");
    shift_spans(&mut snapshot, FUNCTION_START, EXPORT_BYTES);
    snapshot["files"][0]["functions"][0]["export_span"] = serde_json::json!({
        "file": 0,
        "start": FUNCTION_START,
        "end": FUNCTION_START + 6,
    });
    snapshot["files"][0]["functions"][0]["span"]["start"] = serde_json::Value::from(FUNCTION_START);
    let snapshot = serde_json::to_vec(&snapshot).expect("shifted snapshot");
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "src/main.zry".to_owned(), text: source }])
            .expect("source map");
    let syntax = verify_snapshot(decode_snapshot(&snapshot).expect("snapshot"), &sources)
        .expect("verified syntax");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    lower(SemanticInput::try_new(&syntax, &sources, entry).expect("semantic input"))
        .expect("verified program")
}

fn shift_spans(value: &mut serde_json::Value, threshold: u64, shift: u64) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                shift_spans(value, threshold, shift);
            }
        }
        serde_json::Value::Object(fields) => {
            if fields.contains_key("file")
                && fields.contains_key("start")
                && fields.contains_key("end")
            {
                for key in ["start", "end"] {
                    if let Some(offset) = fields.get_mut(key)
                        && offset.as_u64().is_some_and(|offset| offset >= threshold)
                    {
                        *offset = serde_json::Value::from(offset.as_u64().expect("offset") + shift);
                    }
                }
            } else {
                for value in fields.values_mut() {
                    shift_spans(value, threshold, shift);
                }
            }
        }
        _ => {}
    }
}

fn invocation() -> Invocation {
    Invocation::new("pairScore".to_owned(), vec![ScalarValue::I32(2), ScalarValue::I32(3)])
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn ownership_objects_link_publish_create_only_and_execute() {
    use std::os::unix::fs::PermissionsExt as _;

    let root = TestRoot::new();
    let program = verified();
    let limits = NativeProcessLimits::default();
    let toolchain = discover_linux_native_toolchain(limits).expect("toolchain");
    let first = prepare_data_ownership_executable(
        &program,
        invocation(),
        &root.output,
        zryna_backend_native::NATIVE_OBJECT_TARGET,
        &toolchain,
        limits,
    )
    .expect("first executable");
    let second = prepare_data_ownership_executable(
        &program,
        invocation(),
        &root.output,
        zryna_backend_native::NATIVE_OBJECT_TARGET,
        &toolchain,
        limits,
    )
    .expect("deterministic executable");
    assert_eq!(first.object_bytes(), second.object_bytes());
    assert_eq!(first.executable_bytes(), second.executable_bytes());
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.identity().target(), zryna_backend_native::NATIVE_OBJECT_TARGET);
    assert_eq!(first.identity().logical_export(), "pairScore");

    let object = publish_data_ownership_object(&first, &root.output, "pair-score")
        .expect("published object");
    let executable = publish_data_ownership_executable(&first, &root.output, "pair-score")
        .expect("published executable");
    assert_eq!(fs::read(object.path()).expect("object bytes"), first.object_bytes());
    assert_eq!(fs::read(executable.path()).expect("executable bytes"), first.executable_bytes());
    assert_eq!(fs::metadata(executable.path()).expect("mode").permissions().mode() & 0o777, 0o755);
    assert_eq!(executable.data_ownership_identity(), Some(first.identity()));
    assert_eq!(
        run_native_invocation(&executable, limits).expect("native outcome"),
        ScalarOutcome::Returned { value: ScalarValue::I32(65) }
    );

    let object_before = fs::read(object.path()).expect("original object");
    let executable_before = fs::read(executable.path()).expect("original executable");
    assert!(publish_data_ownership_object(&first, &root.output, "pair-score").is_err());
    assert!(publish_data_ownership_executable(&first, &root.output, "pair-score").is_err());
    assert_eq!(fs::read(object.path()).expect("retained object"), object_before);
    assert_eq!(fs::read(executable.path()).expect("retained executable"), executable_before);
    assert!(fs::read_dir(root.output.path()).expect("output entries").all(|entry| {
        !entry.expect("entry").file_name().to_string_lossy().starts_with(".zryna-link")
    }));
}

#[test]
fn unsupported_target_fails_before_staging_or_publication() {
    let root = TestRoot::new();
    let program = verified();
    let toolchain = discover_linux_native_toolchain(NativeProcessLimits::default());
    if let Ok(toolchain) = toolchain {
        let failure = prepare_data_ownership_executable(
            &program,
            invocation(),
            &root.output,
            "x86_64-linux-gnu",
            &toolchain,
            NativeProcessLimits::default(),
        )
        .expect_err("target alias must fail");
        assert_eq!(failure[0].code(), "ZRYNA-N3001");
    }
    assert_eq!(fs::read_dir(root.output.path()).expect("output entries").count(), 0);
}
