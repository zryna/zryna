use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Deadline,
    Transport,
    Request,
    Http,
    Limit,
    Frames,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Method {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Body<'a> {
    Empty,
    Json(&'a [u8]),
    Tar(&'a [u8]),
}

impl<'a> Body<'a> {
    pub(super) fn bytes(self) -> &'a [u8] {
        match self {
            Self::Empty => &[],
            Self::Json(bytes) | Self::Tar(bytes) => bytes,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Streams {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

pub(super) fn remaining(deadline: Instant) -> Result<Duration, Error> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
        .ok_or(Error::Deadline)
}

pub(super) fn io_error(deadline: Instant) -> Error {
    if remaining(deadline).is_err() { Error::Deadline } else { Error::Transport }
}

pub(super) fn request_header(
    method: Method,
    path: &str,
    body: Body<'_>,
    limit: usize,
) -> Result<String, Error> {
    if path.len() > 4096
        || !path.starts_with('/')
        || path.starts_with("//")
        || !path.bytes().all(|byte| (0x21..=0x7e).contains(&byte) && !matches!(byte, b'#' | b'\\'))
    {
        return Err(Error::Request);
    }
    if body.bytes().len() > limit {
        return Err(Error::Limit);
    }
    let verb = match method {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Delete => "DELETE",
    };
    let content_type = match body {
        Body::Empty | Body::Json(_) => "application/json",
        Body::Tar(_) => "application/x-tar",
    };
    Ok(format!(
        "{verb} /v1.49{path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
        body.bytes().len()
    ))
}
