use serde_json::Value;
use sha2::{Digest, Sha256};
use zryna_diagnostics::Diagnostic;

use super::{
    INVALID, UNSUPPORTED, error,
    model::{Capability, POLICY, Requirement, Row, Selection},
};

const REGISTRY: &str = include_str!("../../../../tests/wit-capability-profiles-v1.json");
const REGISTRY_SHA256: &str = "20e5f07b8b64ee3daf61ca4c341bc23ef9bb76bcec6973047562147b41706398";

pub(super) struct Policy(Value);

impl Policy {
    pub(super) fn load() -> Result<Self, Vec<Diagnostic>> {
        Self::from_bytes(REGISTRY.as_bytes())
    }

    pub(super) fn from_bytes(bytes: &[u8]) -> Result<Self, Vec<Diagnostic>> {
        if format!("{:x}", Sha256::digest(bytes)) != REGISTRY_SHA256 {
            return Err(vec![error(INVALID, "pinned WIT registry bytes changed")]);
        }
        let value = serde_json::from_slice(bytes)
            .map_err(|_| vec![error(INVALID, "pinned WIT registry cannot be decoded")])?;
        Ok(Self(value))
    }

    fn profile(&self, row: Row) -> Option<&Value> {
        let id = match row {
            Row::WitBrowser => "browser",
            Row::WitCommand => "command",
            Row::WitServer => "server",
            _ => return None,
        };
        self.0["profiles"].as_array()?.iter().find(|value| value["id"] == id)
    }

    pub(super) fn selection(&self, selection: &Selection) -> Result<(), Diagnostic> {
        if selection.policy_version != POLICY {
            return Err(error(UNSUPPORTED, "unknown host policy version"));
        }
        let expected = self.profile(selection.row).and_then(|value| value["world"].as_str());
        if selection.world.as_deref() != expected {
            return Err(error(UNSUPPORTED, "selected row and exact WIT world differ"));
        }
        let maximum = self.limits(selection.row);
        if selection.ceilings.iter().zip(maximum).any(|(value, max)| *value > max) {
            return Err(error(
                UNSUPPORTED,
                "host policy cannot enlarge the pinned resource ceiling",
            ));
        }
        if selection.approved.iter().any(|requirement| {
            !self.known(requirement)
                || (Self::ceiling_allows(selection.row, requirement.capability)
                    && !self.admits(selection.row, requirement))
        }) {
            return Err(error(UNSUPPORTED, "approved request has an unsupported exact interface"));
        }
        Ok(())
    }

    pub(super) fn admits(&self, row: Row, requirement: &Requirement) -> bool {
        self.profile(row)
            .and_then(|profile| profile["capabilities"].as_array())
            .and_then(|rows| rows.iter().find(|value| value["id"] == requirement.capability.name()))
            .is_some_and(|value| {
                value["interfaces"].as_array().is_some_and(|interfaces| {
                    interfaces
                        .iter()
                        .any(|interface| interface.as_str() == Some(&requirement.interface))
                })
            })
    }

    pub(super) fn known(&self, requirement: &Requirement) -> bool {
        [Row::WitCommand, Row::WitServer].into_iter().any(|row| self.admits(row, requirement))
    }

    pub(super) fn ceiling_allows(row: Row, capability: Capability) -> bool {
        match row {
            Row::JavaScriptBrowser | Row::WitServer => matches!(
                capability,
                Capability::Clock | Capability::Network | Capability::Randomness
            ),
            Row::JavaScriptNode | Row::WitCommand | Row::NativeHost => true,
            _ => false,
        }
    }

    pub(super) fn target(row: Row) -> u8 {
        match row {
            Row::UniversalJavaScript | Row::JavaScriptBrowser | Row::JavaScriptNode => 0,
            Row::UniversalWebAssembly | Row::WitBrowser | Row::WitCommand | Row::WitServer => 1,
            Row::UniversalNative | Row::NativeHost => 2,
        }
    }

    pub(super) fn limits(&self, row: Row) -> [u64; 10] {
        let Some(profile) = self.profile(row) else {
            return [0; 10];
        };
        let names = [
            ["maxSubscriptions", "maxTimers"],
            ["maxEntries", "maxTotalBytes"],
            ["maxPreopens", "maxOpenDescriptors"],
            ["maxAllowedEndpoints", "maxConcurrentOperations"],
            ["maxBytesPerCall", "maxBytesPerInstance"],
        ];
        let mut result = [0; 10];
        for (index, pair) in names.iter().enumerate() {
            for (offset, name) in pair.iter().enumerate() {
                result[index * 2 + offset] = profile["capabilities"][index]["limits"][name]
                    .as_u64()
                    .expect("authenticated registry numeric limit");
            }
        }
        result
    }
}
