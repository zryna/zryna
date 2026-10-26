//! Exact scalar roles; primitive lookups and value publication retain existing checked owners.
use super::{Body, Expr, Failure, UntrustedSpan, Value, raw, reject};
use raw::ScalarBinaryKind as B;

pub(super) fn binary_kind(source: &Expr) -> Option<B> {
    Some(match source {
        Expr::Addition { .. } => B::Add,
        Expr::Subtraction { .. } => B::Sub,
        Expr::Multiplication { .. } => B::Mul,
        Expr::Equal { .. } => B::Eq,
        Expr::NotEqual { .. } => B::Ne,
        Expr::LessThan { .. } => B::LtS,
        Expr::LessEqual { .. } => B::LeS,
        Expr::GreaterThan { .. } => B::GtS,
        Expr::GreaterEqual { .. } => B::GeS,
        _ => return None,
    })
}
impl Body<'_, '_> {
    pub(super) fn scalar_negation(
        &mut self,
        operand: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let value = self.expression(operand, depth + 1)?;
        let ty = self.primitive(1)?;
        if value.ty != ty {
            return Err(reject("source negation requires exact i32"));
        }
        self.emit(ty, raw::Operation::I32Neg { value: value.id }, span)
    }
    pub(super) fn scalar_binary(
        &mut self,
        lhs: u32,
        rhs: u32,
        kind: B,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        // Both source operands are evaluated exactly once in their original left-to-right order.
        let left = self.expression(lhs, depth + 1)?;
        let right = self.expression(rhs, depth + 1)?;
        let integer = self.primitive(1)?;
        let ty = if matches!(kind, B::Eq | B::Ne) {
            let boolean = self.primitive(0)?;
            if left.ty != right.ty || !(left.ty == integer || left.ty == boolean) {
                return Err(reject("source equality requires identical exact i32 or bool"));
            }
            boolean
        } else {
            if left.ty != integer || right.ty != integer {
                return Err(reject("source arithmetic/comparison requires exact i32"));
            }
            if matches!(kind, B::LtS | B::LeS | B::GtS | B::GeS) {
                self.primitive(0)?
            } else {
                integer
            }
        };
        self.emit(ty, raw::Operation::ScalarBinary { kind, left: left.id, right: right.id }, span)
    }
}
