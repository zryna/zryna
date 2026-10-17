use super::{Result, approval::hash, client::Client, config::ENV, reject};
use crate::native::recipe::engine::Method;
use serde_json::{Value, json};

pub(super) fn preflight(client: Client<'_>, image: &str) -> Result<()> {
    let state = client.json(Method::Get, &format!("/images/{image}/json"), None)?;
    validate(&state, image)
}

pub(super) fn validate(state: &Value, image: &str) -> Result<()> {
    if !image.strip_prefix("sha256:").is_some_and(hash)
        || state["Id"] != image
        || state["Os"] != "linux"
        || state["Architecture"] != "amd64"
        || state.get("Variant").is_some_and(|value| value != "")
        || state.get("OsVersion").is_some_and(|value| value != "")
    {
        return Err(reject(
            "PLAYGROUND-HOST-IMAGE",
            "immutable approved Linux x86-64 image differs",
        ));
    }
    let config = state["Config"]
        .as_object()
        .ok_or_else(|| reject("PLAYGROUND-HOST-IMAGE", "image configuration absent"))?;
    let fixed = json!({"User":"65534:65534","Entrypoint":["/supervisor"],
        "Cmd":["idle"],"WorkingDir":"/tmp","Env":ENV});
    for (key, expected) in fixed.as_object().expect("fixed image configuration") {
        if config.get(key) != Some(expected) {
            return Err(reject(
                "PLAYGROUND-HOST-IMAGE",
                "inherited fixed image configuration differs",
            ));
        }
    }
    for (key, value) in config {
        if fixed.get(key).is_some() {
            continue;
        }
        let permitted = match key.as_str() {
            "Hostname" | "Domainname" | "MacAddress" | "StopSignal" => value == "",
            "AttachStdin" | "AttachStdout" | "AttachStderr" | "Tty" | "OpenStdin" | "StdinOnce"
            | "ArgsEscaped" | "NetworkDisabled" => value == false,
            "Volumes" | "ExposedPorts" | "Healthcheck" | "Labels" => {
                value.is_null() || value.as_object().is_some_and(serde_json::Map::is_empty)
            }
            "OnBuild" | "Shell" => value.is_null() || value.as_array().is_some_and(Vec::is_empty),
            "StopTimeout" => value.is_null(),
            "Image" => value.as_str().is_some_and(|value| {
                value.is_empty() || value.strip_prefix("sha256:").is_some_and(hash)
            }),
            _ => false,
        };
        if !permitted {
            return Err(reject(
                "PLAYGROUND-HOST-IMAGE",
                "inherited or unknown image authority refused",
            ));
        }
    }
    Ok(())
}
