//! Pure operator aid: prints a candidate policy digest; performs no admission or Engine I/O.
use std::io::{self, Read};
use zryna_driver::{PlaygroundDaemonOperatorApproval, playground_daemon_policy_sha256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os().len() != 1 {
        return Err("read one operator record from stdin; no arguments accepted".into());
    }
    let mut bytes = Vec::new();
    io::stdin().take(262_145).read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > 262_144 {
        return Err("finite operator record required".into());
    }
    let record: PlaygroundDaemonOperatorApproval = serde_json::from_slice(&bytes)?;
    println!("{}", playground_daemon_policy_sha256(&record)?);
    Ok(())
}
