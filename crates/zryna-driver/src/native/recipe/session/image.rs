//! Reject inherited image effects before creating any owned volumes or containers.
use super::client::{Client, digest_id};
use super::{Result, fail};
use crate::native::recipe::engine::Method;
use serde_json::{Value, json};

const BUILDER_PATH: &str = "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

pub(super) fn preflight(client: Client, image: &str) -> Result<()> {
    if !image.strip_prefix("sha256:").is_some_and(digest_id) {
        return Err(fail("immutable tool image identity required"));
    }
    let state = client.json(Method::Get, &format!("/images/{image}/json"), None)?;
    validate(&state, image)
}

fn validate(state: &Value, image: &str) -> Result<()> {
    if !image.strip_prefix("sha256:").is_some_and(digest_id)
        || state["Id"] != image
        || state["Os"] != "linux"
        || state["Architecture"] != "amd64"
        || state.get("Variant").is_some_and(|value| value != "")
        || state.get("OsVersion").is_some_and(|value| value != "")
    {
        return Err(fail("actual immutable Linux x86-64 image differs"));
    }
    let config =
        state["Config"].as_object().ok_or_else(|| fail("actual image configuration absent"))?;
    let fixed = json!({"User":"65534:65534","Entrypoint":["/supervisor"],
        "Cmd":["idle"],"WorkingDir":"/tmp","Env":[BUILDER_PATH]});
    for (key, expected) in fixed.as_object().expect("fixed image config") {
        if config.get(key) != Some(expected) {
            return Err(fail("fixed inherited image configuration differs"));
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
            // Legacy builder provenance is metadata; it is never used to select an image.
            "Image" => value.as_str().is_some_and(|value| {
                value.is_empty() || value.strip_prefix("sha256:").is_some_and(digest_id)
            }),
            _ => false,
        };
        if !permitted {
            return Err(fail("unsupported inherited image authority"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
