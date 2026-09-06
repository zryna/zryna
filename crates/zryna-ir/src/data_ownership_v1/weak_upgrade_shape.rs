use super::{VerifiedEdge, VerifiedTerminator, raw, verified_edge};
use zryna_layout::{TypeCategory, TypeId, VerifiedLayouts};

impl VerifiedTerminator<'_> {
    /// Returns the verified success and expired edges of an indivisible Weak upgrade.
    #[must_use]
    pub fn weak_upgrade_edges(self) -> Option<(VerifiedEdge, VerifiedEdge)> {
        let raw::Terminator::WeakUpgradeBranch { success, expired, .. } = &self.terminator.kind
        else {
            return None;
        };
        let owner = self.function.id();
        Some((verified_edge(owner, success), verified_edge(owner, expired)))
    }
}

/// Reusable, target-neutral type schema for planning a weak-upgrade edge.
///
/// The success schema starts with [`Self::success_parameter_type`], followed by
/// the ordinary explicit argument types. The expired schema contains only its
/// ordinary argument types. This shape does not issue an owner, choose a runtime
/// outcome, prove liveness or cleanup, or identify a program or invocation.
/// Copying or deriving it again grants no additional authority.
///
/// Producers may use this schema before a complete CFG exists. Every completed
/// raw program must still pass [`crate::data_ownership_v1::verify`]; only that
/// full boundary seals ownership, dominance, edge signatures and cleanup.
/// This is not a refcount transition, execution receipt or verified program.
///
/// Callers cannot construct a shape from independently selected type fields:
///
/// ```compile_fail
/// use zryna_ir::data_ownership_v1::WeakUpgradeShape;
/// fn forge(shape: WeakUpgradeShape) {
///     let _ = WeakUpgradeShape {
///         weak: shape.weak_type(),
///         referent: shape.referent_type(),
///         success_parameter: shape.success_parameter_type(),
///     };
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeakUpgradeShape {
    weak: TypeId,
    referent: TypeId,
    success_parameter: TypeId,
}

impl WeakUpgradeShape {
    /// Derives a planning shape from an exact branded Weak type and sealed layout.
    ///
    /// Returns `None` for a different type universe, a non-Weak type, or absence
    /// of a Shared type with the exact same referent in this layout snapshot.
    /// The bounded lookup uses only sealed type records, not raw numeric IDs.
    #[must_use]
    pub fn derive(layouts: &VerifiedLayouts, weak: TypeId) -> Option<Self> {
        let record = layouts.type_by_id(weak)?;
        if record.category() != TypeCategory::Weak {
            return None;
        }
        let referent = record.referenced_type()?;
        let success_parameter = layouts
            .types()
            .find(|candidate| {
                candidate.category() == TypeCategory::Shared
                    && candidate.referenced_type() == Some(referent)
            })?
            .id();
        Some(Self { weak, referent, success_parameter })
    }

    /// Returns the exact Weak operand type, not an operand value or live owner.
    #[must_use]
    pub const fn weak_type(self) -> TypeId {
        self.weak
    }

    /// Returns the sealed payload type shared by the Weak and Shared schemas.
    #[must_use]
    pub const fn referent_type(self) -> TypeId {
        self.referent
    }

    /// Returns the first success parameter's type, without issuing that owner.
    #[must_use]
    pub const fn success_parameter_type(self) -> TypeId {
        self.success_parameter
    }
}
