//! Fixed local Engine transport; no environment-selected endpoints or credentials.
#![cfg(target_os = "linux")]

mod frames;
mod headers;
mod http;
mod types;
mod wire;

use std::io::Write;
use std::time::Instant;

pub(super) use types::{Body, Error, Method, Response, Streams};

pub(super) fn request(
    deadline: Instant,
    expected_uid: u32,
    method: Method,
    path: &str,
    body: Body<'_>,
    body_limit: usize,
) -> Result<Response, Error> {
    types::remaining(deadline)?;
    let header = types::request_header(method, path, body, body_limit)?;
    let mut stream = wire::open(deadline, expected_uid)?;
    stream.write_all(header.as_bytes()).map_err(|_| types::io_error(deadline))?;
    stream.write_all(body.bytes()).map_err(|_| types::io_error(deadline))?;
    types::remaining(deadline)?;
    http::response(stream, deadline, body_limit)
}

pub(super) fn parse_frames(deadline: Instant, bytes: &[u8]) -> Result<Streams, Error> {
    frames::parse(deadline, bytes)
}

#[cfg(test)]
mod tests;
