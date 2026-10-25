//! Inspect actual claim only after genuine source/type/state/cleanup issuance.
use zryna_ir::generic_v1::execution_v1::raw;
fn has_indexed(function: &raw::Function, mut id: u32) -> bool {
    for _ in 0..function.loans.len() {
        match function.loans[id as usize].origin {
            raw::LoanOrigin::Indexed { .. } => return true,
            raw::LoanOrigin::InternalReborrow(parent)
            | raw::LoanOrigin::StaticProjection { parent, .. } => id = parent,
            raw::LoanOrigin::Place(_) | raw::LoanOrigin::Formal(_) => return false,
        }
    }
    panic!("sealed ancestry must terminate");
}
pub(super) fn claim(case: &str, p: &raw::Program) {
    let expected_mode = if case == "indexed-option-exclusive-join" {
        raw::MatchMode::ExclusiveLoan
    } else {
        raw::MatchMode::SharedLoan
    };
    let mut matches = 0;
    for function in &p.functions {
        for block in &function.blocks {
            let raw::Terminator::ClosedEnumMatch { scrutinee, mode, arms, .. } = &block.terminator
            else {
                continue;
            };
            assert_eq!(*mode, expected_mode);
            let raw::PlaceOrLoan::Loan(parent) = scrutinee else {
                panic!("borrowed source cannot become owned match")
            };
            assert!(has_indexed(function, *parent));
            let mut join = None;
            for arm in arms {
                let target = &function.blocks[arm.edge.target as usize];
                let raw::Terminator::Jump(edge) = &target.terminator else {
                    panic!("these finite direct arm expressions end at their own result join")
                };
                if let Some(previous) = join {
                    assert_eq!(previous, edge.target);
                } else {
                    join = Some(edge.target);
                }
                assert!(
                    !target.instructions.iter().any(
                        |i| matches!(i.operation,raw::Operation::EndLoan{loan} if loan==*parent)
                    ),
                    "parent remains live until own join"
                );
                match &arm.binding {
                    Some(raw::MatchBinding::Loan(child)) => {
                        assert!(matches!(function.loans[child.id as usize].origin,
                            raw::LoanOrigin::StaticProjection{parent:actual,projection:raw::StaticProjection::EnumPayload(ordinal)} if actual==*parent && ordinal==arm.ordinal));
                        assert!(target.instructions.iter().any(|i|matches!(i.operation,raw::Operation::EndLoan{loan} if loan==child.id)),"selected child ends before own join");
                    }
                    None => {}
                    Some(raw::MatchBinding::Value(_)) => panic!("no fake owned payload binding"),
                }
            }
            let join = &function.blocks[join.expect("actual source join") as usize];
            assert!(
                join.instructions
                    .iter()
                    .any(|i| matches!(i.operation,raw::Operation::EndLoan{loan} if loan==*parent)),
                "inline parent or lexical scope closes after join"
            );
            matches += 1;
        }
    }
    assert_eq!(matches, 1, "each unchanged literal has exactly one borrowed match");
}
