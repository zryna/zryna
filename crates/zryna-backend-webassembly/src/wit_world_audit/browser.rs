//! Backend-private immutable authority for the capability-free browser world.

use sha2::{Digest, Sha256};
use zryna_diagnostics::Diagnostic;

use super::{WitSource, WitWorldAudit, audit_error, audit_resolved, authenticate, resolve_sources};

pub(crate) const BROWSER_WORLD: &str = "zryna:capability-profiles/browser@0.1.0";

pub(crate) struct AuthenticatedBrowserWorld {
    audit: WitWorldAudit,
    source_digest: [u8; 32],
}

impl AuthenticatedBrowserWorld {
    pub(crate) fn new(sources: &[WitSource]) -> Result<Self, Diagnostic> {
        let authenticated = authenticate(sources)?;
        let (resolve, root) = resolve_sources(&authenticated)?;
        let audit = audit_resolved(&resolve, root)?;
        resolve
            .select_world(&[root], Some(BROWSER_WORLD))
            .map_err(|_| audit_error("authenticated browser world cannot be selected"))?;
        let browser = audit
            .worlds()
            .first()
            .filter(|candidate| candidate.identity() == BROWSER_WORLD)
            .ok_or_else(|| audit_error("authenticated browser world observation is missing"))?;
        if !browser.explicit_imports().is_empty()
            || !browser.resolved_imports().is_empty()
            || !browser.exports().is_empty()
        {
            return Err(audit_error("authenticated browser world is not capability-free"));
        }
        Ok(Self { audit, source_digest: source_digest(sources) })
    }

    pub(crate) fn audit(&self) -> &WitWorldAudit {
        &self.audit
    }

    pub(crate) const fn source_digest(&self) -> &[u8; 32] {
        &self.source_digest
    }
}

fn source_digest(sources: &[WitSource]) -> [u8; 32] {
    let mut ordered = sources.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.path().cmp(right.path()));
    let mut digest = Sha256::new();
    digest.update(b"ZRYNA-PINNED-WIT-SOURCES\0\x01");
    for source in ordered {
        digest.update(u64::try_from(source.path().len()).unwrap_or(u64::MAX).to_le_bytes());
        digest.update(source.path().as_bytes());
        digest.update(u64::try_from(source.bytes().len()).unwrap_or(u64::MAX).to_le_bytes());
        digest.update(source.bytes());
    }
    digest.finalize().into()
}
