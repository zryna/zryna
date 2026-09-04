use super::{Errors, Span, Ty, TypeCategory, raw};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScalarOperation {
    Neg,
    Add,
    Sub,
    Mul,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

pub(super) fn require_type(
    expected: Ty,
    actual: Ty,
    at: Span,
    what: &str,
    errors: &mut Errors<'_>,
) -> Option<()> {
    if expected.layout == actual.layout {
        Some(())
    } else {
        errors.at(
            "ZRYNA-M3007",
            at,
            format!("{what} has a different exact aggregate type"),
            "use a value with the exact declared type",
        );
        None
    }
}

pub(super) fn integer(spelling: &str, at: Span, errors: &mut Errors<'_>) -> Option<i32> {
    spelling.parse::<i32>().ok().or_else(|| {
        errors.at(
            "ZRYNA-M3008",
            at,
            format!("integer literal '{spelling}' is outside i32"),
            "use a decimal i32 literal",
        );
        None
    })
}

impl ScalarOperation {
    pub(super) fn validate(
        self,
        integer: Option<Ty>,
        lhs: Ty,
        rhs: Option<Ty>,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        match self {
            Self::Neg => require_type(integer?, lhs, at, "negation operand", errors),
            Self::Add | Self::Sub | Self::Mul => {
                let expected = integer?;
                require_type(expected, lhs, at, "left operand", errors)?;
                require_type(expected, rhs?, at, "right operand", errors)
            }
            Self::Eq | Self::Ne => {
                require_type(lhs, rhs?, at, "comparison", errors)?;
                if !matches!(lhs.category, TypeCategory::Bool | TypeCategory::I32) {
                    errors.at(
                        "ZRYNA-M3008",
                        at,
                        "equality is scalar-only in aggregate M3",
                        "compare bool or i32 projections rather than whole aggregates",
                    );
                    return None;
                }
                Some(())
            }
            Self::Lt | Self::Le | Self::Gt | Self::Ge => {
                let expected = integer?;
                require_type(expected, lhs, at, "relational operand", errors)?;
                require_type(expected, rhs?, at, "relational operand", errors)
            }
        }
    }

    pub(super) fn instruction(
        self,
        lhs: raw::ValueId,
        rhs: Option<raw::ValueId>,
    ) -> raw::InstructionKind {
        if self == Self::Neg {
            assert!(rhs.is_none(), "unary scalar operation has exactly one operand");
            return raw::InstructionKind::I32Neg { operand: lhs };
        }
        let rhs = rhs.expect("binary scalar operation has exactly two operands");
        match self {
            Self::Neg => unreachable!("unary scalar operation returned"),
            Self::Add => raw::InstructionKind::I32Add { lhs, rhs },
            Self::Sub => raw::InstructionKind::I32Sub { lhs, rhs },
            Self::Mul => raw::InstructionKind::I32Mul { lhs, rhs },
            Self::Eq => raw::InstructionKind::Eq { lhs, rhs },
            Self::Ne => raw::InstructionKind::Ne { lhs, rhs },
            Self::Lt => raw::InstructionKind::I32LtS { lhs, rhs },
            Self::Le => raw::InstructionKind::I32LeS { lhs, rhs },
            Self::Gt => raw::InstructionKind::I32GtS { lhs, rhs },
            Self::Ge => raw::InstructionKind::I32GeS { lhs, rhs },
        }
    }
}
