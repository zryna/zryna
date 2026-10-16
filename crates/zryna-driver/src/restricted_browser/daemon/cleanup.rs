use super::{
    Result,
    client::{Client, string},
    identity, kernel, reject,
    resources::Resources,
};
use crate::native::recipe::engine::Method;
use std::time::{Duration, Instant};

impl Resources<'_> {
    /// One separate two-second cleanup budget; cancellation never disables cleanup.
    pub fn cleanup(&mut self) -> Result<()> {
        let client = Client {
            deadline: Instant::now() + Duration::from_secs(2),
            engine_uid: self.client.engine_uid,
            cancel: None,
        };
        let mut primary = None;
        for index in (0..self.containers.len()).rev() {
            if self.containers[index].removed {
                continue;
            }
            if let Err(error) = self.remove_with(client, index) {
                primary.get_or_insert(error);
            }
        }
        for (name, options) in self.volumes.iter().rev() {
            let result = (|| {
                let value = client.lookup("volumes", name)?.ok_or_else(|| {
                    reject(
                        "PLAYGROUND-HOST-TEARDOWN",
                        "owned or uncertain volume absent before removal",
                    )
                })?;
                identity::volume(&value, name, &self.owner, options)?;
                client.empty(Method::Delete, &format!("/volumes/{name}"), &[204])?;
                if client.lookup("volumes", name)?.is_some() {
                    return Err(reject(
                        "PLAYGROUND-HOST-TEARDOWN",
                        "owned volume removal unconfirmed",
                    ));
                }
                Ok(())
            })();
            if let Err(error) = result {
                primary.get_or_insert(error);
            }
        }
        match primary {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Staging helper is removed before guest dispatch, keeping its overhead separate.
    pub fn remove(&mut self, index: usize) -> Result<()> {
        self.remove_with(self.client, index)
    }

    fn remove_with(&mut self, client: Client<'_>, index: usize) -> Result<()> {
        let item = self
            .containers
            .get(index)
            .ok_or_else(|| reject("PLAYGROUND-HOST-STATE", "unknown cleanup role"))?;
        if item.removed {
            return Ok(());
        }
        let value = client.lookup("containers", &item.name)?.ok_or_else(|| {
            reject("PLAYGROUND-HOST-TEARDOWN", "owned or uncertain create absent before removal")
        })?;
        let cid = string(&value, "Id")?;
        if item.cid.as_deref().is_some_and(|expected| expected != cid) {
            return Err(reject(
                "PLAYGROUND-HOST-TEARDOWN",
                "cleanup container identity substituted",
            ));
        }
        identity::validate(
            &value,
            cid,
            &item.name,
            string(&item.requested, "Image")?,
            &item.requested,
        )?;
        // Rejected admission cannot erase ownership or become a cleanup prerequisite.
        // A missing anchor cannot be replaced by a later process in the same container.
        kernel::cleanup_owned(item.init.as_ref(), &value, cid, client.deadline)?;
        let force = u8::from(value["State"]["Running"] == true);
        client.empty(Method::Delete, &format!("/containers/{cid}?force={force}&v=0"), &[204])?;
        if client.lookup("containers", cid)?.is_some()
            || client.lookup("containers", &item.name)?.is_some()
        {
            return Err(reject("PLAYGROUND-HOST-TEARDOWN", "owned container removal unconfirmed"));
        }
        if let Some(init) = &item.init {
            kernel::gone(init, client.deadline)?;
        }
        self.containers[index].removed = true;
        Ok(())
    }
}
