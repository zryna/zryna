//! NEW arithmetic model only: raw counters/requests grant no execution or resource-effect authority.
use crate::{MAX_ALLOCATION_OPERATIONS, MAX_LIVE_ALLOCATIONS, MAX_STATUS_TRANSITIONS};
/// Invocation arithmetic state; seeded values are test/model data, never an executable seal.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CounterState {
    /// Admitted positive backing requests, including target failure.
    pub attempts: u64,
    /// Successful currently live backing blocks.
    pub live: u64,
    /// Spent admitted outer canonical logical events.
    pub spent: u64,
    /// Exact mandatory future logical cleanup debt.
    pub held: u64,
}
/// Model request; a target must separately prove its source-bound effect/classification.
#[derive(Clone, Copy, Debug)]
pub struct Request {
    /// Canonical-valid positive backing acquisition/replacement request.
    pub attempt: bool,
    /// Prospective successful owner's new cleanup debt (zero for known failure/transfer).
    pub new_hold: u64,
    /// Successful new block must fit while an old growth block is still live.
    pub temporary_live: bool,
}
/// Global pre-call denial and malformed private state are distinct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fault {
    /// No logical helper was admitted; source CAPACITY2, with counters unchanged.
    Denied,
    /// Private invariant ABI255; not a source quota result.
    Invariant,
}
/// Consumed exactly once after its admitted canonical call; no caller-constructible receipt.
#[derive(Debug)]
pub struct Admission {
    before: CounterState,
    reserved: CounterState,
    request: Request,
}
impl CounterState {
    /// Validate the frozen scalar-entry coupling and held-credit invariant.
    /// # Errors
    /// Returns Invariant for corrupt/unreachable counter arithmetic.
    pub fn checked(self) -> Result<(), Fault> {
        if self.attempts > MAX_ALLOCATION_OPERATIONS
            || self.live > MAX_LIVE_ALLOCATIONS
            || self.live > self.attempts
            || self.spent > MAX_STATUS_TRANSITIONS
            || self.held > MAX_STATUS_TRANSITIONS
            || self.spent.checked_add(self.held).is_none_or(|n| n > MAX_STATUS_TRANSITIONS)
        {
            return Err(Fault::Invariant);
        }
        Ok(())
    }
    /// Atomically prepay a real classified outer event before canonical side effects.
    /// # Errors
    /// Denied leaves the original state intact; malformed state is Invariant.
    pub fn forward(&mut self, request: Request) -> Result<Admission, Fault> {
        self.checked()?;
        if request.temporary_live && !request.attempt {
            return Err(Fault::Invariant);
        }
        let attempts =
            self.attempts.checked_add(u64::from(request.attempt)).ok_or(Fault::Denied)?;
        let spent = self.spent.checked_add(1).ok_or(Fault::Denied)?;
        let held = self.held.checked_add(request.new_hold).ok_or(Fault::Denied)?;
        let temporary =
            self.live.checked_add(u64::from(request.temporary_live)).ok_or(Fault::Denied)?;
        if attempts > MAX_ALLOCATION_OPERATIONS
            || temporary > MAX_LIVE_ALLOCATIONS
            || spent.checked_add(held).is_none_or(|n| n > MAX_STATUS_TRANSITIONS)
        {
            return Err(Fault::Denied);
        }
        let before = *self;
        let reserved = Self { attempts, live: self.live, spent, held };
        *self = reserved;
        Ok(Admission { before, reserved, request })
    }
    /// Commit a successful new block or cancel only the failed prospective debt.
    /// Growth's subsequent old-block live decrement belongs to its separate typed effect.
    /// # Errors
    /// A mismatched/interleaved/forged reservation is Invariant and leaves state intact.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "Admission is a consumed non-Copy transaction receipt; borrowing would allow reuse"
    )]
    pub fn complete(&mut self, admission: Admission, success: bool) -> Result<(), Fault> {
        if *self != admission.reserved {
            return Err(Fault::Invariant);
        }
        let mut next = *self;
        if success {
            next.live = next
                .live
                .checked_add(u64::from(admission.request.temporary_live))
                .ok_or(Fault::Invariant)?;
        } else {
            next.held = admission.before.held;
        }
        next.checked()?;
        *self = next;
        Ok(())
    }
    /// Complete an admitted positive Vec growth as one outer logical event.
    /// The old block retires only on success; internal C Grow/Release adds no S or H.
    /// No-growth and known pre-backing failure use ordinary `complete` instead.
    /// # Errors
    /// Rejects a different/interleaved receipt, non-growth request or unfunded old storage.
    /// Rejection leaves the currently reserved state intact.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "Admission is a consumed non-Copy transaction receipt; borrowing would allow reuse"
    )]
    pub fn complete_growth(
        &mut self,
        admission: Admission,
        success: bool,
        old_positive: bool,
    ) -> Result<(), Fault> {
        if *self != admission.reserved
            || !admission.request.attempt
            || !admission.request.temporary_live
            || admission.request.new_hold != 0
            || admission.before.held == 0
            || admission.before.live < u64::from(old_positive)
        {
            return Err(Fault::Invariant);
        }
        let mut next = *self;
        if success {
            next.live = admission
                .before
                .live
                .checked_add(1)
                .and_then(|n| n.checked_sub(u64::from(old_positive)))
                .ok_or(Fault::Invariant)?;
        }
        next.checked()?;
        *self = next;
        Ok(())
    }
    /// Spend an existing release credit; this operation has no quota-denial outcome.
    /// # Errors
    /// Missing debt/live provenance is Invariant, with no publication or wrap.
    pub fn release(&mut self, positive: bool) -> Result<(), Fault> {
        self.checked()?;
        let next = Self {
            attempts: self.attempts,
            live: self.live.checked_sub(u64::from(positive)).ok_or(Fault::Invariant)?,
            spent: self.spent.checked_add(1).ok_or(Fault::Invariant)?,
            held: self.held.checked_sub(1).ok_or(Fault::Invariant)?,
        };
        next.checked()?;
        *self = next;
        Ok(())
    }
}
