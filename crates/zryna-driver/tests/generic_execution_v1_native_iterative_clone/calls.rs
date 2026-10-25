//! Source outer roles and pending destination ownership, including every actual nested status.
fn expected(depth: u32, fault: bool) -> Vec<(u64, u64, u64)> {
    let d = u64::from(depth);
    let mut roles = Vec::new();
    roles
        .try_reserve_exact(usize::try_from(4 * d + 4).expect("fixed depth role count"))
        .expect("fallible bounded source roles");
    roles.push((3, 0, 0));
    roles.extend(std::iter::repeat_n(
        (7, 0, 0),
        usize::try_from(depth).expect("fixed depth fits host"),
    ));
    roles.extend((0..d).map(|used| (7, used, 0)));
    roles.push((4, d, u64::from(fault)));
    if !fault {
        roles.push((6, d, 0));
    }
    roles.extend((0..d).rev().map(|used| (9, used, 0)));
    roles.push((6, d, 0));
    roles.extend((0..d).rev().map(|used| (9, used, 0)));
    roles
}
pub(super) fn check(rows: &[[u64; 8]], depth: u32, fault: bool) {
    let expected = expected(depth, fault);
    let mut stack: Vec<(u64, Option<usize>)> = Vec::new();
    let mut next = 0;
    let mut failures = 0;
    let mut fault_markers = 0;
    for row in rows {
        match row[0] {
            1 => {
                assert!(row[1] < 17);
                assert_eq!((row[2], row[3]), (0, 0));
                let outer = if stack.is_empty() {
                    let (role, used, _) = expected.get(next).expect("no extra source call");
                    assert_eq!((row[1], row[4]), (*role, *used), "outer role/used at entry{next}");
                    next += 1;
                    Some(next - 1)
                } else {
                    None
                };
                stack.try_reserve(1).expect("fallible bounded actual call stack");
                stack.push((row[1], outer));
            }
            4 => {
                let (role, outer) = stack.pop().expect("actual leave must have an enter");
                assert_eq!((row[1], row[3]), (role, 0), "actual calls must leave in LIFO order");
                let status = outer.map_or(0, |index| {
                    let (_, used, status) = expected[index];
                    assert_eq!(row[4], used, "exact pending owners at outer leave{index}");
                    status
                });
                assert_eq!(row[2], status, "exact canonical status");
                failures += status;
            }
            2 | 3 | 6 => {
                assert_eq!(stack.last().map(|frame| frame.0), Some(row[1]));
                if row[0] == 6 {
                    assert!(fault);
                    assert_eq!((row[1], row[2], row[3], row[4]), (4, 1, 1, u64::from(depth)));
                    fault_markers += 1;
                }
            }
            5 | 7 | 8 => assert!(stack.is_empty(), "root boundary has no unfinished call"),
            _ => panic!("unknown actual event"),
        }
    }
    assert!(stack.is_empty());
    assert_eq!(next, expected.len(), "all source canonical calls execute in exact order");
    assert_eq!(failures, u64::from(fault));
    assert_eq!(fault_markers, usize::from(fault));
}
