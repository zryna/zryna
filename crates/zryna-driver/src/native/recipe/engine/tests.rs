use std::io::{self, Cursor, Read};
use std::time::{Duration, Instant};

use super::types::{Body, Error, Method, request_header};
use super::{frames, http};

fn future() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

fn response(bytes: &[u8], limit: usize) -> Result<super::types::Response, Error> {
    http::response(Cursor::new(bytes), future(), limit)
}

#[test]
fn content_length_binary_exact_limit_and_first_extra() {
    let exact =
        response(b"HTTP/1.1 201 Created\r\nContent-Length: 3\r\n\r\n\0\xffa", 3).expect("exact");
    assert_eq!(exact.status, 201);
    assert_eq!(exact.body, b"\0\xffa");
    assert_eq!(
        response(b"HTTP/1.1 500 Error\r\nContent-Length: 4\r\n\r\nfail", 3),
        Err(Error::Limit)
    );
    assert_eq!(
        response(b"HTTP/1.1 500 Error\r\nContent-Length: 4\r\n\r\nfail", 4).expect("status").status,
        500
    );
}

#[test]
fn chunked_and_close_delimited_are_bounded_and_binary() {
    assert_eq!(
        response(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n\0x\r\n1\r\ny\r\n0\r\n\r\n",
            3
        )
        .expect("chunked")
        .body,
        b"\0xy"
    );
    assert_eq!(response(b"HTTP/1.1 200 OK\r\n\r\nabc", 3).expect("close").body, b"abc");
    assert_eq!(response(b"HTTP/1.1 200 OK\r\n\r\nabcd", 3), Err(Error::Limit));
    assert_eq!(
        response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nabcd\r\n0\r\n\r\n", 3),
        Err(Error::Limit)
    );
}

#[test]
fn ambiguous_framing_and_http_control_fields_refuse() {
    for fields in [
        "Content-Length: 0\r\nContent-Length: 0\r\n",
        "Content-Length: 0\r\nTransfer-Encoding: chunked\r\n",
        "Transfer-Encoding: gzip, chunked\r\n",
        "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n",
        "Content-Length: +0\r\n",
        "Content-Length: 184467440737095516160\r\n",
        "Connection: Upgrade\r\n",
        "Trailer: X\r\n",
        "Upgrade: tcp\r\n",
        "Content-Encoding: gzip\r\n",
        " Folded: value\r\n",
        "X: bad\tvalue\r\n",
    ] {
        let bytes = format!("HTTP/1.1 200 OK\r\n{fields}\r\n");
        assert_eq!(response(bytes.as_bytes(), 10), Err(Error::Http), "{fields}");
    }
}

#[test]
fn truncated_smuggled_and_non_http11_responses_refuse() {
    for bytes in [
        b"HTTP/1.0 200 OK\r\n\r\n".as_slice(),
        b"HTTP/1.1 101 Switching\r\n\r\n",
        b"HTTP/1.1 099 Invalid\r\n\r\n",
        b"HTTP/1.1 600 Invalid\r\n\r\n",
        b"HTTP/1.1 200 OK\n\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nx",
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\nx",
        b"HTTP/1.1 204 Empty\r\nContent-Length: 1\r\n\r\nx",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;x=y\r\na\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX: y\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\naX\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nffffffffffffffffffffffff\r\n",
    ] {
        assert!(response(bytes, 64).is_err(), "{bytes:?}");
    }
}

#[test]
fn header_field_and_whole_header_exact_first_extra() {
    let fields = format!("HTTP/1.1 200 OK\r\n{}\r\n", "X: y\r\n".repeat(64));
    assert!(response(fields.as_bytes(), 0).is_ok());
    let extra = format!("HTTP/1.1 200 OK\r\n{}\r\n", "X: y\r\n".repeat(65));
    assert_eq!(response(extra.as_bytes(), 0), Err(Error::Limit));
    let prefix = "HTTP/1.1 200 OK\r\nX: ";
    let exact = format!("{prefix}{}\r\n\r\n", "a".repeat(16384 - prefix.len() - 4));
    assert_eq!(exact.len(), 16384);
    assert!(response(exact.as_bytes(), 0).is_ok());
    let extra = exact.replacen("X: ", "X: a", 1);
    assert_eq!(response(extra.as_bytes(), 0), Err(Error::Limit));
}

