//! Source-bound internal command proof; no public CLI target selection is enabled here.

use sha2::{Digest, Sha256};
use zryna_backend_webassembly::{ValidatedCommandComponent, WitSource, emit_command_self_check};
use zryna_diagnostics::Diagnostic;
use zryna_frontend::VerifiedFrontendProvider;
use zryna_source::SourceMap;

use crate::{
    SourceToIrError,
    profile_composition::{PureCommandSource, compile_pure_command},
};

mod denied;
mod envelope;
mod session;

#[cfg(test)]
mod tests;

const POLICY: &str = "zryna.command-self-check.deny-all.v1";

/// Explicit host policy for this proof slice: no requested or granted host capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandHostPolicy {
    revision: &'static str,
}

impl CommandHostPolicy {
    /// Selects the only admitted policy, with every host operation denied.
    #[must_use]
    pub const fn deny_all() -> Self {
        Self { revision: POLICY }
    }
}

/// A source, composition and component binding that can only be produced by real compilation.
pub struct PreparedCommand<'source> {
    source: PureCommandSource<'source>,
    component: ValidatedCommandComponent,
    policy: CommandHostPolicy,
    binding: [u8; 32],
}

impl PreparedCommand<'_> {
    /// Returns non-fatal authenticated frontend diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.source.diagnostics()
    }

    /// Returns the bound artifact observation; its private authority remains immutable.
    #[must_use]
    pub const fn component(&self) -> &ValidatedCommandComponent {
        &self.component
    }

    /// Executes one fresh instance and returns the typed command result, independent of process status.
    ///
    /// # Errors
    ///
    /// Rejects stale source/composition/artifact/policy bindings before runtime construction;
    /// traps, denied calls and exhausted execution limits invalidate and destroy their store.
    pub fn execute(&self, policy: CommandHostPolicy) -> Result<Result<(), ()>, Vec<Diagnostic>> {
        if policy.revision != POLICY || policy != self.policy {
            return Err(vec![invalid("command host policy identity changed")]);
        }
        self.source.revalidate(self.component.world_audit())?;
        self.component.revalidate(self.source.program()).map_err(|error| vec![error])?;
        if binding(&self.source, &self.component, policy)? != self.binding {
            return Err(vec![invalid(
                "command source, ABI, composition or artifact binding changed",
            )]);
        }
        let mut session = session::Session::new(&self.component).map_err(|error| vec![error])?;
        let result = session.invoke().map_err(|error| vec![error]);
        if session.denial_entries != 0 || session.denial.is_some() {
            return Err(vec![invalid("pure command entered a denied host callback")]);
        }
        result
    }
}

/// Compiles the matching source and seals the private scalar self-check command arrangement.
///
/// # Errors
///
/// Rejects unsupported source, unauthenticated WIT, mismatched invocation, nonempty composition
/// requirements, unsupported host policy, or any independent component audit failure.
pub fn prepare_command_self_check<'source, Provider: VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &'source SourceMap,
    wit: &[WitSource],
    export: &str,
    arguments: &[i32],
    expected: i32,
    policy: CommandHostPolicy,
) -> Result<PreparedCommand<'source>, SourceToIrError> {
    if policy.revision != POLICY {
        return Err(SourceToIrError::Rejected(vec![invalid("unsupported command host policy")]));
    }
    let source = compile_pure_command(frontend, sources, wit)?;
    let component = emit_command_self_check(source.program(), wit, export, arguments, expected)
        .map_err(|error| SourceToIrError::Rejected(vec![error]))?;
    source.revalidate(component.world_audit()).map_err(SourceToIrError::Rejected)?;
    let binding = binding(&source, &component, policy).map_err(SourceToIrError::Rejected)?;
    Ok(PreparedCommand { source, component, policy, binding })
}

fn binding(
    source: &PureCommandSource<'_>,
    component: &ValidatedCommandComponent,
    policy: CommandHostPolicy,
) -> Result<[u8; 32], Vec<Diagnostic>> {
    let mut digest = Sha256::new();
    digest.update(b"zryna.command-self-check.binding.v1\0");
    digest.update(source.source_identity());
    digest.update(source.composition_identity());
    digest.update(component.core_digest());
    digest.update(component.digest());
    for text in [component.bridge_revision(), policy.revision] {
        text_digest(&mut digest, text);
    }
    digest.update(0_u64.to_le_bytes()); // Explicit empty host request set.
    digest.update(0_u64.to_le_bytes()); // Explicit empty host grant set.
    let abi = source.program().scalar_abi();
    digest.update((abi.version() as u16).to_le_bytes());
    digest.update((abi.exports().len() as u64).to_le_bytes());
    for export in abi.exports() {
        text_digest(&mut digest, export.logical_name().as_str());
        text_digest(&mut digest, export.webassembly_name().as_str());
        digest.update((export.parameters().len() as u64).to_le_bytes());
        for ty in export.parameters().iter().copied().chain([export.result()]) {
            let byte = match ty {
                zryna_abi::ScalarType::I32 => 1,
                zryna_abi::ScalarType::Bool => {
                    return Err(vec![invalid("command ABI is not entirely i32")]);
                }
            };
            digest.update([byte]);
        }
    }
    for world in component.world_audit().worlds() {
        text_digest(&mut digest, world.identity());
        for names in [world.explicit_imports(), world.resolved_imports(), world.exports()] {
            digest.update((names.len() as u64).to_le_bytes());
            for name in names {
                text_digest(&mut digest, name);
            }
        }
    }
    Ok(digest.finalize().into())
}

fn text_digest(digest: &mut Sha256, text: &str) {
    digest.update((text.len() as u64).to_le_bytes());
    digest.update(text.as_bytes());
}

fn invalid(message: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C4020",
        None,
        message,
        "prepare the command from one authenticated source and denied host policy",
    )
}
