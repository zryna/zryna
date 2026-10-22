//! Exact tiny adapter capture; provider bytes never enter this audit.
use super::*;

pub(super) fn library_source(variant: Variant) -> String {
    match variant {
        Variant::Pristine => LIBRARY.to_owned(),
        Variant::WrongCount(count) => {
            assert!([0, 2, 4097].contains(&count));
            let marker = "  *out_length = length;";
            assert_eq!(LIBRARY.matches(marker).count(), 1);
            LIBRARY.replace(marker, &format!("  *out_length = {count};"))
        }
        Variant::MissingRelease => {
            let marker = "  sqlite3_free(bytes);";
            assert_eq!(LIBRARY.matches(marker).count(), 1);
            LIBRARY.replace(marker, "  if (bytes == NULL) sqlite3_free(bytes);")
        }
    }
}

pub(super) fn capture(
    requirements: &HandleLinkRequirements,
    material: &LibraryMaterial<'_>,
    bytes: &[u8],
) -> Result<CapturedForeignLibrary, Diagnostic> {
    capture_foreign_library(
        requirements,
        &ForeignLibraryInput {
            library_id: material.library_id,
            header_bytes: material.header_bytes,
            policy_bytes: material.policy_bytes,
            object_bytes: bytes,
            object_size: bytes.len(),
            object_sha256: &sha(bytes),
            dependency_symbols: &["sqlite3_free", "sqlite3_malloc"],
        },
    )
}

pub(super) fn adapter(
    fixture: &Fixture,
    provider: &provider::Provider,
    source: &str,
    sanitized: bool,
) -> Vec<u8> {
    const INCLUDES: &str = "#include \"library.h\"\n#include \"sqlite3.h\"\n";
    let stage = stage::Stage::new(fixture);
    assert_eq!(source.matches("#include \"library.h\"").count(), 1);
    assert_eq!(source.matches("#include \"sqlite3.h\"").count(), 1);
    let body = source.strip_prefix(INCLUDES).expect("only two exact leading adapter includes");
    let source = [provider.header(), b"\n", HEADER, b"\n", body.as_bytes()].concat();
    stage.write_input(&stage.harness, &source).expect("authenticated headers plus fixed adapter");
    let object = stage.capability_file_path("runtime.o").expect("separate adapter object");
    let input = stage.capability_file_path("invocation.c").expect("retained adapter source");
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
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-c",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    if sanitized {
        args.extend(SANITIZERS.map(Into::into));
    }
    args.extend([input.into_os_string(), "-o".into(), object.clone().into_os_string()]);
    provider.compile(&stage, &args, "compile-adapter");
    let bytes = source_input::bounded_read(&object, zryna_backend_native::MAX_NATIVE_OBJECT_BYTES);
    eprintln!(
        "417-sqlite-adapter-source {}",
        serde_json::json!({
        "sanitized":sanitized,"translation_unit_sha256":hex(&sha(&source)),
        "adapter_body_sha256":hex(&sha(body.as_bytes())),
        "local_header_sha256":hex(&sha(HEADER)),"upstream_header_sha256":hex(&sha(provider.header()))})
    );
    drop(stage);
    fixture.empty();
    bytes
}
