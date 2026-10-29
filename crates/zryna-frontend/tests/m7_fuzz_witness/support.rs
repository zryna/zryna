//! Optional retained originals for the existing deterministic tests; no input generation.
use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::io::Write;

pub struct Witness {
    file: File,
    bytes: usize,
}

pub fn open(header: impl FnOnce() -> Value) -> Option<Witness> {
    let path = std::env::var_os("ZRYNA_M7_FUZZ_WITNESS")?;
    assert!(std::path::Path::new(&path).is_absolute(), "absolute witness path");
    let file =
        OpenOptions::new().write(true).create_new(true).open(path).expect("new campaign witness");
    let mut witness = Some(Witness { file, bytes: 0 });
    record(&mut witness, header);
    witness
}

pub fn record(witness: &mut Option<Witness>, value: impl FnOnce() -> Value) {
    if let Some(witness) = witness {
        let mut bytes = serde_json::to_vec(&value()).expect("campaign record");
        bytes.push(b'\n');
        witness.bytes = witness.bytes.checked_add(bytes.len()).expect("witness size");
        assert!(witness.bytes <= 16 * 1024 * 1024, "campaign witness byte cap");
        witness.file.write_all(&bytes).expect("retain original campaign record");
        witness.file.flush().expect("flush original campaign record");
    }
}
