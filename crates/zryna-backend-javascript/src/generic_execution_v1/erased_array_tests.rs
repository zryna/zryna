//! Genuine erased-parent array refills preserve type extent without initializing missing siblings.
mod run;
mod source;
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, raw};
const POSITIVE: &str = "erased-parent-fixed-array-bounds";
#[test]
fn whole_source_seal_retains_real_root_move_element_refills_and_direct_dynamic_array_read() {
    for provider in source::PROVIDERS {
        source::with_program(POSITIVE, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let function = program
                .functions()
                .find(|function| {
                    function.blocks().any(|block| {
                        block
                            .operations()
                            .any(|op| matches!(op.kind(), raw::Operation::IndexCopy { .. }))
                    })
                })
                .expect("actual indexed source function");
            let operation = function
                .blocks()
                .flat_map(VerifiedBlock::operations)
                .find(|op| matches!(op.kind(), raw::Operation::IndexCopy { .. }))
                .expect("actual direct index operation");
            let raw::Operation::IndexCopy { base: raw::PlaceOrLoan::Place(id), .. } =
                operation.kind()
            else {
                panic!("direct Place base, without intervening carrier normalization");
            };
            let array = function.places().nth(*id as usize).expect("same-seal array place");
            let ty = program
                .layouts(zryna_layout::StorageTarget::Linear32V1)
                .types()
                .nth(array.ty() as usize)
                .expect("exact retained array type");
            assert_eq!(ty.category(), zryna_layout::TypeCategory::FixedArray);
            assert_eq!(ty.array_length(), Some(2));
            let root = array.root().id();
            assert!(function.blocks().flat_map(VerifiedBlock::operations).any(|op| matches!(op.kind(), raw::Operation::ReadPlace { place, mode: raw::ReadMode::Move } if *place == root)), "the original affine parent is really moved/erased");
            let mut filled = Vec::new();
            for op in function.blocks().flat_map(VerifiedBlock::operations) {
                if let raw::Operation::InitializePlace { place, .. } = op.kind() {
                    let target =
                        function.places().nth(*place as usize).expect("same-owner refill place");
                    if let raw::PlaceKind::FixedArrayConstant { base, index } = target.kind() {
                        let parent = function.places().nth(base as usize).expect("actual parent");
                        if parent.root().id() == root && parent.ty() == array.ty() {
                            filled.push(index);
                        }
                    }
                }
            }
            assert_eq!(filled, [0, 1], "individual Copy children restore the absent array");
        });
    }
}
#[test]
fn physical_sparse_wrapper_cannot_authorize_reading_an_unfilled_copy_sibling() {
    let case = "partly-refilled-array-read";
    let row = source::row(case);
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        let offset = |key: &str| {
            u32::try_from(row["primary"][key].as_u64().expect("pre-provider offset"))
                .expect("bounded source")
        };
        let span = sources
            .verify_span(zryna_source::UntrustedSpan {
                file: 0,
                start: offset("start"),
                end: offset("end"),
            })
            .expect("exact original base span");
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
                .expect_err("missing element remains unavailable")
        else {
            panic!("owning original diagnostic, not an internal/target rejection");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), row["code"].as_str().expect("frozen owning code"));
        assert_eq!(errors[0].primary_span(), Some(span));
    }
}
#[test]
fn ordinary_erased_array_reads_have_exact_extent_and_pristine_bounds_retry() {
    run::execute(false);
}
#[test]
fn observed_erased_array_bounds_preserve_exact_cleanup_and_fault_retry() {
    run::execute(true);
}
