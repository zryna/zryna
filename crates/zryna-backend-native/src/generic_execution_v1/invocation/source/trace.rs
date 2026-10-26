//! Test-only handles to actual emitted tails; no synthetic source or effect authority.
use super::{Cases, Inst, Value};
use cranelift_codegen::ir::Function;
use zryna_diagnostics::Diagnostic;
#[derive(Clone, Copy)]
pub(in crate::generic_execution_v1) struct Tail {
    previous: Option<Inst>,
    end: Inst,
    status: Value,
    cases: Cases,
}
thread_local! {static ACTIVE:std::cell::RefCell<Option<Vec<Tail>>>=const{std::cell::RefCell::new(None)};}
pub(super) fn record(previous: Option<Inst>, end: Inst, status: Value, cases: Cases) {
    ACTIVE.with_borrow_mut(|rows| {
        if let Some(rows) = rows {
            rows.push(Tail { previous, end, status, cases });
        }
    });
}
pub(in crate::generic_execution_v1) fn start() {
    ACTIVE.with_borrow_mut(|rows| *rows = Some(Vec::new()));
}
pub(in crate::generic_execution_v1) fn finish() -> Vec<Tail> {
    ACTIVE.with_borrow_mut(|rows| rows.take().expect("active actual graph trace"))
}
impl Tail {
    pub fn check(self, f: &Function) -> Result<(), Diagnostic> {
        super::check(f, self.previous, self.end, self.status, self.cases)
    }
    pub fn codes(self) -> impl Iterator<Item = u32> {
        self.cases.entries().map(|(_, source)| source.code())
    }
    pub fn instructions(self, f: &Function) -> Vec<Inst> {
        let block = f.layout.inst_block(self.end).expect("actual tail block");
        let mut next =
            self.previous.map_or_else(|| f.layout.first_inst(block), |i| f.layout.next_inst(i));
        let mut result = Vec::new();
        while let Some(inst) = next {
            result.push(inst);
            if inst == self.end {
                break;
            }
            next = f.layout.next_inst(inst);
        }
        result
    }
}
