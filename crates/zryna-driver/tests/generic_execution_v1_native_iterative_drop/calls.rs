//! Source-derived outer canonical roles; nested factoring still has exact balanced status checks.
fn expected(case: &str, depth: u32, fault: bool) -> Vec<u64> {
    let d = usize::try_from(depth).expect("source depth fits host");
    let capacity = d
        .checked_mul(2)
        .and_then(|n| n.checked_add(14))
        .expect("bounded source-derived role count");
    let mut roles = Vec::new();
    roles.try_reserve_exact(capacity).expect("fallible source-role storage");
    roles.push(3);
    roles.extend(std::iter::repeat_n(7, d));
    match case {
        "drop-only" => {
            assert!(!fault);
            roles.push(6);
        }
        "controlled-cleanup" => {
            roles.extend([3, 4, 6, 6]);
            if !fault {
                roles.push(6);
            }
        }
        "shared-weak-recursive" => {
            assert!(!fault);
            roles.extend([0, 10, 11, 12, 16, 16, 14, 14, 6]);
        }
        _ => panic!("unknown unchanged source case"),
    }
    roles.extend(std::iter::repeat_n(9, d));
    if case == "shared-weak-recursive" {
        roles.push(15);
    }
    roles
}
pub(super) fn check(rows: &[[u64; 8]], case: &str, depth: u32, fault: bool) {
    let mut stack: Vec<(u64, bool)> = Vec::new();
    let mut outer = Vec::new();
    let mut failed = 0;
    for row in rows {
        match row[0] {
            1 => {
                assert!(row[1] < 17, "actual canonical role must be declared");
                assert_eq!((row[2], row[3]), (0, 0), "canonical entry framing");
                let source_call = stack.is_empty();
                if source_call {
                    outer.try_reserve(1).expect("fallible bounded outer-role storage");
                    outer.push(row[1]);
                }
                stack.try_reserve(1).expect("fallible bounded canonical stack");
                stack.push((row[1], source_call));
            }
            4 => {
                let (role, source_call) = stack.pop().expect("leave needs actual prior enter");
                assert_eq!(row[1], role, "canonical enter/leave must be LIFO by role");
                assert_eq!(row[3], 0, "canonical leave framing");
                let status = u64::from(fault && source_call && role == 4);
                assert_eq!(row[2], status, "{case}/depth{depth}/role{role}: exact status");
                failed += status;
            }
            2 | 3 | 6 => {
                assert_eq!(
                    stack.last().map(|frame| frame.0),
                    Some(row[1]),
                    "allocator/control observation must belong to the current canonical frame"
                );
            }
            5 | 7 | 8 => assert!(stack.is_empty(), "root boundary has no outstanding calls"),
            _ => panic!("unknown actual observer event"),
        }
        if matches!(row[0], 1 | 4) && matches!(row[1], 0 | 3 | 4 | 7 | 10 | 11 | 12) {
            assert_eq!(row[4], 0, "construction/marker Clone has no pending Drop frame");
        }
    }
    assert!(stack.is_empty(), "every actual canonical enter must leave");
    assert_eq!(failed, u64::from(fault), "only one designated controlled Clone may fail");
    assert_eq!(outer, expected(case, depth, fault), "{case}/depth{depth}: source canonical order");
}
