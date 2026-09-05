use zryna_ir::data_ownership_v1 as ir;

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1) enum OwnedCleanupReservationContext {
    String,
    Vec,
}

impl OwnedCleanupReservationContext {
    pub(super) fn reservation(self) -> (&'static str, &'static str) {
        match self {
            Self::String => (
                "reserved String cleanup exceeds the per-function M3 limits",
                "reduce simultaneously live Strings or fallible String operations",
            ),
            Self::Vec => (
                "reserved Vec cleanup exceeds the per-function M3 limits",
                "reduce simultaneously live owned values or fallible Vec operations",
            ),
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1) enum OwnedCleanupPlanContext {
    String,
    Vec,
    Aggregate,
}

impl OwnedCleanupPlanContext {
    pub(super) fn plan_guidance(self) -> &'static str {
        match self {
            Self::String => "reduce fallible private String operations",
            Self::Vec => "reduce fallible private Vec operations",
            Self::Aggregate => "reduce fallible String leaves in private aggregate construction",
        }
    }

    pub(super) fn action_guidance(self) -> &'static str {
        match self {
            Self::String => {
                "reduce simultaneously live Strings or fallible private String operations"
            }
            Self::Vec => {
                "reduce simultaneously live owned values or fallible private Vec operations"
            }
            Self::Aggregate => "reduce simultaneously live owned aggregates and String leaves",
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1) enum OwnedCleanupActionContext {
    StringBranchLocal,
    StringTerminalArm,
    VecBranchLocal,
    VecTerminalArm,
}

impl OwnedCleanupActionContext {
    pub(super) fn diagnostic(self) -> (String, &'static str) {
        match self {
            Self::StringBranchLocal => (
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce branch-local owned Strings or fallible String operations",
            ),
            Self::StringTerminalArm => (
                "terminal String arm cleanup exceeds the per-function M3 limit".to_owned(),
                "reduce owned temporaries in the returning branch expression",
            ),
            Self::VecBranchLocal => (
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce branch-local owned values or fallible Vec operations",
            ),
            Self::VecTerminalArm => (
                "terminal Vec arm cleanup exceeds the per-function M3 limit".to_owned(),
                "reduce owned temporaries in the returning branch expression",
            ),
        }
    }
}
