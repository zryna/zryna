use std::{
    fmt,
    io::{self, BufRead, Write},
};

/// Maximum accepted JSON-RPC message bytes, including escaped source text.
pub const MAX_LSP_MESSAGE_BYTES: usize = 16 * 1_024 * 1_024;
const MAX_HEADER_BYTES: usize = 8 * 1_024;

/// Stable framing failure.
#[derive(Debug)]
pub enum FrameError {
    /// The underlying stream failed.
    Io(io::Error),
    /// Header syntax, inventory, or content length was invalid.
    Header,
    /// The declared payload exceeds the admission limit.
    Limit,
    /// The stream ended before the declared payload was complete.
    Truncated,
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Io(_) => "language-server transport I/O failed",
            Self::Header => "language-server frame header is malformed",
            Self::Limit => "language-server frame exceeds the byte limit",
            Self::Truncated => "language-server frame payload is truncated",
        })
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for FrameError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Reads one LSP `Content-Length` frame, returning `None` only for clean EOF between frames.
///
/// # Errors
///
/// Returns an error for invalid or oversized headers, oversized payloads, truncated frames, or
/// underlying input failures.
pub fn read_frame(input: &mut impl BufRead) -> Result<Option<Vec<u8>>, FrameError> {
    let mut header_bytes = 0_usize;
    let mut content_length = None;
    let mut content_type = false;
    loop {
        let mut line = Vec::new();
        let read = input.read_until(b'\n', &mut line)?;
        if read == 0 {
            return if header_bytes == 0 { Ok(None) } else { Err(FrameError::Truncated) };
        }
        header_bytes = header_bytes.checked_add(read).ok_or(FrameError::Limit)?;
        if header_bytes > MAX_HEADER_BYTES {
            return Err(FrameError::Limit);
        }
        if !line.ends_with(b"\r\n") {
            return Err(FrameError::Header);
        }
        line.truncate(line.len() - 2);
        if line.is_empty() {
            break;
        }
        let text = std::str::from_utf8(&line).map_err(|_| FrameError::Header)?;
        let (name, value) = text.split_once(':').ok_or(FrameError::Header)?;
        let value = value.trim_matches([' ', '\t']);
        if name.eq_ignore_ascii_case("Content-Length") {
            if content_length.is_some()
                || value.is_empty()
                || !value.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(FrameError::Header);
            }
            let parsed = value.parse::<usize>().map_err(|_| FrameError::Limit)?;
            if parsed > MAX_LSP_MESSAGE_BYTES {
                return Err(FrameError::Limit);
            }
            content_length = Some(parsed);
        } else if name.eq_ignore_ascii_case("Content-Type") {
            if content_type
                || !value.eq_ignore_ascii_case("application/vscode-jsonrpc; charset=utf-8")
            {
                return Err(FrameError::Header);
            }
            content_type = true;
        } else {
            return Err(FrameError::Header);
        }
    }
    let length = content_length.ok_or(FrameError::Header)?;
    let mut payload = vec![0_u8; length];
    input.read_exact(&mut payload).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            FrameError::Truncated
        } else {
            FrameError::Io(error)
        }
    })?;
    Ok(Some(payload))
}

/// Writes one canonical LSP frame.
///
/// # Errors
///
/// Returns an error when the payload exceeds the transport limit or the output fails.
pub fn write_frame(output: &mut impl Write, payload: &[u8]) -> Result<(), FrameError> {
    if payload.len() > MAX_LSP_MESSAGE_BYTES {
        return Err(FrameError::Limit);
    }
    write!(output, "Content-Length: {}\r\n\r\n", payload.len())?;
    output.write_all(payload)?;
    Ok(())
}
