//! Actual graph mutation controls for fresh owner, exact progress and release recipe closure.
use super::fixture;
use crate::generic_execution_v1::recursive::{calls::Kind, check};
use cranelift_codegen::ir::{InstructionData as D, Opcode as O, condcodes::CondCode};
#[test]
fn genuine_protocol_recipes_cover_every_block_and_reject_stale_or_wrong_frame_operands() {
    for case in ["drop-only", "controlled-cleanup", "shared-weak-recursive"] {
        for kind in [Kind::Arena, Kind::Find, Kind::Drop, Kind::Release] {
            fixture::with_graph(case, kind, |p, f, refs, runtime| {
                check::function(p, f, kind, refs, runtime).expect("pristine genuine recipe");
                let mut witnessed = 0;
                let instructions =
                    f.layout.blocks().flat_map(|b| f.layout.block_insts(b)).collect::<Vec<_>>();
                for inst in instructions {
                    let original = f.dfg.insts[inst];
                    let mut wrong = original;
                    match &mut wrong {
                        D::Load { offset, .. } | D::Store { offset, .. } => {
                            *offset = 123_456.into();
                        }
                        D::IntCompare { cond, .. } => {
                            *cond = cond.complement();
                        }
                        D::UnaryImm { opcode: O::Iconst, imm } => {
                            *imm = (i64::from(*imm) ^ 1).into();
                        }
                        _ => continue,
                    }
                    f.dfg.insts[inst] = wrong;
                    assert!(
                        check::function(p, f, kind, refs, runtime).is_err(),
                        "{case} {kind:?} rejects actual altered {inst:?}"
                    );
                    f.dfg.insts[inst] = original;
                    if let D::Store { .. } = original {
                        let mut truncated = original;
                        if let D::Store { opcode, .. } = &mut truncated {
                            *opcode = O::Istore8;
                        }
                        f.dfg.insts[inst] = truncated;
                        assert!(
                            check::function(p, f, kind, refs, runtime).is_err(),
                            "{case} {kind:?} rejects truncating actual store {inst:?}"
                        );
                        f.dfg.insts[inst] = original;
                    }
                    witnessed += 1;
                }
                assert!(witnessed > 0, "actual covered primitive operands");
                check::function(p, f, kind, refs, runtime).expect("pristine retry");
            });
        }
    }
}
