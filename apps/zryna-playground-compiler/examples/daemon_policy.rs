//! Pure operator aid: prints a candidate policy digest; performs no admission or Engine I/O.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::io::{self, Read};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zryna_driver::{PlaygroundDaemonOperatorApproval, playground_daemon_policy_sha256};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
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

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("daemon policy calculation requires Linux x86_64".into())
}
