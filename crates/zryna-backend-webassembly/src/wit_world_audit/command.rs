//! Backend-private immutable type authority from the exact authenticated WIT closure.

use wit_parser::{Resolve, WorldId};
use zryna_diagnostics::Diagnostic;

use super::{WitSource, WitWorldAudit, audit_error, audit_resolved, authenticate, resolve_sources};

pub(crate) const COMMAND_WORLD: &str = "zryna:capability-profiles/command@0.1.0";

pub(crate) struct AuthenticatedCommandWorld {
    resolve: Resolve,
    world: WorldId,
    audit: WitWorldAudit,
}

impl AuthenticatedCommandWorld {
    pub(crate) fn new(sources: &[WitSource]) -> Result<Self, Diagnostic> {
        let authenticated = authenticate(sources)?;
        let (resolve, root) = resolve_sources(&authenticated)?;
        let audit = audit_resolved(&resolve, root)?;
        let world = resolve
            .select_world(&[root], Some(COMMAND_WORLD))
            .map_err(|_| audit_error("authenticated command world cannot be selected"))?;
        Ok(Self { resolve, world, audit })
    }

    pub(crate) fn resolve(&self) -> &Resolve {
        &self.resolve
    }

    pub(crate) fn world(&self) -> WorldId {
        self.world
    }

    pub(crate) fn audit(&self) -> &WitWorldAudit {
        &self.audit
    }
}
