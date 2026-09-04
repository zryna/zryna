use super::{check_sites, site};
use crate::data_ownership_v1::tests::{LogicalOperation, mixed_string_read_scopes};
use mixed_string_read_scopes::{ReadCase, read_fixture};

#[test]
fn mixed_read_faults_keep_prior_local_and_unconsumed_literal_owners() {
    for case in [ReadCase::LiteralClone, ReadCase::LocalConcat] {
        let (source, snapshot) = read_fixture(case);
        let (operation, retained): (_, &'static [u32]) = match case {
            ReadCase::LiteralClone => (LogicalOperation::StringClone, &[2]),
            ReadCase::LocalConcat => (LogicalOperation::StringConcat, &[1, 2]),
            ReadCase::NestedConcat => unreachable!("separate nested source case"),
        };
        check_sites(
            &source,
            snapshot,
            6,
            &[
                site(0, LogicalOperation::StringFromUtf8Copy, &[], &[]),
                site(2, LogicalOperation::StringFromUtf8Copy, &[], &[1]),
                site(3, operation, retained, &[2, 1]),
                site(5, LogicalOperation::VecAllocate, &[4], &[4, 2, 1]),
            ],
        );
    }
}

#[test]
fn mixed_nested_read_faults_keep_intermediate_owners_before_outer_vec_commit() {
    let (source, snapshot) = read_fixture(ReadCase::NestedConcat);
    check_sites(
        &source,
        snapshot,
        7,
        &[
            site(0, LogicalOperation::StringFromUtf8Copy, &[], &[]),
            site(2, LogicalOperation::StringClone, &[1], &[1]),
            site(3, LogicalOperation::StringFromUtf8Copy, &[], &[2, 1]),
            site(4, LogicalOperation::StringConcat, &[2, 3], &[3, 2, 1]),
            site(6, LogicalOperation::VecAllocate, &[5], &[5, 3, 2, 1]),
        ],
    );
}
