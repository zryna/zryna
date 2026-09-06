//! Independently verified Linux x86-64 native MIR for `DataOwnershipV1`.

use zryna_diagnostics::Diagnostic;

mod lower;
pub mod raw;
mod verify;

/// Opaque verified native MIR. Raw claims cannot be recovered or mutated.
#[derive(Clone, Debug)]
pub struct VerifiedMirModule {
    program: raw::Program,
}

impl VerifiedMirModule {
    /// Iterates immutable verified function views.
    #[must_use]
    pub fn functions(&self) -> impl ExactSizeIterator<Item = VerifiedFunction<'_>> {
        self.program.functions.iter().map(|function| VerifiedFunction { function })
    }
    /// Returns the exact approved runtime symbol inventory.
    #[must_use]
    pub fn runtime_symbols(&self) -> impl ExactSizeIterator<Item = &str> {
        self.program.runtime_symbols.iter().map(String::as_str)
    }
}

/// Immutable verified native MIR function view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedFunction<'a> {
    function: &'a raw::Function,
}

impl<'a> VerifiedFunction<'a> {
    /// Returns the dense module/declaration identity.
    #[must_use]
    pub const fn identity(self) -> (u32, u32) {
        (self.function.module, self.function.declaration)
    }
    /// Returns the deterministic private native symbol.
    #[must_use]
    pub fn symbol(self) -> &'a str {
        &self.function.symbol
    }
    /// Iterates immutable verified blocks.
    #[must_use]
    pub fn blocks(self) -> impl ExactSizeIterator<Item = VerifiedBlock<'a>> {
        self.function.blocks.iter().map(|block| VerifiedBlock { block })
    }
    /// Returns the exact place count.
    #[must_use]
    pub fn place_count(self) -> usize {
        self.function.places.len()
    }
    /// Returns the exact cleanup-plan count.
    #[must_use]
    pub fn cleanup_plan_count(self) -> usize {
        self.function.cleanup_plans.len()
    }
}

/// Immutable verified native MIR block view.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedBlock<'a> {
    block: &'a raw::Block,
}

impl<'a> VerifiedBlock<'a> {
    /// Returns the dense block identity.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.block.id
    }
    /// Returns the exact operation count.
    #[must_use]
    pub fn operation_count(self) -> usize {
        self.block.operations.len()
    }
    /// Returns the closed terminator.
    #[must_use]
    pub const fn terminator(self) -> &'a raw::Terminator {
        &self.block.terminator
    }
}

/// Lowers sealed Universal IR one-for-one and invokes the mandatory independent verifier.
pub use lower::{lower, lower_unverified};
/// Verifies untrusted native MIR claims against exact layout and runtime ABI authorities.
pub use verify::verify;

fn error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        code,
        None,
        message,
        "provide canonical MIR derived from the sealed DataOwnershipV1 program",
    )
}
