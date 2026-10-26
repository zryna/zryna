use super::*;
use std::panic::{UnwindSafe, catch_unwind, resume_unwind};

fn without_listener_inheritance(fixture: impl FnOnce() + UnwindSafe) {
    // Acquire before creating any fixture listener: earlier children cannot
    // inherit it, and later child creation waits until the fixture finishes.
    #[cfg(target_arch = "x86_64")]
    let guard = zryna_process::snapshot_writer().expect("listener fixture spawn exclusion");
    let result = catch_unwind(fixture);
    // Release normally even when a fixture fails, so its panic cannot poison
    // the shared gate and obscure the original failure in unrelated tests.
    #[cfg(target_arch = "x86_64")]
    drop(guard);
    if let Err(panic) = result {
        resume_unwind(panic);
    }
}

fn assert_original_survives(original: &Identity) {
    let panic = catch_unwind(|| original.assert_gone_probe())
        .expect_err("actual surviving original inode must reject closure");
    let text = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .expect("original closure diagnostic text");
    let expected = format!(
        "original listener really closed: process={} address={} inode={} surviving=",
        original.process, original.address, original.inode,
    );
    assert!(text.starts_with(&expected), "unrelated failure cannot prove inode survival: {text}");
}

#[test]
fn actual_live_original_listener_cannot_receive_gone_credit() {
    without_listener_inheritance(|| {
        let listener = TcpListener::bind("127.0.0.1:0").expect("live original listener");
        let original = Identity::capture(&listener);
        assert_original_survives(&original);
        drop(listener);
        original.assert_gone_probe();
    });
}

#[test]
fn cloned_original_listener_remains_live_after_first_handle_drops() {
    without_listener_inheritance(|| {
        let listener = TcpListener::bind("127.0.0.1:0").expect("cloned original listener");
        let original = Identity::capture(&listener);
        let alias = listener.try_clone().expect("test-only live alias negative");
        drop(listener);
        assert_original_survives(&original);
        drop(alias);
        original.assert_gone_probe();
    });
}

#[test]
fn different_inode_at_same_endpoint_is_not_the_original_listener() {
    without_listener_inheritance(|| {
        let listener = TcpListener::bind("127.0.0.1:0").expect("old original listener");
        let original = Identity::capture(&listener);
        let address = listener.local_addr().expect("old endpoint");
        drop(listener);
        let replacement = TcpListener::bind(address).expect("new listener at exact old endpoint");
        let replacement_identity = Identity::capture(&replacement);
        assert_ne!(original.inode, replacement_identity.inode, "distinct actual socket identities");
        original.assert_gone_probe();
        assert_original_survives(&replacement_identity);
        // The old strict endpoint probe still rejects a real live peer. No waiver or retry.
        assert!(catch_unwind(|| super::super::assert_listener_closed(address)).is_err());
    });
}

#[cfg(target_arch = "x86_64")]
#[test]
fn fixture_panic_preserves_shared_spawn_gate() {
    assert!(catch_unwind(|| without_listener_inheritance(|| panic!("fixture failure"))).is_err());
    zryna_process::spawn(|| Ok(())).expect("shared spawn gate remains usable after fixture panic");
}

fn parse(bytes: &[u8]) -> Snapshot {
    super::parse(bytes, 42)
}

fn table(row: &str) -> Vec<u8> {
    format!("sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode\n{row}\n").into_bytes()
}

#[test]
fn table_schema_and_first_extra_bounds_are_strict() {
    let row = "0: 0100007F:1234 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 42";
    let rows = parse(&table(row));
    assert_eq!(rows.rows, 1);
    assert_eq!(rows.first_match.as_ref().expect("target row").inode, 42);
    assert_eq!(rows.first_match.as_ref().expect("target row").state, 0x0a);
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
    assert_eq!(parse(&table(&repeated)).rows, TABLE_ROWS);
    assert!(catch_unwind(|| parse(&table(&(repeated + "\n" + row)))).is_err());
}

#[test]
fn expired_original_deadline_cannot_read_or_credit_a_table() {
    let expired = Instant::now().checked_sub(PROBE_TIME).expect("past instant");
    assert!(catch_unwind(|| snapshot(expired, 42, "expired-control", Duration::ZERO)).is_err());
}

#[test]
fn complete_nonmatching_rows_and_rows_after_target_still_require_valid_fields() {
    let row = "0: 0100007F:1234 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 42";
    let valid_other = row.replace(" 42", " 99");
    assert_eq!(parse(&table(&format!("{row}\n{valid_other}"))).rows, 2);
    for invalid_other in [
        valid_other.replace("00000000:0000", "0000000Z:0000"),
        valid_other.replace(" 0A ", " 00 "),
        valid_other.replace("00000000:00000000", "00000000:0000000Z"),
        valid_other.replace("00:00000000", "0Z:00000000"),
        valid_other.replace(" 00000000 1000", " Z0000000 1000"),
        valid_other.replace(" 1000 ", " 4294967296 "),
        valid_other.replace(" 0 99", " 18446744073709551616 99"),
        valid_other.replace(" 99", " 18446744073709551616"),
        format!("{valid_other} malformed"),
    ] {
        for joined in [format!("{row}\n{invalid_other}"), format!("{invalid_other}\n{row}")] {
            assert!(catch_unwind(|| parse(&table(&joined))).is_err());
        }
    }
}

#[test]
fn byte_limit_is_inclusive_and_first_extra_byte_is_rejected() {
    let row = "0: 0100007F:1234 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 42";
    let mut exact = table(row);
    exact.pop();
    exact.resize(usize::try_from(TABLE_BYTES).expect("bound fits usize") - 1, b' ');
    exact.push(b'\n');
    assert_eq!(parse(&exact).rows, 1);
    exact.insert(exact.len() - 1, b' ');
    assert!(catch_unwind(|| parse(&exact)).is_err());
}

#[test]
fn numerical_suffixes_and_field_limit_remain_strict() {
    let row = "0: 0100007F:1234 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 42";
    // Signed diagnostics and unsigned hexadecimal diagnostics are both kernel-compatible.
    assert_eq!(parse(&table(&format!("{row} -1 ffffffffffffffff"))).rows, 1);
    let exact = format!("{row}{}", " 0".repeat(22));
    assert_eq!(parse(&table(&exact)).rows, 1);
    for changed in [
        format!("{exact} 0"),
        format!("{row} 100000000000000000000"),
        format!("{row} -9223372036854775809"),
    ] {
        assert!(catch_unwind(|| parse(&table(&changed))).is_err());
    }
}

#[test]
fn original_inode_summary_counts_duplicates_and_every_tcp_state() {
    let row = "0: 0100007F:1234 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 42";
    for state in 1..=0x0d {
        let changed = row.replace(" 0A ", &format!(" {state:02X} "));
        let snapshot = parse(&table(&changed));
        assert_eq!(snapshot.matches, 1, "original inode survives in state {state}");
        assert_eq!(snapshot.first_match.expect("matching row").state, state);
    }
    let duplicate = parse(&table(&format!("{row}\n{row}")));
    assert_eq!(duplicate.rows, 2);
    assert_eq!(duplicate.matches, 2, "capture must reject ambiguous original identity");
    let other = parse(&table(&row.replace(" 42", " 99")));
    assert_eq!(other.rows, 1);
    assert_eq!(other.matches, 0);
    assert!(other.first_match.is_none());
}
