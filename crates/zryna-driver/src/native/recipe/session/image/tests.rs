use super::*;

fn sample() -> (Value, String) {
    let image = format!("sha256:{}", "a".repeat(64));
    let state = json!({"Id":image,"Os":"linux","Architecture":"amd64",
        "Config":{"Hostname":"","Domainname":"","User":"65534:65534",
        "AttachStdin":false,"AttachStdout":false,"AttachStderr":false,
        "Tty":false,"OpenStdin":false,"StdinOnce":false,
        "Env":[BUILDER_PATH],"Cmd":["idle"],
        "Image":format!("sha256:{}", "b".repeat(64)),"Volumes":null,
        "WorkingDir":"/tmp","Entrypoint":["/supervisor"],"OnBuild":null,"Labels":null}});
    (state, image)
}

#[test]
fn immutable_image_defaults_are_closed_before_resource_creation() {
    let (original, image) = sample();
    assert!(validate(&original, &image).is_ok());
    let mut api_149 = original.clone();
    api_149["Config"]["Image"] = json!("");
    assert!(validate(&api_149, &image).is_ok());
    for (field, value) in [
        ("Volumes", json!({"/surprise":{}})),
        ("Healthcheck", json!({"Test":["CMD","/supervisor","compile"]})),
        ("OnBuild", json!(["RUN /supervisor compile"])),
        ("Shell", json!(["/supervisor"])),
        ("Labels", json!({"inherited.owner":"other"})),
        ("ExposedPorts", json!({"1234/tcp":{}})),
        ("StopSignal", json!("SIGKILL")),
        ("StopTimeout", json!(1)),
        ("UnknownAuthority", json!(false)),
        ("OpenStdin", json!(true)),
        ("Env", json!(["TOKEN=secret-canary"])),
        ("User", json!("0:0")),
        ("Entrypoint", json!(["/tool/bin/gcc"])),
        ("Cmd", json!(["compile"])),
        ("WorkingDir", json!("/input")),
        ("Image", json!("mutable-tag")),
    ] {
        let mut changed = original.clone();
        changed["Config"][field] = value;
        assert!(validate(&changed, &image).is_err(), "field {field}");
    }
    assert!(validate(&original, &image).is_ok());
}

#[test]
fn image_identity_platform_and_typed_inactive_defaults_cannot_be_substituted() {
    let (original, image) = sample();
    for (field, value) in [
        ("Id", json!(format!("sha256:{}", "c".repeat(64)))),
        ("Architecture", json!("arm64")),
        ("Os", json!("windows")),
        ("Variant", json!("v2")),
        ("OsVersion", json!("other")),
        ("Config", Value::Null),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        assert!(validate(&changed, &image).is_err(), "field {field}");
    }
    for field in ["Volumes", "Healthcheck", "OnBuild", "Shell", "Labels", "ExposedPorts"] {
        for value in [json!(false), json!(0), json!("")] {
            let mut changed = original.clone();
            changed["Config"][field] = value;
            assert!(validate(&changed, &image).is_err(), "field {field}");
        }
    }
    for field in ["User", "Entrypoint", "Cmd", "WorkingDir", "Env"] {
        let mut changed = original.clone();
        changed["Config"].as_object_mut().expect("sample config").remove(field);
        assert!(validate(&changed, &image).is_err(), "field {field}");
    }
    assert!(validate(&original, "sha256:tag").is_err());
    assert!(validate(&original, &image).is_ok());
}
