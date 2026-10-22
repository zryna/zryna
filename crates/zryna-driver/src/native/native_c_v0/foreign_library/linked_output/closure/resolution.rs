//! Indexed declared-symbol compatibility, not runtime relocation or resolver attestation.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

type Definitions<'a> = BTreeMap<(String, Option<String>), &'a symbols::Symbol>;
pub(super) struct Index<'a> {
    definitions: BTreeMap<String, Definitions<'a>>,
    versions: BTreeMap<String, BTreeMap<String, u32>>,
    dependencies: BTreeMap<String, &'a [String]>,
}
impl<'a> Index<'a> {
    pub(super) fn new(
        providers: &'a [(String, ObjectInventory)],
        names: &BTreeMap<String, usize>,
    ) -> Result<Self, Diagnostic> {
        let mut result = Self {
            definitions: BTreeMap::new(),
            versions: BTreeMap::new(),
            dependencies: BTreeMap::new(),
        };
        for (name, &position) in names {
            let object = &providers[position].1;
            let mut definitions = BTreeMap::new();
            let mut versions = BTreeMap::new();
            for version in object.versions.definitions.values() {
                if versions.insert(version.name.clone(), version.hash).is_some() {
                    return Err(rejected());
                }
            }
            for symbol in object.symbols.iter().skip(1) {
                if !symbol.defined
                    || !symbol.lookup_reachable
                    || !matches!(symbol.binding, 1 | 2)
                    || !matches!(symbol.visibility, 0 | 3)
                    || symbol.version == 0
                {
                    continue;
                }
                let version =
                    object.versions.definitions.get(&symbol.version).map(|v| v.name.clone());
                if symbol.version > 1 && version.is_none() {
                    continue;
                } // Imported COPY records are not provider definitions.
                if definitions.insert((symbol.name.clone(), version.clone()), symbol).is_some() {
                    return Err(rejected());
                }
                if version.is_some()
                    && !symbol.hidden_version
                    && definitions.insert((symbol.name.clone(), None), symbol).is_some()
                {
                    return Err(rejected());
                }
            }
            result.definitions.insert(name.clone(), definitions);
            result.versions.insert(name.clone(), versions);
            result.dependencies.insert(name.clone(), &object.needed);
        }
        Ok(result)
    }
    pub(super) fn check(&self, object: &ObjectInventory) -> Result<(), Diagnostic> {
        for need in object.versions.needs.values() {
            let provider = need.provider.as_ref().ok_or_else(rejected)?;
            if self.versions.get(provider).and_then(|v| v.get(&need.name)) != Some(&need.hash) {
                return Err(rejected());
            }
        }
        let mut scope = BTreeSet::new();
        for name in &object.needed {
            self.expand(name, &mut scope)?;
        }
        for symbol in object.symbols.iter().skip(1) {
            let need = object.versions.needs.get(&symbol.version);
            if symbol.defined && need.is_none() {
                continue;
            }
            if symbol.binding == 2 && need.is_none() {
                continue;
            } // Explicit unresolved weak, no address claim.
            let key = (symbol.name.clone(), need.map(|v| v.name.clone()));
            let matches = |name: &String| {
                self.definitions
                    .get(name)
                    .and_then(|d| d.get(&key))
                    .is_some_and(|definition| compatible(symbol.kind, definition.kind))
            };
            let found = if let Some(version) = need {
                matches(version.provider.as_ref().ok_or_else(rejected)?)
            } else {
                scope.iter().any(matches)
            };
            if !found {
                return Err(rejected());
            }
        }
        Ok(())
    }
    fn expand(&self, name: &str, scope: &mut BTreeSet<String>) -> Result<(), Diagnostic> {
        if !scope.insert(name.to_owned()) {
            return Ok(());
        }
        for next in *self.dependencies.get(name).ok_or_else(rejected)? {
            self.expand(next, scope)?;
        }
        Ok(())
    }
}
fn compatible(required: u8, provided: u8) -> bool {
    matches!((required, provided), (0, 0) | (1, 1) | (2, 2 | 10) | (6, 6) | (10, 10))
}
