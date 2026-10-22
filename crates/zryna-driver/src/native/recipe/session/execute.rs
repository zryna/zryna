//! Only fixed trusted supervisor modes can enter the recipe namespace.
use super::client::{digest_id, string};
use super::config::ENV;
use super::resources::Resources;
use super::{Result, fail};
use crate::native::recipe::engine::{self, Body, Method};
use serde_json::json;

pub(super) fn execute(
    resources: &Resources,
    index: usize,
    mode: &str,
    expected: &[u8],
) -> Result<()> {
    if !matches!(mode, "inspect" | "check-wx" | "check-boundaries" | "compile") {
        return Err(fail("undeclared native invocation"));
    }
    resources.live(index)?;
    let cid = resources.id(index)?;
    let argv = ["/supervisor", mode];
    let create = json!({"AttachStdin":false,"AttachStdout":true,"AttachStderr":true,
        "Tty":false,"Privileged":false,"User":"65534:65534","WorkingDir":"/tmp",
        "Env":ENV,"Cmd":argv});
    let state =
        resources.client.json(Method::Post, &format!("/containers/{cid}/exec"), Some(&create))?;
    let eid = string(&state, "Id")?;
    if !digest_id(eid) {
        return Err(fail("full owned exec identity required"));
    }
    let initial = resources.client.json(Method::Get, &format!("/exec/{eid}/json"), None)?;
    inspect(&initial, eid, cid, mode, false)?;
    let payload = serde_json::to_vec(&json!({"Detach":false,"Tty":false}))
        .map_err(|_| fail("fixed exec encoding failed"))?;
    let frames = resources.client.bytes(
        Method::Post,
        &format!("/exec/{eid}/start"),
        Body::Json(&payload),
        132 * 1024,
        &[200],
    )?;
    let streams = engine::parse_frames(resources.client.deadline, &frames)
        .map_err(|_| fail("bounded native process streams invalid"))?;
    let finished = resources.client.json(Method::Get, &format!("/exec/{eid}/json"), None)?;
    inspect(&finished, eid, cid, mode, true)?;
    if streams.stdout != expected || !streams.stderr.is_empty() {
        return Err(fail("fixed native invocation output differs"));
    }
    resources.live(index)
}
fn inspect(
    state: &serde_json::Value,
    eid: &str,
    cid: &str,
    mode: &str,
    finished: bool,
) -> Result<()> {
    let process = &state["ProcessConfig"];
    if state["ID"] != eid
        || state["ContainerID"] != cid
        || state["Running"] != false
        || state["OpenStdin"] != false
        || state["OpenStdout"] != true
        || state["OpenStderr"] != true
        || process["entrypoint"] != "/supervisor"
        || process["arguments"] != json!([mode])
        || process["privileged"] != false
        || process["tty"] != false
        || process["user"] != "65534:65534"
    {
        return Err(fail("actual native exec identity or result differs"));
    }
    if finished && state["ExitCode"] != 0 {
        return Err(fail(if mode == "compile" {
            "actual native compiler exited unsuccessfully"
        } else {
            "actual native control exited unsuccessfully"
        }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transplanted_or_running_exec_cannot_satisfy_process_success() {
        let original = json!({"ID":"exec","ContainerID":"owner","Running":false,
            "OpenStdin":false,"OpenStdout":true,"OpenStderr":true,"ExitCode":0,
            "ProcessConfig":{"entrypoint":"/supervisor","arguments":["compile"],
                "privileged":false,"tty":false,"user":"65534:65534"}});
        assert!(inspect(&original, "exec", "owner", "compile", true).is_ok());
        for (key, value) in
            [("ContainerID", json!("other")), ("Running", json!(true)), ("ExitCode", json!(137))]
        {
            let mut changed = original.clone();
            changed[key] = value;
            assert!(inspect(&changed, "exec", "owner", "compile", true).is_err());
        }
    }
}
