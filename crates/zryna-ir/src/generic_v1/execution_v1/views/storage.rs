use super::{VerifiedExecution, VerifiedOperation, raw};

/// Exact ordinary value; numeric IDs remain bound to one execution/function.
#[derive(Clone, Copy)]
pub struct VerifiedValue<'p, 'a> {
    pub(super) owner: &'p VerifiedExecution<'a>,
    pub(super) function: usize,
    pub(super) id: u32,
}
/// Exact static root/projection whose type and overlap ancestry were independently checked.
#[derive(Clone, Copy)]
pub struct VerifiedPlace<'p, 'a> {
    pub(super) owner: &'p VerifiedExecution<'a>,
    pub(super) function: usize,
    pub(super) id: u32,
}
/// Exact loan metadata; operation/state views establish where it is live.
#[derive(Clone, Copy)]
pub struct VerifiedLoan<'p, 'a> {
    pub(super) owner: &'p VerifiedExecution<'a>,
    pub(super) function: usize,
    pub(super) id: u32,
}
/// Exact checked loan source role.
#[derive(Clone, Copy)]
pub enum VerifiedLoanOrigin<'p, 'a> {
    /// Caller-owned formal referent, without a fabricated local root.
    Formal(u32),
    /// Complete exact local place.
    Place(VerifiedPlace<'p, 'a>),
    /// Inherited compiler-created parent binding.
    Parent(VerifiedLoan<'p, 'a>),
    /// Exact checked static projection from its parent loan.
    StaticProjection {
        /// Compiler-created parent loan.
        parent: VerifiedLoan<'p, 'a>,
        /// Exact ordinal or constant index.
        projection: raw::StaticProjection,
    },
}
/// Exact compared cleanup plan from one operation/exit.
#[derive(Clone, Copy)]
pub struct VerifiedCleanupPlan<'p, 'a> {
    pub(super) owner: &'p VerifiedExecution<'a>,
    pub(super) function: usize,
    pub(super) id: u32,
}
/// Checked cleanup action; prefix contains a site token rather than a runtime count.
#[derive(Clone, Copy)]
pub enum VerifiedCleanupAction<'p, 'a> {
    /// End an exact live loan.
    EndLoan(VerifiedLoan<'p, 'a>),
    /// Release exact initialized place state.
    DropPlace(VerifiedPlace<'p, 'a>),
    /// Unwind operation-local actual runtime progress first.
    DropInitializedPrefix {
        /// Exact destination storage.
        destination: VerifiedPlace<'p, 'a>,
        /// Exact successful-preparation protocol.
        preparation: VerifiedOperation<'p, 'a>,
    },
}

