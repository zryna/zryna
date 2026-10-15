//! The owning session's fixed local daemon connection and bounded response parsing.
use super::super::engine::{self, Body, Method};
use super::{Failure, Result, expired, fail};
use serde_json::Value;
use std::time::Instant;

#[derive(Clone, Copy)]
pub(super) struct Client {
    pub deadline: Instant,
    pub engine_uid: u32,
}
impl Client {
    pub fn bytes(
        &self,
        method: Method,
        path: &str,
        body: Body<'_>,
        limit: usize,
        statuses: &[u16],
    ) -> Result<Vec<u8>> {
        let response = engine::request(self.deadline, self.engine_uid, method, path, body, limit)
            .map_err(transport)?;
        if !statuses.contains(&response.status) {
            return Err(fail("local Engine operation did not succeed"));
        }
        Ok(response.body)
    }
    pub fn json(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value> {
        let encoded = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| fail("cannot encode fixed request"))?;
        let body = encoded.as_deref().map_or(Body::Empty, Body::Json);
        let bytes = self.bytes(method, path, body, 256 * 1024, &[200, 201])?;
        serde_json::from_slice(&bytes).map_err(|_| fail("invalid Engine JSON response"))
    }
    pub fn empty(&self, method: Method, path: &str, statuses: &[u16]) -> Result<()> {
        let bytes = self.bytes(method, path, Body::Empty, 256 * 1024, statuses)?;
        if !bytes.is_empty() {
            return Err(fail("unexpected Engine operation body"));
        }
        Ok(())
    }
    pub fn lookup(&self, kind: &str, name: &str) -> Result<Option<Value>> {
        let path = if kind == "containers" {
            format!("/containers/{name}/json")
        } else {
            format!("/volumes/{name}")
        };
        let response = engine::request(
            self.deadline,
            self.engine_uid,
            Method::Get,
            &path,
            Body::Empty,
            256 * 1024,
        )
        .map_err(transport)?;
        match response.status {
            404 => Ok(None),
            200 => serde_json::from_slice(&response.body)
                .map(Some)
                .map_err(|_| fail("invalid owned resource inspection")),
            _ => Err(fail("cannot inspect owned resource")),
        }
    }
}
pub(super) fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value.get(key).and_then(Value::as_str).ok_or_else(|| fail("missing typed Engine identity"))
}
pub(super) fn digest_id(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
impl From<&'static str> for Failure {
    fn from(value: &'static str) -> Self {
        fail(value)
    }
}

fn transport(error: engine::Error) -> Failure {
    if error == engine::Error::Deadline {
        expired("bounded local Engine deadline exceeded")
    } else {
        fail("bounded local Engine transport failed")
    }
}
