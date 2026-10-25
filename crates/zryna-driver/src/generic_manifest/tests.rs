//! Hostile metadata is rejected independently of source/runtime authority issuance.
use super::*;
use serde_json::{Value, json};

fn base() -> Value {
    let digest = "0".repeat(64);
    json!({
        "format":FORMAT,"profile":PROFILE,"syntaxProtocol":5,"command":"build",
        "artifactStem":"sample",
        "source":{
            "entrypoint":"main.zry","entryModule":1,
            "modules":[
                {"id":0,"path":"a.zry","bytes":1,"sha256":digest},
                {"id":1,"path":"main.zry","bytes":1,"sha256":digest}
            ],
            "edges":[{"importer":"main.zry","target":"a.zry","specifier":"./a.zry",
                "imported":"id","local":"alias"}],
            "graphSha256":digest,"executionSha256":digest,"typeUniverseSha256":digest,
            "linear32Sha256":digest,"linuxX8664Sha256":digest,"runtimeAbiSha256":digest
        },
        "exports":[{"name":"entry","parameters":[],"result":"i32"}],
        "artifacts":[{"target":"javascript","kind":"ecmascript-module",
            "path":"javascript/sample.mjs","bytes":1,"sha256":digest}],
        "native":null,"invocation":null
    })
}
fn read(value: &Value) -> Result<GenericManifestWire, Diagnostic> {
    decode(&serde_json::to_vec(value).expect("fixture JSON"))
}

#[test]
fn strict_generic_metadata_roundtrip_retains_nonzero_entry_and_named_alias() {
    let wire = read(&base()).expect("closed plain metadata");
    assert_eq!(wire.source.entry_module, 1);
    assert_eq!(wire.source.edges[0].local, "alias");
    let bytes = encode(&wire).expect("bounded validated serialization");
    assert_eq!(decode(&bytes).expect("strict roundtrip"), wire);
    assert_eq!(MANIFEST_NAME, "zryna-generic-manifest-v1.json");
}

#[test]
fn unknown_fields_and_old_profile_versions_do_not_enter_the_new_document() {
    for pointer in
        ["", "/source", "/source/modules/0", "/source/edges/0", "/exports/0", "/artifacts/0"]
    {
        let mut value = base();
        value
            .pointer_mut(pointer)
            .expect("known nested fixture")
            .as_object_mut()
            .expect("record")
            .insert("extra".into(), json!(true));
        assert!(read(&value).is_err(), "unknown field at {pointer}");
    }
    for (field, changed) in [
        ("profile", json!("data-ownership-v1")),
        ("format", json!("zryna.generic-bundle.v2")),
        ("syntaxProtocol", json!(4)),
    ] {
        let mut value = base();
        value[field] = changed;
        assert!(read(&value).is_err());
    }
}

#[test]
fn independent_source_export_and_target_metadata_drift_is_rejected() {
    let changes = [
        ("/source/entryModule", json!(0)),
        ("/source/modules/1/id", json!(2)),
        ("/source/modules/1/path", json!("../main.zry")),
        ("/source/edges/0/target", json!("absent.zry")),
        ("/source/edges/0/specifier", json!("./main.zry")),
        ("/source/edges/0/specifier", json!("/a.zry")),
        ("/source/executionSha256", json!("F".repeat(64))),
        ("/exports/0/result", json!("String")),
        ("/artifacts/0/path", json!("javascript/../sample.mjs")),
        ("/artifacts/0/kind", json!("webassembly-component")),
        ("/artifacts/0/bytes", json!(0)),
        ("/artifacts", json!([])),
    ];
    for (pointer, changed) in changes {
        let mut value = base();
        *value.pointer_mut(pointer).expect("known field") = changed;
        assert!(read(&value).is_err(), "drift at {pointer}");
    }
}

#[test]
fn completed_run_accepts_only_selected_typed_returns_and_five_source_traps() {
    let mut value = base();
    value["command"] = json!("run");
    value["invocation"] = json!({"export":"entry","arguments":[],"outcomes":[{
        "target":"javascript","outcome":{"kind":"returned","value":{"type":"i32","value":14}}
    }]});
    read(&value).expect("typed completed run");
    for code in [
        "zryna.trap.bounds-v1",
        "zryna.trap.allocation-v1",
        "zryna.trap.capacity-v1",
        "zryna.trap.refcount-v1",
        "zryna.trap.utf8-v1",
    ] {
        value["invocation"]["outcomes"][0]["outcome"] = json!({"kind":"trapped","code":code});
        read(&value).expect("one exact controlled source trap");
    }
    for outcome in [
        json!({"kind":"trapped","code":"target-trap"}),
        json!({"kind":"trapped","code":"unreachable"}),
        json!({"kind":"host-error","code":"target-unavailable"}),
        json!({"kind":"returned","value":{"type":"bool","value":true}}),
    ] {
        value["invocation"]["outcomes"][0]["outcome"] = outcome;
        assert!(read(&value).is_err(), "host/engine/type failure is not a source outcome");
    }
}

#[test]
fn empty_export_build_metadata_cannot_mint_a_run_selection() {
    let mut value = base();
    value["exports"] = json!([]);
    let build = read(&value).expect("compile-time-only library has no scalar exports");
    assert!(build.exports.is_empty());
    let bytes = encode(&build).expect("empty build ABI serializes");
    assert_eq!(decode(&bytes).expect("empty build ABI roundtrip"), build);
    value["command"] = json!("run");
    assert!(read(&value).is_err(), "a Run requires an actual selected scalar export");
    value["invocation"] = json!({"export":"identity","arguments":[],"outcomes":[{
        "target":"javascript","outcome":{"kind":"returned","value":{"type":"i32","value":7}}
    }]});
    assert!(read(&value).is_err(), "plain metadata cannot invent a template entrypoint");
}

#[test]
fn lowercase_suffix_metadata_preserves_shared_dotfile_dependency_grammar() {
    let importer = zryna_source::NormalizedSourcePath::new("main.zry").expect("portable entry");
    assert_eq!(
        zryna_source::resolve_explicit_zry_import(&importer, "./.zry")
            .expect("existing shared grammar")
            .as_str(),
        ".zry"
    );
    let mut value = base();
    value["source"]["modules"][0]["path"] = json!(".zry");
    value["source"]["edges"][0]["target"] = json!(".zry");
    value["source"]["edges"][0]["specifier"] = json!("./.zry");
    let wire = read(&value).expect("shared exact lowercase suffix permits .zry dependency");
    let bytes = encode(&wire).expect("bounded metadata output");
    assert_eq!(decode(&bytes).expect("same strict metadata roundtrip"), wire);
    // Isolate module-suffix rejection from import resolution by removing optional edges.
    value["source"]["edges"] = json!([]);
    for wrong in [".ZRY", "dep.zry.bak", "dir/leaf.zry.bak"] {
        value["source"]["modules"][0]["path"] = json!(wrong);
        assert!(read(&value).is_err(), "wrong module suffix {wrong}");
    }
}
