use serde::{Deserialize, Serialize};

/// Negotiated LSP position encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PositionEncoding {
    Utf8,
    Utf16,
    Utf32,
}

impl PositionEncoding {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf16 => "utf-16",
            Self::Utf32 => "utf-32",
        }
    }

    pub(crate) fn select(values: Option<&[String]>) -> Self {
        values
            .and_then(|values| {
                values.iter().find_map(|value| match value.as_str() {
                    "utf-8" => Some(Self::Utf8),
                    "utf-16" => Some(Self::Utf16),
                    "utf-32" => Some(Self::Utf32),
                    _ => None,
                })
            })
            .unwrap_or(Self::Utf16)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Position {
    pub(crate) line: u32,
    pub(crate) character: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Range {
    pub(crate) start: Position,
    pub(crate) end: Position,
}

pub(crate) fn position_to_byte(
    text: &str,
    position: Position,
    encoding: PositionEncoding,
) -> Option<u32> {
    let bounds = line_bounds(text, position.line)?;
    let line = text.get(bounds.start..bounds.content_end)?;
    let relative = match encoding {
        PositionEncoding::Utf8 => {
            let index = usize::try_from(position.character).ok()?;
            (index <= line.len() && line.is_char_boundary(index)).then_some(index)?
        }
        PositionEncoding::Utf16 => units_to_byte(line, position.character, char::len_utf16)?,
        PositionEncoding::Utf32 => units_to_byte(line, position.character, |_| 1)?,
    };
    u32::try_from(bounds.start.checked_add(relative)?).ok()
}

pub(crate) fn byte_range_to_positions(
    text: &str,
    start: u32,
    end: u32,
    encoding: PositionEncoding,
) -> Option<Range> {
    if start > end {
        return None;
    }
    Some(Range {
        start: byte_to_position(text, start, encoding)?,
        end: byte_to_position(text, end, encoding)?,
    })
}

fn byte_to_position(text: &str, offset: u32, encoding: PositionEncoding) -> Option<Position> {
    let offset = usize::try_from(offset).ok()?;
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let mut line = 0_u32;
    loop {
        let bounds = line_bounds(text, line)?;
        if offset <= bounds.content_end {
            let prefix = text.get(bounds.start..offset)?;
            let character = match encoding {
                PositionEncoding::Utf8 => u32::try_from(prefix.len()).ok()?,
                PositionEncoding::Utf16 => u32::try_from(prefix.encode_utf16().count()).ok()?,
                PositionEncoding::Utf32 => u32::try_from(prefix.chars().count()).ok()?,
            };
            return Some(Position { line, character });
        }
        if offset < bounds.next_start {
            return None;
        }
        line = line.checked_add(1)?;
    }
}

fn units_to_byte(text: &str, requested: u32, width: impl Fn(char) -> usize) -> Option<usize> {
    let mut units = 0_u32;
    for (offset, character) in text.char_indices() {
        if units == requested {
            return Some(offset);
        }
        let amount = u32::try_from(width(character)).ok()?;
        let next = units.checked_add(amount)?;
        if requested < next {
            return None;
        }
        units = next;
    }
    (units == requested).then_some(text.len())
}

#[derive(Clone, Copy)]
struct LineBounds {
    start: usize,
    content_end: usize,
    next_start: usize,
}

fn line_bounds(text: &str, requested: u32) -> Option<LineBounds> {
    let bytes = text.as_bytes();
    let mut line = 0_u32;
    let mut start = 0_usize;
    let mut index = 0_usize;
    loop {
        if index == bytes.len() {
            return (line == requested).then_some(LineBounds {
                start,
                content_end: bytes.len(),
                next_start: bytes.len(),
            });
        }
        let (content_end, next_start) = match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => (index, index + 2),
            b'\r' | b'\n' => (index, index + 1),
            _ => {
                index += 1;
                continue;
            }
        };
        if line == requested {
            return Some(LineBounds { start, content_end, next_start });
        }
        line = line.checked_add(1)?;
        start = next_start;
        index = next_start;
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, PositionEncoding, byte_range_to_positions, position_to_byte};

    const TEXT: &str = "// é😀e\u{301}\r\nnext\n";

    #[test]
    fn negotiated_units_preserve_unicode_crlf_and_scalar_boundaries() {
        for (encoding, character, byte) in [
            (PositionEncoding::Utf8, 9, 9),
            (PositionEncoding::Utf16, 6, 9),
            (PositionEncoding::Utf32, 5, 9),
        ] {
            assert_eq!(
                position_to_byte(TEXT, Position { line: 0, character }, encoding),
                Some(byte)
            );
            assert_eq!(
                byte_range_to_positions(TEXT, byte, byte, encoding).map(|range| range.start),
                Some(Position { line: 0, character })
            );
        }
        assert_eq!(
            position_to_byte(TEXT, Position { line: 1, character: 0 }, PositionEncoding::Utf16),
            Some(14)
        );
        assert!(
            position_to_byte(TEXT, Position { line: 0, character: 5 }, PositionEncoding::Utf16)
                .is_none()
        );
        assert!(byte_range_to_positions(TEXT, 13, 13, PositionEncoding::Utf8).is_none());
    }
}
