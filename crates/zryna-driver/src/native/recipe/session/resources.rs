//! Own every attempted daemon resource before sending its create operation.
use super::client::{Client, digest_id, string};
use super::config::{self, Role, Volume};
use super::identity;
use super::kernel::{self, Init};
use super::{Result, fail};
use crate::native::recipe::engine::{Body, Method};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub(super) struct Container {
    pub name: String,
    pub cid: Option<String>,
    pub requested: Value,
    pub init: Option<Init>,
}
pub(super) struct Resources {
    pub owner: String,
    pub image: String,
    pub volumes: Vec<(String, Value)>,
    pub containers: Vec<Container>,
    pub client: Client,
}
impl Resources {
    pub fn volumes(&mut self) -> Result<()> {
        for (suffix, role) in
            [("input", Volume::Input), ("output", Volume::Output), ("control", Volume::Control)]
        {
            let name = format!("zryna405-{}-{suffix}", self.owner);
            let options = config::volumeopts(role);
            self.volumes.push((name.clone(), options.clone()));
            let state = self.client.json(Method::Post, "/volumes/create", Some(&json!({"Name":name,"Driver":"local","DriverOpts":options,"Labels":{"zryna405.owner":self.owner}})))?;
            identity::validate_volume(&state, &name, &self.owner, &options)?;
        }
        Ok(())
    }
    pub fn create(&mut self, role: Role) -> Result<usize> {
        if self.volumes.len() != 3 {
            return Err(fail("complete owned volumes required"));
        }
        let suffix = if role == Role::Holder { "holder" } else { "recipe" };
        let name = format!("zryna405-{}-{suffix}", self.owner);
        let requested = config::config(
            role,
            &self.image,
            &self.owner,
            [&self.volumes[0].0, &self.volumes[1].0, &self.volumes[2].0],
        );
        let index = self.containers.len();
        self.containers.push(Container {
            name: name.clone(),
            cid: None,
            requested: requested.clone(),
            init: None,
        });
        let state = self.client.json(
            Method::Post,
            &format!("/containers/create?name={name}"),
            Some(&requested),
        )?;
        let cid = string(&state, "Id")?;
        if !digest_id(cid) {
            return Err(fail("full created container ID required"));
        }
        self.containers[index].cid = Some(cid.to_owned());
        self.inspect(index)?;
        Ok(index)
    }
    pub fn id(&self, index: usize) -> Result<&str> {
        self.containers
            .get(index)
            .and_then(|item| item.cid.as_deref())
            .ok_or_else(|| fail("owned role unavailable"))
    }
    pub fn inspect(&self, index: usize) -> Result<Value> {
        let item = self.containers.get(index).ok_or_else(|| fail("owned role unavailable"))?;
        let cid = self.id(index)?;
        let state = self.client.json(Method::Get, &format!("/containers/{cid}/json"), None)?;
        identity::validate(&state, cid, &item.name, &self.owner, &self.image, &item.requested)?;
        Ok(state)
    }
    pub fn start(&mut self, index: usize) -> Result<()> {
        self.inspect(index)?;
        self.client.empty(
            Method::Post,
            &format!("/containers/{}/start", self.id(index)?),
            &[204],
        )?;
        let state = self.inspect(index)?;
        self.containers[index].init =
            Some(kernel::capture(&state, self.id(index)?, self.client.deadline)?);
        self.live(index)
    }
    pub fn live(&self, index: usize) -> Result<()> {
        let state = self.inspect(index)?;
        let init =
            self.containers[index].init.as_ref().ok_or_else(|| fail("original init absent"))?;
        kernel::same(init, &state, self.id(index)?, self.client.deadline)?;
        let measurements = self.client.json(
            Method::Get,
            &format!("/containers/{}/stats?stream=false&one-shot=true", self.id(index)?),
            None,
        )?;
        let host = &self.containers[index].requested["HostConfig"];
        check_stats(&measurements, self.id(index)?, &self.containers[index].name, host)?;
        let after = self.inspect(index)?;
        kernel::same(init, &after, self.id(index)?, self.client.deadline)?;
        Ok(())
    }
    pub fn stop_recipe(&self, index: usize) -> Result<()> {
        self.live(index)?;
        let cid = self.id(index)?;
        self.client.empty(Method::Post, &format!("/containers/{cid}/kill?signal=KILL"), &[204])?;
        let waited = self.client.json(
            Method::Post,
            &format!("/containers/{cid}/wait?condition=not-running"),
            None,
        )?;
        let state = self.inspect(index)?;
        if waited["StatusCode"] != 137
            || !waited["Error"].is_null()
            || state["State"]["Running"] != false
            || state["State"]["Pid"] != 0
            || state["State"]["OOMKilled"] != false
            || state["State"]["Restarting"] != false
        {
            return Err(fail("private recipe namespace teardown unproved"));
        }
        kernel::gone(
            self.containers[index].init.as_ref().ok_or_else(|| fail("original init absent"))?,
            self.client.deadline,
        )
    }
    /// One shared independent cleanup deadline, including uncertain-create reconciliation.
    pub fn cleanup(&self) -> Result<()> {
        let client = Client {
            deadline: Instant::now() + Duration::from_secs(5),
            engine_uid: self.client.engine_uid,
        };
        let mut first_error = None;
        for item in self.containers.iter().rev() {
            let result = self.remove_container(client, item);
            if let Err(error) = result {
                first_error.get_or_insert(error);
            }
        }
        for (name, options) in self.volumes.iter().rev() {
            let result = self.remove_volume(client, name, options);
            if let Err(error) = result {
                first_error.get_or_insert(error);
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    fn remove_container(&self, client: Client, item: &Container) -> Result<()> {
        let state = client
            .lookup("containers", &item.name)?
            .ok_or_else(|| fail("owned or uncertain container disappeared before cleanup"))?;
        let cid = string(&state, "Id")?;
        if item.cid.as_deref().is_some_and(|expected| expected != cid) {
            return Err(fail("cleanup container identity changed"));
        }
        identity::validate(&state, cid, &item.name, &self.owner, &self.image, &item.requested)?;
        client.empty(Method::Delete, &format!("/containers/{cid}?force=1&v=0"), &[204])?;
        if client.lookup("containers", cid)?.is_some()
            || client.lookup("containers", &item.name)?.is_some()
        {
            return Err(fail("owned container cleanup unconfirmed"));
        }
        if let Some(init) = &item.init {
            kernel::gone(init, client.deadline)?;
        }
        Ok(())
    }
    fn remove_volume(&self, client: Client, name: &str, options: &Value) -> Result<()> {
        let state = client
            .lookup("volumes", name)?
            .ok_or_else(|| fail("owned volume disappeared before cleanup"))?;
        identity::validate_volume(&state, name, &self.owner, options)?;
        client.empty(Method::Delete, &format!("/volumes/{name}"), &[204])?;
        if client.lookup("volumes", name)?.is_some() {
            return Err(fail("owned volume cleanup unconfirmed"));
        }
        Ok(())
    }
    pub fn upload_input(&self, holder: usize, bytes: &[u8]) -> Result<()> {
        self.live(holder)?;
        let body = self.client.bytes(
            Method::Put,
            &format!(
                "/containers/{}/archive?path=%2Finput&noOverwriteDirNonDir=1",
                self.id(holder)?
            ),
            Body::Tar(bytes),
            2 * 1024 * 1024,
            &[200],
        )?;
        if !body.is_empty() {
            return Err(fail("unexpected archive upload response"));
        }
        Ok(())
    }
    pub fn archive(&self, holder: usize, path: &str, limit: usize) -> Result<Vec<u8>> {
        self.client.bytes(
            Method::Get,
            &format!("/containers/{}/archive?path={path}", self.id(holder)?),
            Body::Empty,
            limit,
            &[200],
        )
    }
}

fn check_stats(measurements: &Value, cid: &str, name: &str, host: &Value) -> Result<()> {
    // The authenticated request names the full immutable CID. Docker may omit optional ID data.
    if measurements.get("id").is_some_and(|id| id != cid)
        || measurements.get("name").is_some_and(|actual| actual != &format!("/{name}"))
        || measurements["memory_stats"]["limit"] != host["Memory"]
        || measurements["pids_stats"]["limit"] != host["PidsLimit"]
        || measurements["pids_stats"]["current"]
            .as_u64()
            .is_none_or(|count| count == 0 || count > host["PidsLimit"].as_u64().unwrap_or(0))
    {
        return Err(fail("actual memory or PID control unavailable"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_stats_require_controls_even_when_optional_id_is_omitted() {
        let host = json!({"Memory":16_777_216,"PidsLimit":2});
        let original =
            json!({"memory_stats":{"limit":16_777_216},"pids_stats":{"limit":2,"current":1}});
        assert!(check_stats(&original, "owned", "owner-name", &host).is_ok());
        for id in [json!("other"), Value::Null, json!(1)] {
            let mut changed = original.clone();
            changed["id"] = id;
            assert!(check_stats(&changed, "owned", "owner-name", &host).is_err());
        }
        for name in [json!("/other"), Value::Null, json!(1)] {
            let mut changed = original.clone();
            changed["name"] = name;
            assert!(check_stats(&changed, "owned", "owner-name", &host).is_err());
        }
        for (path, value) in [
            ("memory_stats", json!({})),
            ("pids_stats", json!({"limit":2})),
            ("pids_stats", json!({"limit":3,"current":1})),
            ("pids_stats", json!({"limit":2,"current":0})),
            ("pids_stats", json!({"limit":2,"current":3})),
        ] {
            let mut changed = original.clone();
            changed[path] = value;
            assert!(check_stats(&changed, "owned", "owner-name", &host).is_err());
        }
        let mut identified = original.clone();
        identified["id"] = json!("owned");
        identified["name"] = json!("/owner-name");
        assert!(check_stats(&identified, "owned", "owner-name", &host).is_ok());
    }
}
