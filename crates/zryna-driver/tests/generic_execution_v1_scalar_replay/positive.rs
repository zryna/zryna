//! Selective source-derived operation inventory through the genuine complete execution issuer.
use super::source;
use std::collections::BTreeMap;
use zryna_ir::generic_v1::execution_v1::raw;
pub(super) fn check(case: &str) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(!wire.is_empty());
            let mut observed = BTreeMap::<String, u64>::new();
            for op in program.operations() {
                let name = match op.kind() {
                    raw::Operation::I32Neg { .. } => Some("I32Neg".to_owned()),
                    raw::Operation::ScalarBinary { kind, .. } => Some(format!("{kind:?}")),
                    raw::Operation::SourceCall { .. } => Some("SourceCall".to_owned()),
                    raw::Operation::ClosedGenericCall { .. } => {
                        Some("ClosedGenericCall".to_owned())
                    }
                    _ => None,
                };
                if let Some(name) = name {
                    *observed.entry(name).or_default() += 1;
                }
            }
            let row = source::row(case);
            let expected: BTreeMap<String, u64> =
                serde_json::from_value(row["original_operation_inventory"].clone())
                    .expect("pre-output selective opcode inventory");
            assert_eq!(observed, expected, "{case}/{provider}: exact admitted source opcode roles");
            assert_eq!(row["runtime_ownership"]["canonical_abi_entries"], 0);
            assert_eq!(program.scalar_abi().exports().count(), 1);
        });
    }
}
