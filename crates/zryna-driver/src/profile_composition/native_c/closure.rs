//! Complete reachable native closure budgets and contract/symbol consistency.
use super::{BTreeMap, Diagnostic, NativeAuthorities, SourceMapIdentity, error};
use zryna_semantics::native_c_v0::VerifiedOperation;

type Libraries<'a> = BTreeMap<&'a str, Vec<(&'a [u8], &'a [u8])>>;
type Operations<'a> = BTreeMap<&'a str, Vec<(SourceMapIdentity, VerifiedOperation<'a>)>>;

pub(super) fn check(native: &NativeAuthorities) -> Result<(), Vec<Diagnostic>> {
    // Graph validation proves every supplied authority is reachable from the selected root.
    // Bindings are checked before this function; raw records cannot supply either iterator.
    let mut libraries = Libraries::new();
    let mut operations = Operations::new();
    for issuer in native.values() {
        let declarations = &issuer.declarations;
        for id in declarations.library_ids() {
            let header = declarations
                .header_bytes(id)
                .ok_or_else(|| vec![error("ZRYNA-C4102", "closure library header is absent")])?;
            let policy = declarations
                .policy_bytes(id)
                .ok_or_else(|| vec![error("ZRYNA-C4102", "closure library policy is absent")])?;
            libraries.entry(id).or_default().push((header, policy));
        }
        for key in declarations.operation_keys() {
            let operation = declarations
                .operation(key)
                .ok_or_else(|| vec![error("ZRYNA-C4102", "closure operation is absent")])?;
            operations.entry(key).or_default().push((issuer.sources.identity(), operation));
        }
    }
    // Resource limits precede identity/profile errors, regardless of discovery order.
    if libraries.len() > 16 {
        return Err(vec![error(
            "ZRYNA-C4107",
            "resolved native closure libraries limit 16 exceeded at 17",
        )]);
    }
    if operations.len() > 256 {
        return Err(vec![error(
            "ZRYNA-C4107",
            "resolved native closure operations limit 256 exceeded at 257",
        )]);
    }
    for (id, contracts) in libraries {
        if contracts.iter().any(|contract| contract != &contracts[0]) {
            return Err(vec![error(
                "ZRYNA-C4102",
                format!("conflicting resolved library contract {id}"),
            )]);
        }
    }
    let mut symbols = BTreeMap::new();
    for (key, contracts) in operations {
        let (source, first) = contracts[0];
        for (other_source, operation) in &contracts[1..] {
            if !first.same_contract(*operation)
                || (first.is_export()
                    && (source != *other_source || first.source_span() != operation.source_span()))
            {
                return Err(vec![error(
                    "ZRYNA-C4102",
                    format!("conflicting resolved operation identity {key}"),
                )]);
            }
        }
        let symbol = first.symbol().to_ascii_lowercase();
        if let Some(previous) = symbols.insert(symbol, key) {
            return Err(vec![error(
                "ZRYNA-C4102",
                format!("resolved symbol collision {previous} and {key}"),
            )]);
        }
    }
    Ok(())
}
