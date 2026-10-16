use super::{
    Operation, Result,
    approval::{Approved, hash},
    archive,
    client::{Client, string},
    config, identity, kernel, reject,
};
use crate::native::recipe::engine::{Body, Method};
use serde_json::{Value, json};
use std::{thread, time::Duration};

pub(super) struct Container {
    pub name: String,
    pub cid: Option<String>,
    pub requested: Value,
    pub init: Option<kernel::Init>,
    pub removed: bool,
}

pub(super) struct Resources<'a> {
    pub owner: String,
    pub approval: &'a Approved,
    pub operation: Operation,
    pub volumes: Vec<(String, Value)>,
    pub containers: Vec<Container>,
    pub client: Client<'a>,
}

impl Resources<'_> {
    pub fn volumes(&mut self) -> Result<()> {
        for (suffix, output) in [("input", false), ("output", true)] {
            let name = format!("zryna410-{}-{suffix}", self.owner);
            let options = config::volume(output);
            self.volumes.push((name.clone(), options.clone()));
            let value = self.client.json(Method::Post,"/volumes/create",Some(&json!({
                "Name":name,"Driver":"local","DriverOpts":options,"Labels":{"zryna410.owner":self.owner}})))?;
            identity::volume(&value, &name, &self.owner, &options)?;
        }
        Ok(())
    }

    pub fn create(&mut self, holder: bool) -> Result<usize> {
        if self.volumes.len() != 2 {
            return Err(reject("PLAYGROUND-HOST-STATE", "two owned tmpfs volumes required"));
        }
        let role = if holder { "holder" } else { self.operation.name() };
        let name = format!("zryna410-{}-{role}-{}", self.owner, self.containers.len());
        let digest = if holder || self.operation == Operation::Compile {
            &self.approval.record.compiler_image_sha256
        } else {
            &self.approval.record.evaluator_image_sha256
        };
        let image = format!("sha256:{digest}");
        let requested = config::container(
            self.operation,
            holder,
            self.approval,
            &image,
            &self.owner,
            [&self.volumes[0].0, &self.volumes[1].0],
        );
        let index = self.containers.len();
        // Own the attempted name before sending a mutating create request.
        self.containers.push(Container {
            name: name.clone(),
            cid: None,
            requested: requested.clone(),
            init: None,
            removed: false,
        });
        let value = self.client.json(
            Method::Post,
            &format!("/containers/create?name={name}"),
            Some(&requested),
        )?;
        let cid = string(&value, "Id")?;
        if !hash(cid) {
            return Err(reject(
                "PLAYGROUND-HOST-ENFORCEMENT",
                "full created container identity required",
            ));
        }
        self.containers[index].cid = Some(cid.to_owned());
        self.inspect(index)?;
        Ok(index)
    }

    pub fn id(&self, index: usize) -> Result<&str> {
        self.containers
            .get(index)
            .filter(|item| !item.removed)
            .and_then(|item| item.cid.as_deref())
            .ok_or_else(|| reject("PLAYGROUND-HOST-STATE", "owned live role identity absent"))
    }

    pub fn inspect(&self, index: usize) -> Result<Value> {
        let item = self
            .containers
            .get(index)
            .ok_or_else(|| reject("PLAYGROUND-HOST-STATE", "unknown role index"))?;
        let cid = self.id(index)?;
        let value = self.client.json(Method::Get, &format!("/containers/{cid}/json"), None)?;
        let image = string(&item.requested, "Image")?;
        identity::validate(&value, cid, &item.name, image, &item.requested)?;
        Ok(value)
    }

    pub fn start(&mut self, index: usize) -> Result<()> {
        self.inspect(index)?;
        self.client.empty(
            Method::Post,
            &format!("/containers/{}/start", self.id(index)?),
            &[204],
        )?;
        let value = self.inspect(index)?;
        self.containers[index].init = Some(kernel::capture(
            &value,
            self.id(index)?,
            self.approval.record.cpus,
            self.client.deadline,
        )?);
        self.live(index)
    }

    pub fn live(&self, index: usize) -> Result<()> {
        let value = self.inspect(index)?;
        let init = self.containers[index]
            .init
            .as_ref()
            .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "original init identity absent"))?;
        kernel::same(
            init,
            &value,
            self.id(index)?,
            self.approval.record.cpus,
            self.client.deadline,
        )?;
        let stats = self.client.json(
            Method::Get,
            &format!("/containers/{}/stats?stream=false&one-shot=true", self.id(index)?),
            None,
        )?;
        stats_match(&stats, &self.containers[index].requested["HostConfig"])?;
        let after = self.inspect(index)?;
        kernel::same(init, &after, self.id(index)?, self.approval.record.cpus, self.client.deadline)
    }

    pub fn upload(&self, holder: usize, bytes: &[u8]) -> Result<()> {
        self.live(holder)?;
        let response = self.client.bytes(
            Method::Put,
            &format!(
                "/containers/{}/archive?path=%2Finput&noOverwriteDirNonDir=1",
                self.id(holder)?
            ),
            Body::Tar(bytes),
            archive::LIMIT,
            &[200],
        )?;
        if !response.is_empty() {
            return Err(reject("PLAYGROUND-HOST-TRANSPORT", "unexpected input upload response"));
        }
        Ok(())
    }

    pub fn archive(&self, holder: usize, output: bool) -> Result<Vec<u8>> {
        self.live(holder)?;
        let path = if output { "%2Foutput" } else { "%2Finput" };
        let bytes = self.client.bytes(
            Method::Get,
            &format!("/containers/{}/archive?path={path}", self.id(holder)?),
            Body::Empty,
            archive::LIMIT,
            &[200],
        )?;
        self.live(holder)?;
        Ok(bytes)
    }

    pub fn stop(&self, index: usize) -> Result<()> {
        self.quiescent(index)?;
        let cid = self.id(index)?;
        self.client.empty(Method::Post, &format!("/containers/{cid}/kill?signal=KILL"), &[204])?;
        let waited = self.client.json(
            Method::Post,
            &format!("/containers/{cid}/wait?condition=not-running"),
            None,
        )?;
        let value = self.inspect(index)?;
        if waited["StatusCode"] != 137
            || !waited["Error"].is_null()
            || value["State"]["Running"] != false
            || value["State"]["Pid"] != 0
            || value["State"]["Restarting"] != false
            || value["State"]["OOMKilled"] != false
        {
            return Err(reject(
                "PLAYGROUND-HOST-TEARDOWN",
                "private namespace stop or wait unproved",
            ));
        }
        kernel::gone(
            self.containers[index]
                .init
                .as_ref()
                .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "original init identity absent"))?,
            self.client.deadline,
        )
    }

    pub(super) fn quiescent(&self, index: usize) -> Result<()> {
        loop {
            self.live(index)?;
            let init = self.containers[index]
                .init
                .as_ref()
                .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "original init identity absent"))?;
            let value = self.client.json(
                Method::Get,
                &format!("/containers/{}/stats?stream=false&one-shot=true", self.id(index)?),
                None,
            )?;
            if value["pids_stats"]["current"] == 1
                && kernel::children_reaped(init, self.client.deadline)?
            {
                let after = self.client.json(
                    Method::Get,
                    &format!("/containers/{}/stats?stream=false&one-shot=true", self.id(index)?),
                    None,
                )?;
                stats_match(&after, &self.containers[index].requested["HostConfig"])?;
                kernel::same(
                    init,
                    &self.inspect(index)?,
                    self.id(index)?,
                    self.approval.record.cpus,
                    self.client.deadline,
                )?;
                if after["pids_stats"]["current"] == 1 {
                    return Ok(());
                }
            }
            self.client.check()?;
            thread::sleep(Duration::from_millis(10));
        }
    }
}

pub(super) fn stats_match(value: &Value, host: &Value) -> Result<()> {
    if value["memory_stats"]["limit"] != host["Memory"]
        || value["pids_stats"]["limit"] != host["PidsLimit"]
        || value["pids_stats"]["current"]
            .as_u64()
            .is_none_or(|count| count == 0 || count > host["PidsLimit"].as_u64().unwrap_or(0))
    {
        return Err(reject(
            "PLAYGROUND-HOST-KERNEL",
            "actual memory or whole-tree PID control unavailable",
        ));
    }
    Ok(())
}
