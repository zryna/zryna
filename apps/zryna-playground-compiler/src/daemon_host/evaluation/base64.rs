//! Canonical RFC 4648 encoding only; allocation is bounded before decoding.

pub(super) fn decode(text: &str, maximum: usize) -> Result<Vec<u8>, &'static str> {
    const ERROR: &str = "PLAYGROUND-EVALUATION-REQUEST";
    if text.is_empty() || !text.len().is_multiple_of(4) || text.len() > maximum.div_ceil(3) * 4 {
        return Err(ERROR);
    }
    let value = |byte| match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut result = Vec::with_capacity((text.len() / 4 * 3).min(maximum));
    for (index, group) in text.as_bytes().chunks_exact(4).enumerate() {
        let last = index == text.len() / 4 - 1;
        let a = value(group[0]).ok_or(ERROR)?;
        let b = value(group[1]).ok_or(ERROR)?;
        let c = if group[2] == b'=' { 0 } else { value(group[2]).ok_or(ERROR)? };
        let d = if group[3] == b'=' { 0 } else { value(group[3]).ok_or(ERROR)? };
        let count = if group[2] == b'=' {
            if !last || group[3] != b'=' || b & 15 != 0 {
                return Err(ERROR);
            }
            1
        } else if group[3] == b'=' {
            if !last || c & 3 != 0 {
                return Err(ERROR);
            }
            2
        } else {
            3
        };
        if count > maximum - result.len() {
            return Err(ERROR);
        }
        let decoded = [(a << 2) | (b >> 4), (b << 4) | (c >> 2), (c << 6) | d];
        result.extend_from_slice(&decoded[..count]);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::decode;
    #[test]
    fn exact_last_byte_and_canonical_padding() {
        assert_eq!(decode("YWJj", 3).expect("exact"), b"abc");
        assert!(decode("YWJj", 2).is_err());
        assert_eq!(decode("YQ==", 1).expect("one"), b"a");
        assert!(decode("YR==", 3).is_err());
        assert!(decode("YWJ=", 3).is_err());
        for invalid in ["", "YQ=", "YQ==AAAA", "YQ==\n", "YW-J", "Y=Q="] {
            assert!(decode(invalid, 32).is_err(), "{invalid:?}");
        }
    }
}
