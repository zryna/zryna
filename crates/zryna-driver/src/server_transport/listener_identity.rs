//! Linux test-only ownership proof; numerical endpoint availability is a different claim.

use std::{
    fs::{File, read_link},
    io::Read,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
    os::fd::AsRawFd,
    time::{Duration, Instant},
};

#[path = "listener_identity_holders.rs"]
mod holder_diagnostics;

const TABLE_BYTES: u64 = 1024 * 1024;
const TABLE_ROWS: usize = 4096;
const PROBE_TIME: Duration = Duration::from_millis(100);

#[derive(Debug)]
struct Row {
    local: SocketAddr,
    state: u8,
    inode: u64,
}

// Keep only the original inode's proof; every row is still fully validated.
// Unrelated sockets must not allocate row or field vectors inside the probe budget.
#[derive(Debug, Default)]
struct Snapshot {
    rows: usize,
    matches: usize,
    first_match: Option<Row>,
}

#[derive(Debug)]
pub(super) struct Identity {
    process: u32,
    address: SocketAddr,
    inode: u64,
}
impl Identity {
    pub(super) fn capture(listener: &TcpListener) -> Self {
        let deadline = Instant::now() + PROBE_TIME;
        let address = listener.local_addr().expect("original listener address");
        assert!(address.is_ipv4() && address.ip().is_loopback());
        let inode = socket_inode(listener);
        let snapshot = snapshot(deadline, inode);
        assert_eq!(snapshot.matches, 1, "original listener inode must have one actual TCP row");
        let original = snapshot.first_match.expect("actual original listener TCP row");
        assert_eq!(original.inode, inode, "original listener row identity binding");
        assert_eq!(original.local, address, "original listener endpoint binding");
        assert_eq!(original.state, 0x0a, "original listener must actually be LISTEN");
        assert_eq!(socket_inode(listener), inode, "original listener FD binding unchanged");
        assert_eq!(
            listener.local_addr().expect("original listener address after capture"),
            address
        );
        assert!(Instant::now() < deadline, "original listener capture deadline");
        Self { process: std::process::id(), address, inode }
    }

    pub(super) fn assert_gone(&self) {
        // Synchronize with existing child creation after the owning worker has
        // joined. The strict probe below still rejects every surviving inode;
        // acquiring this gate does not itself establish listener closure.
        #[cfg(target_arch = "x86_64")]
        drop(zryna_process::snapshot_writer().expect("listener closure spawn synchronization"));
        self.assert_gone_probe();
    }

    fn assert_gone_probe(&self) {
        let deadline = Instant::now() + PROBE_TIME;
        assert_eq!(std::process::id(), self.process, "original listener owning process");
        let snapshot = snapshot(deadline, self.inode);
        // All states, not only LISTEN: any surviving original inode is a failure.
        assert!(
            snapshot.matches == 0,
            "original listener really closed: process={} address={} inode={} surviving={snapshot:?} holders={}",
            self.process,
            self.address,
            self.inode,
            holder_diagnostics::collect(self.process, self.inode, deadline)
        );
        assert!(Instant::now() < deadline, "original listener closure deadline");
    }
}

fn socket_inode(listener: &TcpListener) -> u64 {
    let path = format!("/proc/self/fd/{}", listener.as_raw_fd());
    let target = read_link(path).expect("original listener FD socket identity");
    let text = target.to_str().expect("ASCII original socket identity");
    let number = text
        .strip_prefix("socket:[")
        .and_then(|value| value.strip_suffix(']'))
        .expect("original FD must name a socket inode");
    assert!(!number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()));
    let inode = number.parse::<u64>().expect("original socket inode integer");
    assert_ne!(inode, 0, "original socket inode must be nonzero");
    inode
}

