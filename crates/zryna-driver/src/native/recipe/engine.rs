//! Fixed local Engine transport; no environment-selected endpoints or credentials.
#![cfg(target_os = "linux")]

mod frames;
mod headers;
mod http;
mod types;
mod wire;

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub(crate) use types::{Body, Error, Method, Response, Streams};

pub(crate) fn request(
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

pub(crate) fn parse_frames(deadline: Instant, bytes: &[u8]) -> Result<Streams, Error> {
    frames::parse(deadline, bytes)
}

/// Cancellation is an additional internal caller condition; the native request path is unchanged.
pub(crate) fn request_cancellable(
    deadline: Instant,
    expected_uid: u32,
    method: Method,
    path: &str,
    body: Body<'_>,
    body_limit: usize,
    cancel: &AtomicBool,
) -> Result<Response, Error> {
    let check = || {
        types::remaining(deadline)?;
        if cancel.load(Ordering::Acquire) { Err(Error::Transport) } else { Ok(()) }
    };
    check()?;
    let header = types::request_header(method, path, body, body_limit)?;
    check()?;
    let mut stream = wire::open_cancellable(deadline, expected_uid, cancel)?;
    stream.write_all(header.as_bytes()).map_err(|_| types::io_error(deadline))?;
    check()?;
    stream.write_all(body.bytes()).map_err(|_| types::io_error(deadline))?;
    check()?;
    let response = http::response(stream, deadline, body_limit)?;
    check()?;
    Ok(response)
}

#[cfg(test)]
mod tests;
