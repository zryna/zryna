use super::{digest, validate};
use serde_json::{Value, json};

fn encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for group in bytes.chunks(3) {
        let a = group[0];
        let b = group.get(1).copied().unwrap_or(0);
        let c = group.get(2).copied().unwrap_or(0);
        result.push(char::from(ALPHABET[usize::from(a >> 2)]));
        result.push(char::from(ALPHABET[usize::from((a & 3) << 4 | b >> 4)]));
        result.push(if group.len() > 1 {
            char::from(ALPHABET[usize::from((b & 15) << 2 | c >> 6)])
        } else {
            '='
        });
        result.push(if group.len() > 2 { char::from(ALPHABET[usize::from(c & 63)]) } else { '=' });
    }
    result
}

fn fixture() -> Value {
    // Inert correlation fixture, not a guest execution or valid compiler report claim.
    let source = "fn add(a: i32) -> i32 { a }";
    let component = b"\0asm\x01\0\0\0";
    let loader = b"inert loader fixture";
    let declarations = b"inert declarations fixture";
    let source_hash = digest(source.as_bytes());
    let component_hash = digest(component);
    let pin = "a".repeat(64);
    let header = json!({
        "format":"zryna.playground-compilation.v1", "version":1, "revision":7,
        "sourcePath":"src/main.zry", "sourceSha256":source_hash, "sourceBytes":source.len(),
        "profile":"browser-component-v1", "status":"compiled", "report":{}, "failure":null,
        "identity":{"revision":"zryna.browser-component-bindings.v1",
            "world":"zryna:capability-profiles/browser@0.1.0", "witSha256":pin,
            "componentSha256":component_hash, "coreSha256":component_hash,
            "interfaceSha256":pin, "coreOffset":0, "coreBytes":8},
        "exports":[{"logical":"add", "component":"zryna-export-616464", "core":"add", "arity":1}],
        "artifacts":[{"role":"component", "bytes":component.len(), "sha256":component_hash},
            {"role":"loader", "bytes":loader.len(), "sha256":digest(loader)},
            {"role":"declarations", "bytes":declarations.len(), "sha256":digest(declarations)}]
    });
    let metadata = serde_json::to_vec(&header).expect("fixture header");
    let mut frame = u32::try_from(metadata.len()).expect("bounded fixture").to_le_bytes().to_vec();
    frame.extend_from_slice(&metadata);
    frame.extend_from_slice(component);
    frame.extend_from_slice(loader);
    frame.extend_from_slice(declarations);
    json!({"version":1, "sourceRequest":{"version":1,"revision":7,"source":source},
        "frameBase64":encode(&frame),
        "selection":{"revision":7,"sourceSha256":source_hash,"componentSha256":component_hash,
            "logical":"add","args":[1]},
        "receipt":{"revision":7,"sourceSha256":source_hash,"frameSha256":digest(&frame),
            "compilerSha256":pin,"toolkitSha256":pin,"policySha256":pin},
        "authority":{"compilerSha256":pin,"toolkitSha256":pin,"policySha256":pin,
            "bindingTemplateSha256":pin}})
}

#[test]
fn correlated_frame_is_inert_and_all_provenance_axes_reject_drift() {
    let original = fixture();
    validate(&serde_json::to_vec(&original).expect("fixture")).expect("correlated inert shape");
    for (parent, field, replacement) in [
        ("sourceRequest", "version", json!(0)),
        ("sourceRequest", "revision", json!(8)),
        ("sourceRequest", "source", json!("changed source")),
        ("selection", "revision", json!(8)),
        ("selection", "sourceSha256", json!("b".repeat(64))),
        ("selection", "componentSha256", json!("b".repeat(64))),
        ("selection", "logical", json!("unknown")),
        ("selection", "args", json!([])),
        ("receipt", "frameSha256", json!("b".repeat(64))),
        ("receipt", "compilerSha256", json!("b".repeat(64))),
        ("authority", "policySha256", json!("b".repeat(64))),
    ] {
        let mut changed = original.clone();
        changed[parent][field] = replacement;
        assert!(
            validate(&serde_json::to_vec(&changed).expect("mutant")).is_err(),
            "{parent}.{field}"
        );
    }
}

#[test]
fn closed_transport_rejects_unknowns_duplicates_limits_and_bad_i32() {
    for parent in ["sourceRequest", "selection", "receipt", "authority"] {
        let mut changed = fixture();
        changed[parent]["extra"] = json!(true);
        assert!(validate(&serde_json::to_vec(&changed).expect("mutant")).is_err(), "{parent}");
    }
    let mut changed = fixture();
    changed["selection"]["args"] = json!([2_147_483_648_i64]);
    assert!(validate(&serde_json::to_vec(&changed).expect("large integer")).is_err());
    let encoded = serde_json::to_string(&fixture()).expect("fixture");
    let duplicate = encoded.replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
    assert!(validate(duplicate.as_bytes()).is_err());
    assert!(validate(&vec![b' '; super::MAX_EVALUATION_BYTES + 1]).is_err());
    let mut changed = fixture();
    changed["frameBase64"] =
        json!(format!("{}AAAA", changed["frameBase64"].as_str().expect("base64")));
    assert!(validate(&serde_json::to_vec(&changed).expect("extra frame bytes")).is_err());
}

#[test]
fn independently_approved_v2_composition_maps_legacy_wire_without_inventory_aliases() {
    let claims = validate(&serde_json::to_vec(&fixture()).expect("correlation fixture"))
        .expect("inert validated claims");
    let pin = "a".repeat(64);
    let material = json!({"name":"zryna410-materials-fixture","created_at":"2026-10-15T00:00:00Z",
        "nonce":"b".repeat(32),"root_sha256":pin,"files":1,"bytes":1});
    let record = json!({"version":2,"engine_uid":65534,"compiler_image_sha256":pin,
        "evaluator_image_sha256":pin,"compiler_root_sha256":pin,"evaluator_root_sha256":pin,
        "compiler_executable_sha256":pin,"binding_template_sha256":pin,"wit_closure_sha256":pin,
        "composition_kind":"reviewed-source-v1","composition_sha256":pin,
        "compiler_materials":material,"evaluator_materials":material,"cpus":[0,1],
        "compiler_seccomp_json":"{}","evaluator_seccomp_json":"{}","policy_sha256":pin});
    let approved =
        serde_json::from_value(record.clone()).expect("closed independent operator carrier");
    claims.authenticate(&approved).expect("explicit v2 compatibility mapping");
    for (field, value) in [
        ("version", json!(1)),
        ("composition_kind", json!("published-toolkit")),
        ("composition_sha256", json!("b".repeat(64))),
        ("compiler_executable_sha256", json!("b".repeat(64))),
        ("binding_template_sha256", json!("b".repeat(64))),
        ("wit_closure_sha256", json!("b".repeat(64))),
        ("policy_sha256", json!("b".repeat(64))),
    ] {
        let mut changed = record.clone();
        changed[field] = value;
        let changed = serde_json::from_value(changed).expect("inert unapproved mutant");
        assert!(claims.authenticate(&changed).is_err(), "{field}");
    }
    let mut distinct = record;
    distinct["compiler_root_sha256"] = json!("c".repeat(64));
    let distinct = serde_json::from_value(distinct).expect("distinct complete root identity");
    claims.authenticate(&distinct).expect("binary identity is not complete inventory identity");
}
