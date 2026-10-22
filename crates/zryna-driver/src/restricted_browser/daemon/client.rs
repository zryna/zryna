use super::{Result, reject};
use crate::native::recipe::engine::{self, Body, Method};
use serde_json::Value;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

#[derive(Clone, Copy)]
pub(super) struct Client<'a> {
    pub deadline: Instant,
    pub engine_uid: u32,
    pub cancel: Option<&'a AtomicBool>,
}

impl Client<'_> {
    pub fn check(&self) -> Result<()> {
        if self.cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
            return Err(reject(
                "PLAYGROUND-CANCELLED",
                "operation cancelled before further effects",
            ));
        }
        if Instant::now() >= self.deadline {
            return Err(reject(
                "PLAYGROUND-HOST-DEADLINE",
                "complete host operation deadline exceeded",
            ));
        }
        Ok(())
    }

    pub fn request(
        &self,
        method: Method,
        path: &str,
        body: Body<'_>,
        limit: usize,
    ) -> Result<engine::Response> {
        self.check()?;
        let result = self
            .cancel
            .map_or_else(
                || engine::request(self.deadline, self.engine_uid, method, path, body, limit),
                |cancel| {
                    engine::request_cancellable(
                        self.deadline,
                        self.engine_uid,
                        method,
                        path,
                        body,
                        limit,
                        cancel,
                    )
                },
            )
            .map_err(|error| {
                if self.cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
                    reject("PLAYGROUND-CANCELLED", "operation cancelled during daemon I/O")
                } else if error == engine::Error::Deadline {
                    reject("PLAYGROUND-HOST-DEADLINE", "bounded local daemon operation timed out")
                } else {
                    reject(
                        "PLAYGROUND-HOST-TRANSPORT",
                        "bounded fixed local daemon operation failed",
                    )
                }
            })?;
        self.check()?;
        Ok(result)
    }

    pub fn bytes(
        &self,
        method: Method,
        path: &str,
        body: Body<'_>,
        limit: usize,
        statuses: &[u16],
    ) -> Result<Vec<u8>> {
        let response = self.request(method, path, body, limit)?;
        if !statuses.contains(&response.status) {
            return Err(reject(
                "PLAYGROUND-HOST-ENFORCEMENT",
                "fixed local daemon operation refused",
            ));
        }
        Ok(response.body)
    }

    pub fn json(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value> {
        let encoded = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| reject("PLAYGROUND-HOST-TRANSPORT", "fixed request encoding failed"))?;
        let body = encoded.as_deref().map_or(Body::Empty, Body::Json);
        let bytes = self.bytes(method, path, body, 262_144, &[200, 201])?;
        serde_json::from_slice(&bytes)
            .map_err(|_| reject("PLAYGROUND-HOST-TRANSPORT", "bounded daemon JSON invalid"))
    }

    pub fn empty(&self, method: Method, path: &str, statuses: &[u16]) -> Result<()> {
        if !self.bytes(method, path, Body::Empty, 262_144, statuses)?.is_empty() {
            return Err(reject("PLAYGROUND-HOST-TRANSPORT", "unexpected daemon response bytes"));
        }
        Ok(())
    }

    pub fn lookup(&self, kind: &str, name: &str) -> Result<Option<Value>> {
        let path = match kind {
            "containers" => format!("/containers/{name}/json"),
            "volumes" => format!("/volumes/{name}"),
            _ => return Err(reject("PLAYGROUND-HOST-STATE", "unknown owned resource kind")),
        };
        let response = self.request(Method::Get, &path, Body::Empty, 262_144)?;
        match response.status {
            404 => Ok(None),
            200 => serde_json::from_slice(&response.body).map(Some).map_err(|_| {
                reject("PLAYGROUND-HOST-TRANSPORT", "invalid owned resource identity")
            }),
            _ => Err(reject("PLAYGROUND-HOST-TRANSPORT", "owned resource inspection failed")),
        }
    }
}

pub(super) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| reject("PLAYGROUND-HOST-ENFORCEMENT", "typed daemon identity missing"))
}
