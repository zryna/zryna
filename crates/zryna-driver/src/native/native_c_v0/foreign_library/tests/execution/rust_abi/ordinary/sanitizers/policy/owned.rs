//! Exact independently observed hosted data; execution requires separate source admission.
use super::*;
use std::fmt::Write as _;
struct ToolPin {
    path: &'static str,
    bytes: u64,
    sha256: [u8; 32],
}
struct OwnerEvidence {
    candidate: &'static str,
    run: u64,
    attempt: u64,
    job: u64,
    original_zip_sha256: [u8; 32],
    independent_review_sha256: [u8; 32],
}
struct OwnerProfile {
    evidence: OwnerEvidence,
    tools: [ToolPin; 3],
    positive: &'static Bindings,
    heap: &'static Bindings,
    signed: &'static Bindings,
}
// Literal candidate87f06a source profile, pending independent CODE review/admission.
// No JSON/report/environment/tool-family value can construct or select an approval.
mod heap;
mod hosted;
mod materials;
mod positive;
mod signed;
mod tests;
fn independently_reviewed_profile() -> &'static OwnerProfile {
    &hosted::VALUE
}
#[derive(Clone, Copy)]
pub(in super::super) struct RuntimeProfile<'a> {
    owner: &'static OwnerProfile,
    tools: &'a material::OrdinaryTools,
}
#[derive(Clone, Copy)]
pub(in super::super) struct RuntimeCase<'a> {
    owner: &'static OwnerProfile,
    tools: &'a material::OrdinaryTools,
    bindings: &'static Bindings,
    case: Case,
}
fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().fold(String::with_capacity(64), |mut text, byte| {
        write!(text, "{byte:02x}").expect("hex digest write");
        text
    })
}
impl OwnerProfile {
    fn check(&self, tools: &material::OrdinaryTools) -> Result<(), Diagnostic> {
        tools.check()?;
        let report = tools.report(); // Constructor-owned actual tools, never caller JSON.
        let pins = report["pins"].as_array().ok_or_else(rejected)?;
        self.check_pins(pins)
    }
    fn check_pins(&self, pins: &[serde_json::Value]) -> Result<(), Diagnostic> {
        if pins.len() != self.tools.len() {
            return Err(rejected());
        }
        for (actual, expected) in pins.iter().zip(&self.tools) {
            if actual["path"].as_str() != Some(expected.path)
                || actual["bytes"].as_u64() != Some(expected.bytes)
                || actual["sha256"].as_str() != Some(hex(&expected.sha256).as_str())
            {
                return Err(rejected());
            }
        }
        // Actual originating epochs remain enforced by OrdinaryTools::check per run.
        Ok(())
    }
}
impl<'a> RuntimeProfile<'a> {
    pub(in super::super) fn acquire(
        tools: &'a material::OrdinaryTools,
    ) -> Result<Self, Diagnostic> {
        let owner = independently_reviewed_profile();
        owner.check(tools)?;
        for (case, bindings) in [
            (Case::Positive, owner.positive),
            (Case::HeapOverrun, owner.heap),
            (Case::SignedOverflow, owner.signed),
        ] {
            bindings.materials(case)?;
        }
        Ok(Self { owner, tools })
    }
    pub(in super::super) fn case(&self, case: Case) -> Result<RuntimeCase<'a>, Diagnostic> {
        let bindings = match case {
            Case::Positive => self.owner.positive,
            Case::HeapOverrun => self.owner.heap,
            Case::SignedOverflow => self.owner.signed,
        };
        let value = RuntimeCase { owner: self.owner, tools: self.tools, bindings, case };
        value.check(case, self.tools)?;
        Ok(value)
    }
    pub(in super::super) fn report(&self) -> serde_json::Value {
        let evidence = &self.owner.evidence;
        serde_json::json!({
            "scope":"Explicit reviewed ordinary C-only owner profile; no production/recipe grant",
            "originating_candidate":evidence.candidate,"originating_run":evidence.run,
            "originating_attempt":evidence.attempt,"originating_job":evidence.job,
            "original_zip_sha256":hex(&evidence.original_zip_sha256),
            "independent_review_sha256":hex(&evidence.independent_review_sha256)
        })
    }
}
impl RuntimeCase<'_> {
    pub(in super::super) fn check(
        &self,
        case: Case,
        tools: &material::OrdinaryTools,
    ) -> Result<(), Diagnostic> {
        if self.case != case || !std::ptr::eq(self.tools, tools) {
            return Err(rejected());
        }
        self.owner.check(tools)?;
        self.bindings.materials(case)
    }
    pub(in super::super) fn bindings(&self) -> &'static Bindings {
        self.bindings
    }
}
