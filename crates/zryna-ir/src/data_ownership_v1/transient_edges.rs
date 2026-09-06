//! Acyclic continuation of internal indexed operations, never lexical borrow edges.
use std::collections::BTreeSet;

use super::{
    BorrowIdentity, BorrowIndex, Errors, LayoutTypeId, MAX_ACTIVE_BORROWS_PER_FUNCTION,
    PlaceIdentity, Span, VerifiedBorrowAccess, VerifiedTerminator, error_at, layout_type,
    places_overlap, raw,
};

/// Maximum total transient identities cached at block entries in one function.
pub const MAX_CONTINUED_INDEXED_IDENTITIES_PER_FUNCTION: usize = 262_144;

fn reserve_incoming(total: &mut usize, count: usize, errors: &mut Errors) -> bool {
    if let Some(next) = total.checked_add(count)
        && next <= MAX_CONTINUED_INDEXED_IDENTITIES_PER_FUNCTION
    {
        *total = next;
        return true;
    }
    errors.limit(
        "continued indexed identities per function",
        MAX_CONTINUED_INDEXED_IDENTITIES_PER_FUNCTION,
    );
    false
}

pub(super) fn advance(active: &mut Vec<raw::BorrowId>, kind: &raw::InstructionKind) {
    use raw::InstructionKind as I;
    match kind {
        I::BeginBorrow(definition)
        | I::BeginIndexedBorrow { definition, .. }
        | I::BeginIndexedAccess { definition, .. } => active.push(definition.id),
        I::EndBorrow { borrow } => active.retain(|id| id != borrow),
        I::ProjectIndexedBorrow { parent, borrow, .. }
        | I::BindIndexedBorrow { parent, borrow } => {
            active.retain(|id| id != parent);
            active.push(*borrow);
        }
        _ => {}
    }
}

pub(super) fn derive(
    function: &raw::Function,
    successors: &[Vec<usize>],
    dominators: &[BTreeSet<usize>],
    borrows: &BorrowIndex,
    errors: &mut Errors,
) -> Vec<Vec<raw::BorrowId>> {
    if !function.blocks.iter().flat_map(|block| &block.instructions).any(|instruction| {
        matches!(instruction.kind, raw::InstructionKind::BeginIndexedAccess { .. })
    }) {
        return vec![Vec::new(); function.blocks.len()];
    }
    let mut incoming = vec![None; function.blocks.len()];
    incoming[0] = Some(Vec::new());
    let mut queue = BTreeSet::from([0usize]);
    let mut cached = 0;
    while let Some(block) = queue.pop_first() {
        let mut active = incoming[block].clone().expect("reachable predecessor");
        let body = &function.blocks[block];
        for instruction in &body.instructions {
            advance(&mut active, &instruction.kind);
            if active
                .len()
                .checked_add(function.borrow_parameters.len())
                .is_none_or(|count| count > MAX_ACTIVE_BORROWS_PER_FUNCTION)
            {
                errors.limit(
                    "simultaneously active borrows per function",
                    MAX_ACTIVE_BORROWS_PER_FUNCTION,
                );
                return Vec::new();
            }
        }
        let terminator = &body.terminators[0];
        // Lexical authorities never become incoming state. The ownership pass reports
        // their edge prohibition after validating each begin/end, preserving diagnostics.
        active.retain(|id| borrows.origin(*id).is_some());
        for &target in &successors[block] {
            if !active.is_empty() && dominators[block].contains(&target) {
                errors.push(error_at(
                    "ZRYNA-I3011",
                    terminator.span,
                    "transient indexed authority remains active on a backedge",
                    "complete indexed operations before restoring a loop header",
                ));
                return Vec::new();
            }
            match &incoming[target] {
                None => {
                    if !reserve_incoming(&mut cached, active.len(), errors) {
                        return Vec::new();
                    }
                    incoming[target] = Some(active.clone());
                    queue.insert(target);
                }
                Some(existing) if existing != &active => {
                    errors.push(error_at("ZRYNA-I3011", terminator.span,
                        "transient indexed identities or issuance order differ across a continuation",
                        "preserve the same live indexed authority on every incoming edge"));
                    return Vec::new();
                }
                Some(_) => {}
            }
        }
    }
    incoming.into_iter().map(Option::unwrap_or_default).collect()
}

pub(super) fn verify_edge_consumption(
    edge: &raw::Edge,
    owners: &[Option<raw::PlaceId>],
    function: &raw::Function,
    active: &[Option<(raw::PlaceId, raw::BorrowAccess)>],
    at: Span,
    errors: &mut Errors,
) {
    if edge
        .arguments
        .iter()
        .filter_map(|value| owners.get(value.0 as usize).copied().flatten())
        .any(|owner| {
            active
                .iter()
                .flatten()
                .any(|(region, _)| places_overlap(owner, *region, &function.places))
        })
    {
        errors.push(error_at(
            "ZRYNA-I3010",
            at,
            "owned edge argument overlaps an active borrow",
            "retain the borrowed owner until its indexed operation completes",
        ));
    }
}

