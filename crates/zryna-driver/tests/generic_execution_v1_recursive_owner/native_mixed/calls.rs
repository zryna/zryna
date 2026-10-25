//! Exact source outer calls plus all actual nested LIFO/status obligations.
use serde_json::Value;
pub(super) fn check(rows: &[[u64; 8]], expected: &Value, fault: Option<&Value>) {
    let expected = expected.as_array().expect("frozen source calls");
    let mut stack: Vec<(u64, Option<usize>)> = Vec::new();
    let mut next = 0;
    let mut failures = 0;
    let mut injected = 0;
    for row in rows {
        match row[0] {
            1 => {
                assert!(row[1] < 17);
                assert_eq!((row[2], row[3]), (0, 0));
                let index = if stack.is_empty() {
                    let entry = expected.get(next).expect("no extra source calls");
                    assert_eq!((row[1], row[4]), (number(&entry["role"]), number(&entry["used"])));
                    next += 1;
                    Some(next - 1)
                } else {
                    None
                };
                stack.try_reserve(1).expect("bounded actual call stack");
                stack.push((row[1], index));
            }
            4 => {
                let (role, index) = stack.pop().expect("no orphan actual leave");
                assert_eq!((row[1], row[3]), (role, 0));
                let status = index.map_or(0, |index| {
                    assert_eq!(row[4], number(&expected[index]["used"]));
                    number(&expected[index]["status"])
                });
                assert_eq!(row[2], status, "only designated source fault returns1");
                failures += status;
            }
            2 | 3 | 6 => {
                assert_eq!(stack.last().map(|frame| frame.0), Some(row[1]));
                if row[0] == 6 {
                    if matches!(row[1], 14 | 16) {
                        assert!(row[2] <= 1);
                        assert_eq!(row[3], 0);
                    } else {
                        let fault = fault.expect("pristine has no injected fault notice");
                        assert_eq!((row[2], row[3]), (number(&fault["ordinal"]), 1));
                        injected += 1;
                    }
                }
            }
            5 | 7 | 8 => assert!(stack.is_empty()),
            _ => panic!("unknown actual binary event"),
        }
    }
    assert!(stack.is_empty());
    assert_eq!(next, expected.len());
    assert_eq!(failures, u64::from(fault.is_some()));
    assert_eq!(injected, usize::from(fault.is_some()));
}
fn number(value: &Value) -> u64 {
    value.as_u64().expect("frozen unsigned word")
}
