//! Linux test-only ownership proof; numerical endpoint availability is a different claim.

use std::{
    fs::{File, read_link},
    io::Read,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
    os::fd::AsRawFd,
    time::{Duration, Instant},
};

const TABLE_BYTES: u64 = 1024 * 1024;
const TABLE_ROWS: usize = 4096;
const PROBE_TIME: Duration = Duration::from_millis(100);

#[derive(Debug)]
struct Row {
    local: SocketAddr,
    state: u8,
    inode: u64,
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
        let rows = snapshot(deadline);
        let matching: Vec<_> = rows.iter().filter(|row| row.inode == inode).collect();
        assert_eq!(matching.len(), 1, "original listener inode must have one actual TCP row");
        assert_eq!(matching[0].local, address, "original listener endpoint binding");
        assert_eq!(matching[0].state, 0x0a, "original listener must actually be LISTEN");
        assert_eq!(socket_inode(listener), inode, "original listener FD binding unchanged");
        assert_eq!(
            listener.local_addr().expect("original listener address after capture"),
            address
        );
        assert!(Instant::now() < deadline, "original listener capture deadline");
        Self { process: std::process::id(), address, inode }
    }

    pub(super) fn assert_gone(&self) {
        // The owning worker has joined, but a concurrent fork can still hold its
        // inherited listener until exec applies CLOEXEC. Drain existing spawns
        // before the bounded namespace-wide probe; later spawns cannot inherit
        // the already-closed parent FD. Release before any assertion can panic.
        #[cfg(target_arch = "x86_64")]
        drop(zryna_process::snapshot_writer().expect("listener closure spawn synchronization"));
        let deadline = Instant::now() + PROBE_TIME;
        assert_eq!(std::process::id(), self.process, "original listener owning process");
        let rows = snapshot(deadline);
        // All states, not only LISTEN: any surviving original inode is a failure.
        let matching: Vec<_> = rows.iter().filter(|row| row.inode == self.inode).collect();
        assert!(
            matching.is_empty(),
            "original listener really closed: process={} address={} inode={} surviving={matching:?}",
            self.process,
            self.address,
            self.inode
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

fn snapshot(deadline: Instant) -> Vec<Row> {
    assert!(Instant::now() < deadline, "original TCP table deadline before open");
    let file = File::open("/proc/self/net/tcp").expect("actual original-process TCP table");
    let mut bytes = Vec::new();
    file.take(TABLE_BYTES + 1).read_to_end(&mut bytes).expect("bounded actual TCP table read");
    assert!(Instant::now() < deadline, "original TCP table deadline after read");
    let rows = parse(&bytes);
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

fn parse(bytes: &[u8]) -> Vec<Row> {
    assert!(
        bytes.len() <= usize::try_from(TABLE_BYTES).expect("table bound fits usize"),
        "TCP table first-extra byte"
    );
    assert!(bytes.is_ascii(), "actual TCP table ASCII");
    let text = std::str::from_utf8(bytes).expect("actual TCP table text");
    assert!(text.ends_with('\n'), "complete TCP table final row");
    let mut lines = text.lines();
    let header: Vec<_> =
        lines.next().expect("actual TCP table header").split_whitespace().collect();
    assert_eq!(
        header,
        [
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
        ]
    );
    let mut rows = Vec::new();
    for line in lines {
        assert!(rows.len() < TABLE_ROWS, "TCP table first-extra row");
        let fields: Vec<_> = line.split_whitespace().collect();
        assert!((10..=32).contains(&fields.len()), "TCP table row field count");
        fields[0]
            .strip_suffix(':')
            .expect("TCP row ordinal separator")
            .parse::<usize>()
            .expect("TCP row ordinal integer");
        let local = endpoint(fields[1]);
        let _remote = endpoint(fields[2]);
        assert_eq!(fields[3].len(), 2, "TCP state width");
        let state = u8::from_str_radix(fields[3], 16).expect("TCP state integer");
        assert!((1..=0x0d).contains(&state), "known Linux TCP state");
        hex_pair(fields[4], 8, 8);
        hex_pair(fields[5], 2, 8);
        u64::from_str_radix(fields[6], 16).expect("TCP retransmission field");
        fields[7].parse::<u32>().expect("TCP uid integer");
        fields[8].parse::<u64>().expect("TCP timeout integer");
        let inode = fields[9].parse::<u64>().expect("TCP inode integer");
        // Kernel versions append numerical diagnostics; malformed suffixes still fail closed.
        for field in &fields[10..] {
            assert!(
                field.parse::<i64>().is_ok() || u64::from_str_radix(field, 16).is_ok(),
                "TCP numerical diagnostic field"
            );
        }
        rows.push(Row { local, state, inode });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::catch_unwind;

    #[test]
    fn actual_live_original_listener_cannot_receive_gone_credit() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("live original listener");
        let original = Identity::capture(&listener);
        assert!(catch_unwind(|| original.assert_gone()).is_err());
        drop(listener);
        original.assert_gone();
    }

    #[test]
    fn cloned_original_listener_remains_live_after_first_handle_drops() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("cloned original listener");
        let original = Identity::capture(&listener);
        let alias = listener.try_clone().expect("test-only live alias negative");
        drop(listener);
        assert!(catch_unwind(|| original.assert_gone()).is_err());
        drop(alias);
        original.assert_gone();
    }

    #[test]
    fn different_inode_at_same_endpoint_is_not_the_original_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("old original listener");
        let original = Identity::capture(&listener);
        let address = listener.local_addr().expect("old endpoint");
        drop(listener);
        let replacement = TcpListener::bind(address).expect("new listener at exact old endpoint");
        let replacement_identity = Identity::capture(&replacement);
        assert_ne!(original.inode, replacement_identity.inode, "distinct actual socket identities");
        original.assert_gone();
        assert!(catch_unwind(|| replacement_identity.assert_gone()).is_err());
        // The old strict endpoint probe still rejects a real live peer. No waiver or retry.
        assert!(catch_unwind(|| super::super::assert_listener_closed(address)).is_err());
    }

    fn table(row: &str) -> Vec<u8> {
        format!("sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode\n{row}\n").into_bytes()
    }

    #[test]
    fn table_schema_and_first_extra_bounds_are_strict() {
        let row =
            "0: 0100007F:1234 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 42";
        let rows = parse(&table(row));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].inode, 42);
        assert_eq!(rows[0].state, 0x0a);
        for changed in [
            table(""),
            table(&row.replace(" 42", " bad")),
            table(&row.replace("0100007F", "not-hex!")),
            table(&row.replace(" 0A ", " wrong ")),
            table(row).into_iter().rev().collect(),
        ] {
            assert!(catch_unwind(|| parse(&changed)).is_err());
        }
        let mut truncated = table(row);
        truncated.pop();
        assert!(catch_unwind(|| parse(&truncated)).is_err());
        let extra = vec![b'x'; usize::try_from(TABLE_BYTES + 1).expect("bound fits usize")];
        assert!(catch_unwind(|| parse(&extra)).is_err());
        let repeated = vec![row; TABLE_ROWS].join("\n");
        assert_eq!(parse(&table(&repeated)).len(), TABLE_ROWS);
        assert!(catch_unwind(|| parse(&table(&(repeated + "\n" + row)))).is_err());
    }

    #[test]
    fn expired_original_deadline_cannot_read_or_credit_a_table() {
        let expired = Instant::now().checked_sub(PROBE_TIME).expect("past instant");
        assert!(catch_unwind(|| snapshot(expired)).is_err());
    }
}
