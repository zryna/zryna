use super::{PlaygroundDaemonError, Result, reject};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Preparing,
    Member,
    Audited,
    InputSealed,
    Dispatched,
    Quiescent,
    Collected,
    Removed,
    Failed,
}

/// This state is owned by one host operation; no response is issued before removal.
pub(super) struct State {
    pub owner: String,
    pub phase: Phase,
}

impl State {
    pub fn advance(&mut self, expected: Phase, next: Phase) -> Result<()> {
        if self.phase != expected
            || !matches!(
                (expected, next),
                (Phase::Preparing, Phase::Member)
                    | (Phase::Member, Phase::Audited)
                    | (Phase::Audited, Phase::InputSealed)
                    | (Phase::InputSealed, Phase::Dispatched)
                    | (Phase::Dispatched, Phase::Quiescent)
                    | (Phase::Quiescent, Phase::Collected)
                    | (Phase::Collected, Phase::Removed)
            )
        {
            return Err(reject("PLAYGROUND-HOST-STATE", "operation ownership transition refused"));
        }
        self.phase = next;
        Ok(())
    }

    pub fn cleanup_failure(
        &mut self,
        primary: Option<PlaygroundDaemonError>,
        cleanup: PlaygroundDaemonError,
    ) -> PlaygroundDaemonError {
        self.phase = Phase::Failed;
        PlaygroundDaemonError {
            reason: "PLAYGROUND-HOST-TEARDOWN",
            detail: "owned playground namespace or storage cleanup unconfirmed",
            primary: primary.map(Box::new),
            cleanup: Some(Box::new(cleanup)),
            owner: Some(self.owner.clone()),
            exit_code: None,
            stderr: Vec::new(),
        }
    }

    pub fn success(&self, cancel: &AtomicBool) -> Result<()> {
        if self.phase != Phase::Removed {
            return Err(reject(
                "PLAYGROUND-HOST-TEARDOWN",
                "successful response requires owned removal",
            ));
        }
        if cancel.load(Ordering::Acquire) {
            return Err(reject(
                "PLAYGROUND-CANCELLED",
                "cancellation observed after complete owned cleanup",
            ));
        }
        Ok(())
    }
}