pub(super) fn verify_completion(
    function: &raw::Function,
    borrows: &BorrowIndex,
    active: &[Option<(raw::PlaceId, raw::BorrowAccess)>],
    terminator: &raw::SpannedTerminator,
    errors: &mut Errors,
) {
    if active.iter().enumerate().skip(function.borrow_parameters.len()).any(|(id, entry)| {
        entry.is_some()
            && (borrows.origin(raw::BorrowId(u32::try_from(id).expect("bounded borrow"))).is_none()
                || matches!(terminator.kind, raw::Terminator::Return { .. }))
    }) {
        errors.push(error_at(
            "ZRYNA-I3011",
            terminator.span,
            "borrow remains active at a control-flow edge",
            "end every borrow before a branch, jump, loop edge, return, or trap",
        ));
    }
}

/// Exact internal access retained across this terminator's ordinary success edges.
///
/// ```compile_fail
/// fn forge(mut view: zryna_ir::data_ownership_v1::VerifiedContinuedIndexedAccess) {
///     view.borrow = todo!();
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedContinuedIndexedAccess {
    borrow: BorrowIdentity,
    region: PlaceIdentity,
    referent: LayoutTypeId,
    access: VerifiedBorrowAccess,
}

#[allow(missing_docs)]
impl VerifiedContinuedIndexedAccess {
    #[must_use]
    pub const fn borrow(self) -> BorrowIdentity {
        self.borrow
    }
    #[must_use]
    pub const fn region(self) -> PlaceIdentity {
        self.region
    }
    #[must_use]
    pub const fn referent(self) -> LayoutTypeId {
        self.referent
    }
    #[must_use]
    pub const fn access(self) -> VerifiedBorrowAccess {
        self.access
    }
}

impl VerifiedTerminator<'_> {
    /// Same dominating transient identities, in issuance order, on every success edge.
    /// This is not a borrowed edge argument or lexical reference lifetime extension.
    ///
    /// # Panics
    /// Panics only if a verified indexed definition loses its sealed region or layout.
    #[must_use]
    pub fn continued_indexed_accesses(
        self,
    ) -> impl ExactSizeIterator<Item = VerifiedContinuedIndexedAccess> {
        let mut result = Vec::new();
        if !matches!(
            self.terminator.kind,
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. }
        ) {
            let index = self.function.borrows();
            let function = self.function.function;
            for id in index.active_before(
                function,
                self.block_index,
                function.blocks[self.block_index].instructions.len(),
            ) {
                if index.origin(id).is_none() {
                    continue;
                }
                let (ty, access) = index.definition(id).expect("verified referent");
                let owner = self.function.id();
                result.push(VerifiedContinuedIndexedAccess {
                    borrow: BorrowIdentity { owner, index: id.0 },
                    region: PlaceIdentity {
                        owner,
                        index: index.region(id).expect("verified region").0,
                    },
                    referent: layout_type(&self.function.owner.linear32, ty)
                        .expect("verified layout")
                        .id(),
                    access: access.into(),
                });
            }
        }
        result.into_iter()
    }

    /// Exact reverse-issued authorities ended by controlled trap or upgrade failure cleanup.
    #[must_use]
    pub fn failure_ended_borrows(self) -> impl ExactSizeIterator<Item = BorrowIdentity> {
        let mut active = Vec::new();
        if matches!(
            self.terminator.kind,
            raw::Terminator::Trap { .. } | raw::Terminator::WeakUpgradeBranch { .. }
        ) {
            let function = self.function.function;
            active = self.function.borrows().active_before(
                function,
                self.block_index,
                function.blocks[self.block_index].instructions.len(),
            );
        }
        let owner = self.function.id();
        active.into_iter().rev().map(move |id| BorrowIdentity { owner, index: id.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_indexed_resources_checked_cache_accounting_is_atomic() {
        let mut total = MAX_CONTINUED_INDEXED_IDENTITIES_PER_FUNCTION - 1;
        assert!(reserve_incoming(&mut total, 1, &mut Errors::default()));
        let exact = total;
        let mut errors = Errors::default();
        assert!(!reserve_incoming(&mut total, 1, &mut errors));
        assert_eq!(total, exact);
        assert_eq!(errors.diagnostics[0].code(), "ZRYNA-I3201");
        let mut overflow = usize::MAX;
        assert!(!reserve_incoming(&mut overflow, 1, &mut Errors::default()));
        assert_eq!(overflow, usize::MAX);
        let mut fresh = 0;
        assert!(reserve_incoming(&mut fresh, 1, &mut Errors::default()));
        assert_eq!(fresh, 1);
    }
}
