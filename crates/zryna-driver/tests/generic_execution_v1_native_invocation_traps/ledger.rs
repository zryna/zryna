//! Actual complete role/physical ledger; no raw rows are interpreted as compiler authority.
use std::collections::BTreeMap;
#[derive(Default)]
pub(super) struct Ledger {
    pub acquired: Vec<[u64; 8]>,
    pub released: Vec<u64>,
    pub calls: Vec<(u64, u64)>,
    pub failures: Vec<(u64, u64, u64)>,
    physical: BTreeMap<u64, (u64, u64)>,
    live: BTreeMap<u64, [u64; 8]>,
    stack: Vec<u64>,
    count_calls: Vec<(u64, u64)>,
}
impl Ledger {
    pub fn parse(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len() % 64, 0, "complete actual records");
        let mut ledger = Self::default();
        for bytes in bytes.chunks_exact(64) {
            let row = std::array::from_fn::<_, 8, _>(|i| {
                u64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().expect("u64"))
            });
            ledger.observe(row);
        }
        assert!(ledger.live.is_empty() && ledger.stack.is_empty());
        assert_eq!(ledger.physical.len(), ledger.acquired.len());
        assert_eq!(ledger.released.len(), ledger.acquired.len());
        let counts = ledger
            .calls
            .iter()
            .copied()
            .filter(|(role, _)| matches!(*role, 10 | 11 | 12 | 14 | 16))
            .collect::<Vec<_>>();
        assert_eq!(ledger.count_calls, counts, "every actual count call has a checked transition");
        ledger
    }
    fn observe(&mut self, row: [u64; 8]) {
        match row[0] {
            1 => self.allocated(row),
            2 => self.acquired(row),
            3 => self.released(row),
            4 => {
                assert!(self.stack.is_empty(), "only outer canonical calls");
                self.stack.push(row[1]);
            }
            5 => {
                assert_eq!(self.stack.pop(), Some(row[1]));
                self.calls.push((row[1], row[2]));
            }
            6 => {
                assert_eq!(self.stack.last(), Some(&row[1]));
                self.failures.push((row[1], row[2], row[3]));
            }
            7 => {
                assert_eq!(self.stack.last(), Some(&row[1]));
                counts(row);
                self.count_calls.push((row[1], row[6]));
            }
            _ => panic!("unknown observer record"),
        }
    }
    fn allocated(&mut self, row: [u64; 8]) {
        assert_eq!(self.stack.len(), 1);
        assert_eq!(row[1], self.physical.len() as u64 + 1);
        assert!(row[2] > 0 && row[3] > 0);
        assert!(self.physical.insert(row[1], (row[2], row[3])).is_none());
    }
    fn acquired(&mut self, row: [u64; 8]) {
        assert_eq!(self.stack.len(), 1);
        assert_eq!(row[1], self.acquired.len() as u64 + 1);
        let block = self.physical.get(&row[1]).expect("one actual canonical block");
        assert_eq!(row[4], block.0);
        assert!(row[3] >= block.0 && row[3] - block.0 < block.1);
        assert!(self.live.values().all(|previous| previous[3] != row[3]));
        assert!(self.live.insert(row[1], row).is_none());
        self.acquired.push(row);
    }
    fn released(&mut self, row: [u64; 8]) {
        assert_eq!(self.stack.len(), 1);
        let before = self.live.remove(&row[1]).expect("one exact live owner, never double free");
        assert_eq!(&row[1..5], &before[1..5]);
        assert_eq!(&row[6..8], &before[6..8]);
        self.released.push(row[1]);
    }
}
fn counts(row: [u64; 8]) {
    let (role, s, w, ss, ww, status) = (row[1], row[2], row[3], row[4], row[5], row[6]);
    assert!(s > 0 && w > 0);
    if status != 0 {
        assert_eq!((role, status, ss, ww), (12, 3, s, w));
        return;
    }
    match role {
        10 => assert_eq!((ss, ww), (s + 1, w)),
        11 | 12 => assert_eq!((ss, ww), (s, w + 1)),
        14 => assert_eq!((ss, ww), (s - 1, w)),
        16 => assert_eq!((ss, ww), (s, w - 1)),
        _ => panic!("unexpected count role"),
    }
}
