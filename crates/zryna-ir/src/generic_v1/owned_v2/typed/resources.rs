//! One independent aggregate operand account for core and private owned claims.
use super::{Admission, Failure, graph, raw, reject};
use crate::{data_ownership_v1::MAX_AGGREGATE_OPERANDS, generic_v1::budget};

pub(super) fn check(claim: &raw::Program, admission: Admission) -> Result<(), Failure> {
    let mut used = 0usize;
    for function in &claim.graph.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let graph::Operation::ClosedEnumConstruct { payload: Some(_), .. } =
                    &instruction.operation
                {
                    charge(&mut used, 1)?;
                }
            }
        }
    }
    for operation in claim.extensions.iter().flatten().map(|e| &e.operation) {
        match operation {
            raw::Operation::CloneStruct { .. } if admission != Admission::StructCloneV5 => {
                return Err(reject("Struct Clone outside private V5 issuer"));
            }
            raw::Operation::StructConstruct { fields, .. } => {
                if !admission.has_structs() {
                    return Err(reject("Struct operation outside private v4 issuer"));
                }
                if fields.len() > zryna_layout::MAX_MEMBERS_PER_DECLARATION {
                    return Err(budget("Struct field operand vector exceeds 1024"));
                }
                charge(&mut used, fields.len())?;
            }
            raw::Operation::CopyStructField { .. } if !admission.has_structs() => {
                return Err(reject("Struct observation outside private v4 issuer"));
            }
            _ => {}
        }
    }
    Ok(())
}
fn charge(used: &mut usize, extra: usize) -> Result<(), Failure> {
    *used = used
        .checked_add(extra)
        .filter(|n| *n <= MAX_AGGREGATE_OPERANDS)
        .ok_or_else(|| budget("combined core and Struct aggregate operand ceiling"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zryna_source::UntrustedSpan;
    const AT: UntrustedSpan = UntrustedSpan { file: 0, start: 0, end: 1 };

    fn claim() -> raw::Program {
        let mut functions = Vec::new();
        let mut extensions = Vec::new();
        for index in 0..16 {
            // These deliberately untrusted IDs isolate physical credits, not a typed seal.
            let instructions = if index == 0 {
                (0..1024)
                    .map(|id| graph::Instruction {
                        result: graph::Definition { id, ty: graph::Type::Stored(3) },
                        span: AT,
                        operation: graph::Operation::ClosedEnumConstruct {
                            ty: 3,
                            ordinal: 0,
                            payload: Some(0),
                        },
                    })
                    .collect()
            } else {
                Vec::new()
            };
            functions.push(graph::Function {
                key: vec![],
                span: AT,
                public_export: None,
                parameters: vec![],
                result: graph::Type::Unit,
                blocks: vec![graph::Block {
                    id: 0,
                    span: AT,
                    parameters: vec![],
                    instructions,
                    terminator: graph::Terminator::Return(0),
                }],
            });
            extensions.push(
                (0..if index == 15 { 15 } else { 16 })
                    .map(|result| raw::Extension {
                        result,
                        operation: raw::Operation::StructConstruct { ty: 3, fields: vec![0; 1024] },
                    })
                    .collect(),
            );
        }
        raw::Program {
            graph: graph::Program {
                modules: vec![],
                declarations: vec![],
                type_keys: vec![],
                universe: [0; 32],
                linear32: [0; 32],
                linux_x86_64: [0; 32],
                functions,
            },
            extensions,
            plans: vec![],
        }
    }
    #[test]
    fn independent_combined_operand_ceiling_spans_functions_and_core_extensions() {
        let mut c = claim();
        // 255 * 1024 Struct fields + 1024 core enum operands = exactly 262144.
        check(&c, Admission::StructV4).expect("exact combined physical credit");
        assert!(check(&c, Admission::LegacyOwned).is_err(), "untrusted claims cannot select v4");
        c.extensions[15].push(raw::Extension {
            result: 15,
            operation: raw::Operation::CopyStructField { value: 0, ordinal: 0 },
        });
        check(&c, Admission::StructV4).expect("observation does not construct aggregate");
        c.extensions[15].push(raw::Extension {
            result: 16,
            operation: raw::Operation::StructConstruct { ty: 3, fields: vec![0] },
        });
        assert!(check(&c, Admission::StructV4).is_err(), "first extra stored field rejects");
        c.graph.functions[0].blocks[0].instructions.pop();
        check(&c, Admission::StructV4).expect("removing one core operand restores exact credit");
        c.extensions[15].last_mut().expect("field").operation =
            raw::Operation::StructConstruct { ty: 3, fields: vec![0; 1025] };
        assert!(check(&c, Admission::StructV4).is_err(), "per-declaration cap is independent");
    }
}
