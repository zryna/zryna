//! Build-bound identity of the prepared distribution, independent of the CLI's own bytes.

const PREFIX: &[u8] = b"ZRYNA-DISTRIBUTION-V1\0";
const IDENTITY_BYTES: usize = PREFIX.len() + 64 + 1;

static IDENTITY: [u8; IDENTITY_BYTES] = build_identity();

const fn build_identity() -> [u8; IDENTITY_BYTES] {
    let mut identity = [0; IDENTITY_BYTES];
    let mut index = 0;
    while index < PREFIX.len() {
        identity[index] = PREFIX[index];
        index += 1;
    }
    if let Some(value) = option_env!("ZRYNA_DISTRIBUTION_SHA256") {
        let bytes = value.as_bytes();
        assert!(bytes.len() == 64, "distribution digest must contain 64 lowercase hex bytes");
        index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            assert!(
                (byte >= b'0' && byte <= b'9') || (byte >= b'a' && byte <= b'f'),
                "distribution digest must contain lowercase hexadecimal bytes"
            );
            identity[PREFIX.len() + index] = byte;
            index += 1;
        }
    }
    identity
}

pub(super) fn expected_digest() -> Option<&'static str> {
    if IDENTITY[PREFIX.len()] == 0 {
        return None;
    }
    std::str::from_utf8(&IDENTITY[PREFIX.len()..PREFIX.len() + 64]).ok()
}