impl VerifiedValue<'_, '_> {
    /// Function-local dense identity, meaningful only with this view's owner.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.id
    }
    /// Owning function inventory identity.
    #[must_use]
    pub const fn function(self) -> usize {
        self.function
    }
    /// Complete verified value type.
    #[must_use]
    pub fn ty(self) -> raw::ValueType {
        self.owner.typed.functions[self.function].values[self.id as usize]
    }
}
impl VerifiedPlace<'_, '_> {
    /// Dense place identity in this function.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.id
    }
    /// Owning function identity.
    #[must_use]
    pub const fn function(self) -> usize {
        self.function
    }
    /// Exact retained referent type.
    #[must_use]
    pub fn ty(self) -> u32 {
        self.owner.typed.functions[self.function].places.rows[self.id as usize].ty
    }
    /// Exact complete storage root.
    #[must_use]
    pub fn root(self) -> Self {
        Self {
            id: self.owner.typed.functions[self.function].places.rows[self.id as usize].root,
            ..self
        }
    }
    /// Exact static projection parent; never a dynamic index masquerading as a place.
    #[must_use]
    pub fn parent(self) -> Option<Self> {
        self.owner.typed.functions[self.function].places.rows[self.id as usize]
            .parent
            .map(|id| Self { id, ..self })
    }
    /// Source/type-checked static projection metadata in this owner only.
    #[must_use]
    pub fn kind(self) -> raw::PlaceKind {
        self.owner.program.functions[self.function].places[self.id as usize].kind
    }
}
impl<'p, 'a> VerifiedLoan<'p, 'a> {
    /// Owning function identity retained by this verified loan.
    #[must_use]
    pub const fn function(self) -> usize {
        self.function
    }
    /// Dense loan identity in this function.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.id
    }
    /// Exact stored referent type, never a stored Borrow key.
    #[must_use]
    pub fn referent(self) -> u32 {
        self.owner.program.functions[self.function].loans[self.id as usize].referent
    }
    /// Exact shared/exclusive access.
    #[must_use]
    pub fn access(self) -> raw::Access {
        self.owner.program.functions[self.function].loans[self.id as usize].access
    }
    /// Exact authenticated lexical/compiler-created source region.
    #[must_use]
    pub fn region(self) -> &'p raw::Region {
        &self.owner.program.functions[self.function].loans[self.id as usize].region
    }
    /// Retained local root, absent for a caller-owned formal referent.
    #[must_use]
    pub fn local_root(self) -> Option<VerifiedPlace<'p, 'a>> {
        self.owner.typed.functions[self.function].places.loans[self.id as usize]
            .root
            .map(|id| VerifiedPlace { owner: self.owner, function: self.function, id })
    }
    /// Checked origin without allowing caller installation of rows.
    #[must_use]
    pub fn origin(self) -> Option<VerifiedLoanOrigin<'p, 'a>> {
        match self.owner.program.functions[self.function].loans[self.id as usize].origin {
            raw::LoanOrigin::Formal(id) => Some(VerifiedLoanOrigin::Formal(id)),
            raw::LoanOrigin::Place(id) => Some(VerifiedLoanOrigin::Place(VerifiedPlace {
                owner: self.owner,
                function: self.function,
                id,
            })),
            raw::LoanOrigin::InternalReborrow(id) => {
                Some(VerifiedLoanOrigin::Parent(Self { id, ..self }))
            }
            raw::LoanOrigin::StaticProjection { parent, projection } => {
                Some(VerifiedLoanOrigin::StaticProjection {
                    parent: Self { id: parent, ..self },
                    projection,
                })
            }
            raw::LoanOrigin::Indexed { .. } => None,
        }
    }
}
impl<'p, 'a> VerifiedCleanupPlan<'p, 'a> {
    /// Exact dense cleanup identity in this function.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.id
    }
    /// Retained exact actions, without a fresh state replay or comparison.
    #[must_use]
    pub fn actions(self) -> impl ExactSizeIterator<Item = VerifiedCleanupAction<'p, 'a>> {
        self.owner.program.functions[self.function].cleanup_plans[self.id as usize]
            .actions
            .iter()
            .map(move |action| match action {
                raw::CleanupAction::EndLoan(id) => VerifiedCleanupAction::EndLoan(VerifiedLoan {
                    owner: self.owner,
                    function: self.function,
                    id: *id,
                }),
                raw::CleanupAction::DropPlace(id) => {
                    VerifiedCleanupAction::DropPlace(VerifiedPlace {
                        owner: self.owner,
                        function: self.function,
                        id: *id,
                    })
                }
                raw::CleanupAction::DropInitializedPrefix { destination, preparation_site } => {
                    VerifiedCleanupAction::DropInitializedPrefix {
                        destination: VerifiedPlace {
                            owner: self.owner,
                            function: self.function,
                            id: *destination,
                        },
                        preparation: VerifiedOperation {
                            owner: self.owner,
                            index: self.owner.typed.functions[self.function].sites
                                [preparation_site.block as usize]
                                [preparation_site.instruction as usize],
                        },
                    }
                }
            })
    }
}
