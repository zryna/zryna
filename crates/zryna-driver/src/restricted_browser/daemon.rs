//! Fixed local-daemon playground ownership, separate from native recipe authority.
//!
//! The trusted caller supplies independently reviewed immutable image and policy pins.
//! Neither a submission nor an adjacent inventory authenticates those expectations.

mod approval;
mod archive;
mod archive_header;
mod cleanup;
mod client;
mod config;
mod execute;
mod identity;
mod image;
mod kernel;
mod materials;
mod owner;
mod policy;
mod resources;
mod state;

pub use approval::{PlaygroundDaemonOperatorApproval, approve_playground_daemon};
pub use materials::PlaygroundDaemonMaterialPin;
pub use owner::PlaygroundDaemonResponse;
pub use policy::playground_daemon_policy_sha256;

use std::fmt;

/// An independently approved fixed compiler/browser daemon composition.
///
/// Construction performs no execution and does not establish host enforcement.
pub struct PlaygroundDaemon {
    approval: approval::Approved,
    poisoned: bool,
}

/// A refusal of playground host admission, execution, or owned cleanup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaygroundDaemonError {
    reason: &'static str,
    detail: &'static str,
    primary: Option<Box<Self>>,
    cleanup: Option<Box<Self>>,
    owner: Option<String>,
    exit_code: Option<i64>,
    stderr: Vec<u8>,
}

impl PlaygroundDaemonError {
    /// Stable host-policy reason, separate from compiler source diagnostics.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Earlier operation failure retained when cleanup also failed.
    #[must_use]
    pub fn primary(&self) -> Option<&Self> {
        self.primary.as_deref()
    }

    /// Actual cleanup refusal retained separately from the primary operation error.
    #[must_use]
    pub fn cleanup(&self) -> Option<&Self> {
        self.cleanup.as_deref()
    }

    /// Owned attempt identity retained for recovery after unconfirmed cleanup.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// Actual fixed child exit status when it was observed before confirmed cleanup.
    #[must_use]
    pub const fn exit_code(&self) -> Option<i64> {
        self.exit_code
    }

    /// Bounded actual child stderr, separate from source diagnostics.
    #[must_use]
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

impl fmt::Display for PlaygroundDaemonError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}: {}", self.reason, self.detail)
    }
}

impl std::error::Error for PlaygroundDaemonError {}

type Result<T> = std::result::Result<T, PlaygroundDaemonError>;

fn reject(reason: &'static str, detail: &'static str) -> PlaygroundDaemonError {
    PlaygroundDaemonError {
        reason,
        detail,
        primary: None,
        cleanup: None,
        owner: None,
        exit_code: None,
        stderr: Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Compile,
    Evaluate,
}

impl Operation {
    const fn name(self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Evaluate => "evaluate",
        }
    }

    const fn seconds(self) -> u64 {
        match self {
            Self::Compile => 10,
            Self::Evaluate => 5,
        }
    }

    const fn stdout_limit(self) -> usize {
        match self {
            Self::Compile => 1_277_956,
            Self::Evaluate => 16_384,
        }
    }
}

#[cfg(test)]
mod tests;
