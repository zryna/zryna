use super::materials;
use super::{
    Operation, Result, approval::hash, client::string, config::ENV, reject, resources::Resources,
};
use crate::native::recipe::engine::Method;
use serde_json::{Value, json};
use std::{thread, time::Duration};

impl Resources<'_> {
    /// Audits every protected material byte before the first guest carrier is uploaded.
    pub fn audit(&self, index: usize) -> Result<()> {
        self.live(index)?;
        let cid = self.id(index)?;
        let request = json!({"AttachStdin":false,"AttachStdout":true,"AttachStderr":true,
            "Tty":false,"Privileged":false,"User":"65534:65534","WorkingDir":"/tmp",
            "Env":ENV,"Cmd":["/supervisor","audit"]});
        let created =
            self.client.json(Method::Post, &format!("/containers/{cid}/exec"), Some(&request))?;
        let eid = string(&created, "Id")?;
        if !hash(eid) {
            return Err(reject(
                "PLAYGROUND-HOST-MATERIALS",
                "full owned audit exec identity required",
            ));
        }
        let initial = self.client.json(Method::Get, &format!("/exec/{eid}/json"), None)?;
        inspect_fixed(&initial, eid, cid, "audit", true, false)?;
        let body = br#"{"Detach":false,"Tty":false}"#;
        let bytes = self.client.bytes(
            Method::Post,
            &format!("/exec/{eid}/start"),
            crate::native::recipe::engine::Body::Json(body),
            4096,
            &[200],
        )?;
        let streams = crate::native::recipe::engine::parse_frames(self.client.deadline, &bytes)
            .map_err(|_| {
                reject("PLAYGROUND-HOST-MATERIALS", "bounded audit response framing invalid")
            })?;
        let final_state = self.client.json(Method::Get, &format!("/exec/{eid}/json"), None)?;
        inspect_fixed(&final_state, eid, cid, "audit", true, true)?;
        if final_state["Running"] != false
            || final_state["ExitCode"] != 0
            || !streams.stderr.is_empty()
        {
            return Err(reject("PLAYGROUND-HOST-MATERIALS", "actual material audit refused"));
        }
        let pin = if self.operation == Operation::Compile {
            &self.approval.record.compiler_materials
        } else {
            &self.approval.record.evaluator_materials
        };
        materials::audit_receipt(&streams.stdout, pin)?;
        materials::inspect(
            self.client,
            &self.approval.record,
            self.operation == Operation::Evaluate,
        )?;
        self.live(index)
    }

    pub fn execute(&self, index: usize) -> Result<i64> {
        self.live(index)?;
        let cid = self.id(index)?;
        let mode = self.operation.name();
        let request = json!({"AttachStdin":false,"AttachStdout":false,"AttachStderr":false,
            "Tty":false,"Privileged":false,"User":"65534:65534","WorkingDir":"/tmp",
            "Env":ENV,"Cmd":["/supervisor",mode]});
        let created =
            self.client.json(Method::Post, &format!("/containers/{cid}/exec"), Some(&request))?;
        let eid = string(&created, "Id")?;
        if !hash(eid) {
            return Err(reject("PLAYGROUND-HOST-ENFORCEMENT", "full owned exec identity required"));
        }
        let initial = self.client.json(Method::Get, &format!("/exec/{eid}/json"), None)?;
        inspect(&initial, eid, cid, self.operation, false)?;
        let start = json!({"Detach":true,"Tty":false});
        let body = serde_json::to_vec(&start)
            .map_err(|_| reject("PLAYGROUND-HOST-TRANSPORT", "fixed exec encoding failed"))?;
        let response = self.client.bytes(
            Method::Post,
            &format!("/exec/{eid}/start"),
            crate::native::recipe::engine::Body::Json(&body),
            262_144,
            &[200],
        )?;
        if !response.is_empty() {
            return Err(reject("PLAYGROUND-HOST-TRANSPORT", "unexpected detached exec response"));
        }
        loop {
            self.client.check()?;
            let value = self.client.json(Method::Get, &format!("/exec/{eid}/json"), None)?;
            inspect(&value, eid, cid, self.operation, true)?;
            if value["Running"] == false {
                self.live(index)?;
                return value["ExitCode"].as_i64().ok_or_else(|| {
                    reject("PLAYGROUND-HOST-ENFORCEMENT", "actual exit status absent")
                });
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

pub(super) fn inspect(
    value: &Value,
    eid: &str,
    cid: &str,
    operation: Operation,
    started: bool,
) -> Result<()> {
    inspect_fixed(value, eid, cid, operation.name(), false, started)
}

fn inspect_fixed(
    value: &Value,
    eid: &str,
    cid: &str,
    mode: &str,
    attached: bool,
    started: bool,
) -> Result<()> {
    let process = &value["ProcessConfig"];
    if value["ID"] != eid
        || value["ContainerID"] != cid
        || value["OpenStdin"] != false
        || value["OpenStdout"] != attached
        || value["OpenStderr"] != attached
        || process["entrypoint"] != "/supervisor"
        || process["arguments"] != json!([mode])
        || process["privileged"] != false
        || process["tty"] != false
        || process["user"] != "65534:65534"
        || !value["Running"].is_boolean()
        || !started && value["Running"] != false
    {
        return Err(reject(
            "PLAYGROUND-HOST-ENFORCEMENT",
            "actual fixed exec identity substituted",
        ));
    }
    Ok(())
}
