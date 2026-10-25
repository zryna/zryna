//! Isolated sealed-layout accounting: does not claim a source/IR execution seal.
mod layout_fixture;
const CAP: usize = 33_554_432;

#[test]
fn copy_enum_member_walk_counts_all_shared_struct_edges_before_width_work() {
    assert_eq!(crate::generic_owned_v2::MAX_BYTES, CAP);
    let layouts = layout_fixture::copy_enum();
    let all = layouts.types().collect::<Vec<_>>();
    assert!(
        all.iter()
            .all(|ty| ty.drop_kind() == 0 || ty.category() == zryna_layout::TypeCategory::String)
    );
    let leaf = all
        .iter()
        .find(|ty| ty.category() == zryna_layout::TypeCategory::Struct)
        .expect("Copy Struct child");
    let choice = all
        .iter()
        .find(|ty| ty.category() == zryna_layout::TypeCategory::Enum)
        .expect("Copy enum root");
    assert_eq!(leaf.fields().len(), 1);
    assert_eq!(choice.variants().len(), 8);
    assert!(choice.variants().all(|(_, payload)| payload == Some(leaf.id())));
    let mut spent = 0;
    super::precharge_width_members(&layouts, &mut spent).expect("Copy member census");
    assert_eq!(spent, 9, "one Struct field plus eight enum edges, including shared-child hits");
}

#[test]
fn copy_enum_member_exact_first_extra_overflow_and_retry_use_real_cap() {
    let layouts = layout_fixture::copy_enum();
    let mut exact = 33_554_423;
    super::precharge_width_members(&layouts, &mut exact).expect("exact real cap");
    assert_eq!(exact, CAP);
    for prefix in [33_554_424, usize::MAX] {
        let mut spent = prefix;
        let error = super::precharge_width_members(&layouts, &mut spent)
            .expect_err("first extra or arithmetic overflow");
        assert_eq!(error.code, "ZRYNA-W4001");
        assert_eq!(spent, prefix, "failed census rolls back caller credit");
    }
    let mut retry = 0;
    super::precharge_width_members(&layouts, &mut retry).expect("pristine retry");
    assert_eq!(retry, 9);
}
