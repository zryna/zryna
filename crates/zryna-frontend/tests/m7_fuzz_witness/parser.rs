//! Parser-only per-execution retained outcome context.
use super::witness::{Witness, record};
use serde_json::Value;

pub struct ParserTrace<'a> {
    retained: &'a mut Option<Witness>,
    context: (usize, u32, usize),
}

pub fn parser(retained: &mut Option<Witness>, context: (usize, u32, usize)) -> ParserTrace<'_> {
    ParserTrace { retained, context }
}

impl ParserTrace<'_> {
    pub fn raw<T: serde::Serialize>(&mut self, raw: &T) {
        let (case, version, execution) = self.context;
        record(self.retained, || {
            serde_json::json!({"event":"output","case":case,
            "version":version,"execution":execution,"outcome":{"result":raw}})
        });
    }

    pub fn rejection(&mut self, value: Value) -> Value {
        let (case, version, execution) = self.context;
        record(self.retained, || {
            serde_json::json!({"event":"output","case":case,
            "version":version,"execution":execution,"outcome":&value})
        });
        value
    }
}
