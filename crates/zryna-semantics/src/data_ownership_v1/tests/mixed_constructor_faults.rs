#[path = "mixed_read_faults.rs"]
mod reads;
#[path = "recursive_cleanup_witness.rs"]
mod recursive_witness;
#[path = "mixed_two_element_faults.rs"]
mod two_elements;

use crate::data_ownership_v1::tests::*;

struct ExpectedSite {
    instruction: usize,
    operation: LogicalOperation,
    retained: &'static [u32],
    cleanup: &'static [u32],
}

fn site(
    instruction: usize,
    operation: LogicalOperation,
    retained: &'static [u32],
    cleanup: &'static [u32],
) -> ExpectedSite {
    ExpectedSite { instruction, operation, retained, cleanup }
}

// These are bounded verified-view fault injections, not executions of an allocator.
fn check_sites(
    source: &str,
    snapshot: RawProjectSyntaxSnapshot,
    instruction_count: usize,
    expected: &[ExpectedSite],
) {
    let sources = sources_for(source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated mixed fault source");
    let program = lower(pair_input(&syntax, &sources)).expect("independent mixed full IR");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(instructions.len(), instruction_count);
    let actual_sites = instructions
        .iter()
        .enumerate()
        .filter_map(|(index, instruction)| {
            matches!(
                instruction.kind(),
                VerifiedInstructionKind::StringFromUtf8
                    | VerifiedInstructionKind::VecConstruct
                    | VerifiedInstructionKind::StringClone
                    | VerifiedInstructionKind::StringConcat
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_sites, expected.iter().map(|entry| entry.instruction).collect::<Vec<_>>());
    assert!(!actual_sites.is_empty());
    for entry in expected {
        let instruction = instructions[entry.instruction];
        let mut failures = vec![
            (
                RuntimeStatus::Allocation,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
            ),
            (
                RuntimeStatus::Capacity,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
            ),
        ];
        if entry.operation == LogicalOperation::StringFromUtf8Copy {
            failures.push((
                RuntimeStatus::Utf8,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::Utf8V1),
            ));
        }
        failures.push((RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure));
        assert_all_runtime_faults(
            program.runtime_abi(),
            function,
            instruction,
            entry.operation,
            &failures,
        );
        for (status, disposition) in failures {
            let trace = owned_fault_trace(
                program.runtime_abi(),
                function,
                instruction,
                OwnedFaultInjection::Runtime { operation: entry.operation, status },
                0,
                1,
            )
            .expect("exact admitted mixed failure");
            assert_eq!(trace.block, 0);
            assert_eq!(usize::try_from(trace.instruction).expect("site index"), entry.instruction);
            assert_eq!(trace.disposition, disposition);
            assert!(!trace.result_committed);
            assert_eq!(trace.uncommitted_result, instruction.result());
            assert_eq!(
                trace.retained_roots.iter().map(|root| root.index()).collect::<Vec<_>>(),
                entry.retained
            );
            assert_eq!(
                trace.reverse_cleanup.iter().map(|root| root.index()).collect::<Vec<_>>(),
                entry.cleanup
            );
            assert_eq!(trace.prefix_owner, None);
            assert!(
                trace.reverse_prefix.is_empty(),
                "constructor allocation has no clone-prefix phase"
            );
        }
    }
}

#[test]
fn mixed_struct_vec_faults_cover_every_allocation_site_in_both_directions() {
    for vec_outer in [false, true] {
        let (source, snapshot) = mixed_construction::mixed_fixture(vec_outer);
        let (vector, root) = if vec_outer { (2, 1) } else { (1, 0) };
        let roots: &'static [u32] = if root == 1 { &[1] } else { &[0] };
        check_sites(
            &source,
            snapshot,
            3,
            &[
                site(0, LogicalOperation::StringFromUtf8Copy, &[], &[]),
                site(vector, LogicalOperation::VecAllocate, roots, roots),
            ],
        );
    }
}

#[test]
fn mixed_nested_vec_and_selected_enum_faults_preserve_completed_children() {
    let (source, snapshot) = super::nested_vec_fixture();
    check_sites(
        source.0,
        snapshot,
        4,
        &[
            site(1, LogicalOperation::VecAllocate, &[], &[]),
            site(2, LogicalOperation::VecAllocate, &[], &[0]),
            site(3, LogicalOperation::VecAllocate, &[0, 1], &[1, 0]),
        ],
    );
    let (source, snapshot) = super::nested_enum_fixture();
    check_sites(
        source.0,
        snapshot,
        4,
        &[
            site(0, LogicalOperation::StringFromUtf8Copy, &[], &[]),
            site(1, LogicalOperation::VecAllocate, &[0], &[0]),
            site(3, LogicalOperation::VecAllocate, &[2], &[2]),
        ],
    );
}
