//! Acquire the trusted entrypoint's scheduling marker before the unchanged admission check.
use super::{Resources, Result, kernel, reject};
use serde_json::Value;
use std::{
    thread,
    time::{Duration, Instant},
};

impl Resources<'_> {
    pub(super) fn await_supervisor(&self, index: usize) -> Result<Value> {
        let original = self.containers[index]
            .init
            .as_ref()
            .ok_or_else(|| reject("PLAYGROUND-HOST-KERNEL", "original startup init absent"))?;
        let cid = self.id(index)?;
        loop {
            self.client.check()?;
            let before = self.inspect(index)?;
            kernel::startup_epoch(
                original,
                &kernel::capture_owned(&before, cid, self.client.deadline)?,
                &before,
            )?;
            self.client.check()?;
            let ready = kernel::supervisor_ready(original, self.client.deadline)?;
            self.client.check()?;
            let after = self.inspect(index)?;
            kernel::startup_epoch(
                original,
                &kernel::capture_owned(&after, cid, self.client.deadline)?,
                &after,
            )?;
            self.client.check()?;
            if ready {
                return Ok(after);
            }
            // The original operation clock includes this wait. No new grace or admission retry.
            thread::sleep(
                Duration::from_millis(1)
                    .min(self.client.deadline.saturating_duration_since(Instant::now())),
            );
        }
    }
}