#[test]
fn chunk_count_includes_terminal_chunk_and_cannot_run_unbounded() {
    let prefix = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n";
    let exact = format!("{prefix}{}0\r\n\r\n", "1\r\nx\r\n".repeat(4095));
    assert_eq!(response(exact.as_bytes(), 4096).expect("bounded").body.len(), 4095);
    let extra = format!("{prefix}{}0\r\n\r\n", "1\r\nx\r\n".repeat(4096));
    assert_eq!(response(extra.as_bytes(), 4096), Err(Error::Limit));
}

fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let length = u32::try_from(payload.len()).expect("small payload");
    let mut bytes = vec![kind, 0, 0, 0];
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn native_stream_exact_first_extra_and_separate_counters() {
    let mut bytes = frame(1, &vec![b'x'; 65536]);
    bytes.extend(frame(2, &vec![b'y'; 65536]));
    let parsed = frames::parse(future(), &bytes).expect("exact streams");
    assert_eq!(parsed.stdout.len(), 65536);
    assert_eq!(parsed.stderr.len(), 65536);
    bytes.extend(frame(1, b"z"));
    assert_eq!(frames::parse(future(), &bytes), Err(Error::Limit));
    let mut many = Vec::new();
    for _ in 0..4096 {
        many.extend(frame(1, b""));
    }
    assert!(frames::parse(future(), &many).is_ok());
    many.extend(frame(2, b""));
    assert_eq!(frames::parse(future(), &many), Err(Error::Limit));
}

#[test]
fn native_stream_reserved_stdin_truncation_and_trailing_fragment_refuse() {
    for bytes in [
        vec![1],
        frame(0, b"x"),
        frame(3, b"x"),
        vec![1, 1, 0, 0, 0, 0, 0, 0],
        vec![1, 0, 0, 0, 0, 0, 0, 2, b'x'],
    ] {
        assert!(frames::parse(future(), &bytes).is_err());
    }
    let mut trailing = frame(1, b"x");
    trailing.push(0);
    assert_eq!(frames::parse(future(), &trailing), Err(Error::Frames));
    assert!(frames::parse(future(), b"").is_ok());
}

struct NeverRead;
impl Read for NeverRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("expired deadline read");
    }
}

#[test]
fn expired_absolute_deadline_refuses_before_read_or_frame_parse() {
    let expired = Instant::now().checked_sub(Duration::from_secs(1)).expect("past");
    assert_eq!(http::response(NeverRead, expired, 10), Err(Error::Deadline));
    assert_eq!(frames::parse(expired, b""), Err(Error::Deadline));
}

#[test]
fn request_shape_is_fixed_no_header_injection_or_body_cap_bypass() {
    let header =
        request_header(Method::Put, "/containers/id/archive?path=/input", Body::Tar(b"abc"), 3)
            .expect("tar");
    assert!(header.starts_with("PUT /v1.49/containers/id/archive?path=/input HTTP/1.1\r\n"));
    assert!(header.contains("Content-Type: application/x-tar\r\nContent-Length: 3\r\n"));
    assert!(header.contains("Connection: close\r\n"));
    for path in ["", "//other/path", "/x\r\nInjected: y", "/space here", "/é", "/#fragment", "/\\x"]
    {
        assert_eq!(request_header(Method::Get, path, Body::Empty, 0), Err(Error::Request));
    }
    assert_eq!(request_header(Method::Post, "/exec", Body::Json(b"{}"), 1), Err(Error::Limit));
    assert!(request_header(Method::Delete, "/containers/id?force=1", Body::Empty, 0).is_ok());
}

struct CrossDeadline(Instant);
impl Read for CrossDeadline {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        while Instant::now() < self.0 {
            std::hint::spin_loop();
        }
        bytes[0] = b'H';
        Ok(1)
    }
}

#[test]
fn data_returned_after_original_deadline_cannot_renew_or_complete_response() {
    let deadline = Instant::now() + Duration::from_millis(2);
    assert_eq!(http::response(CrossDeadline(deadline), deadline, 10), Err(Error::Deadline));
}