fn snapshot(deadline: Instant, inode: u64) -> Snapshot {
    assert!(Instant::now() < deadline, "original TCP table deadline before open");
    let file = File::open("/proc/self/net/tcp").expect("actual original-process TCP table");
    let mut bytes = Vec::new();
    file.take(TABLE_BYTES + 1).read_to_end(&mut bytes).expect("bounded actual TCP table read");
    assert!(Instant::now() < deadline, "original TCP table deadline after read");
    let rows = parse(&bytes, inode);
    assert!(Instant::now() < deadline, "original TCP table deadline after parse");
    rows
}

fn endpoint(text: &str) -> SocketAddr {
    let (host, port) = text.split_once(':').expect("TCP endpoint separator");
    assert_eq!(host.len(), 8, "IPv4 TCP endpoint width");
    assert_eq!(port.len(), 4, "TCP port width");
    assert!(host.bytes().chain(port.bytes()).all(|byte| byte.is_ascii_hexdigit()));
    let host = u32::from_str_radix(host, 16).expect("TCP IPv4 integer");
    let port = u16::from_str_radix(port, 16).expect("TCP port integer");
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(host.to_ne_bytes()), port))
}

fn hex_pair(text: &str, left: usize, right: usize) {
    let (first, second) = text.split_once(':').expect("TCP combined field separator");
    assert_eq!(first.len(), left, "TCP combined first field width");
    assert_eq!(second.len(), right, "TCP combined second field width");
    assert!(first.bytes().chain(second.bytes()).all(|byte| byte.is_ascii_hexdigit()));
}

fn parse(bytes: &[u8], original_inode: u64) -> Snapshot {
    assert!(
        bytes.len() <= usize::try_from(TABLE_BYTES).expect("table bound fits usize"),
        "TCP table first-extra byte"
    );
    assert!(bytes.is_ascii(), "actual TCP table ASCII");
    let text = std::str::from_utf8(bytes).expect("actual TCP table text");
    assert!(text.ends_with('\n'), "complete TCP table final row");
    let mut lines = text.lines();
    let header = lines.next().expect("actual TCP table header");
    assert!(
        header.split_whitespace().eq([
            "sl",
            "local_address",
            "rem_address",
            "st",
            "tx_queue",
            "rx_queue",
            "tr",
            "tm->when",
            "retrnsmt",
            "uid",
            "timeout",
            "inode"
        ]),
        "actual TCP table header fields"
    );
    let mut snapshot = Snapshot::default();
    for line in lines {
        assert!(snapshot.rows < TABLE_ROWS, "TCP table first-extra row");
        // This iterator borrows every field and preserves the original whitespace
        // grammar without allocating a per-row collection for unrelated inodes.
        let mut fields = line.split_whitespace();
        let mut field = || fields.next().expect("TCP table row field count");
        field()
            .strip_suffix(':')
            .expect("TCP row ordinal separator")
            .parse::<usize>()
            .expect("TCP row ordinal integer");
        let local = endpoint(field());
        let _remote = endpoint(field());
        let state = field();
        assert_eq!(state.len(), 2, "TCP state width");
        let state = u8::from_str_radix(state, 16).expect("TCP state integer");
        assert!((1..=0x0d).contains(&state), "known Linux TCP state");
        hex_pair(field(), 8, 8);
        hex_pair(field(), 2, 8);
        u64::from_str_radix(field(), 16).expect("TCP retransmission field");
        field().parse::<u32>().expect("TCP uid integer");
        field().parse::<u64>().expect("TCP timeout integer");
        let inode = field().parse::<u64>().expect("TCP inode integer");
        // Kernel versions append numerical diagnostics; malformed suffixes still fail closed.
        for (index, field) in fields.enumerate() {
            assert!(index < 22, "TCP table row field count");
            assert!(
                field.parse::<i64>().is_ok() || u64::from_str_radix(field, 16).is_ok(),
                "TCP numerical diagnostic field"
            );
        }
        snapshot.rows += 1;
        if inode == original_inode {
            snapshot.matches += 1;
            if snapshot.first_match.is_none() {
                snapshot.first_match = Some(Row { local, state, inode });
            }
        }
    }
    snapshot
}

#[cfg(test)]
#[path = "listener_identity_tests.rs"]
mod tests;
