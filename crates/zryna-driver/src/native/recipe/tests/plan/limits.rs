use super::{plan, support};
use std::collections::BTreeMap;

#[test]
fn actual_package_c_file_accepts1024_bytes_and_rejects1025_without_a_cap_increase() {
    let mut exact = support::C_SOURCE.to_vec();
    exact.resize(1024, b'\n');
    let f = support::Fixture::with_c_source(&exact);
    assert_eq!(f.plan().source_bytes().len(), 1024);
    let pristine = f.snapshot();
    exact.push(b'\n');
    f.rewrite_source(&exact, support::SOURCE.as_bytes());
    let before = f.snapshot();
    assert_eq!(
        f.frozen().expect_err("1025 actual source bytes are outside package contract").code(),
        "ZRYNA-P4004"
    );
    assert_eq!(f.snapshot(), before);
    exact.pop();
    f.rewrite_source(&exact, support::SOURCE.as_bytes());
    let recovery = f.frozen().expect("fresh pristine source resolves after first-extra rejection");
    plan::prepare(f.request_for(&recovery))
        .expect("original FFI issuer and pristine source recover");
    assert_eq!(f.snapshot(), pristine);
}

#[test]
fn readonly_sysroot_inventory_bounds_paths_and_prefixes_reject_then_recover() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    for path in ["../escape.h", "/absolute.h", "bad\\name.h", "Upper.h", "x/../bad.h", "con.h"] {
        let mut request = f.request();
        request.step.sysroots[0].files = BTreeMap::from([(path.into(), vec![0])]);
        assert_eq!(
            plan::prepare(request).expect_err("unsafe data reference").reason(),
            "TRUST-NATIVE"
        );
        f.plan();
    }
    let mut request = f.request();
    request.step.sysroots[0].files =
        BTreeMap::from([("a".into(), vec![0]), ("a/b.h".into(), vec![0])]);
    assert!(plan::prepare(request).is_err());
    let mut request = f.request();
    request.step.sysroots[0].files = (0..64).map(|n| (format!("h{n:03}.h"), vec![0])).collect();
    plan::prepare(request).expect("64 readonly files");
    let mut request = f.request();
    request.step.sysroots[0].files = (0..65).map(|n| (format!("h{n:03}.h"), vec![0])).collect();
    assert!(plan::prepare(request).is_err());
    let mut request = f.request();
    request.step.sysroots = (0..8)
        .map(|n| plan::ReadOnlyMaterial {
            id: format!("root{n}"),
            files: BTreeMap::from([("h.h".into(), vec![0])]),
        })
        .collect();
    plan::prepare(request).expect("eight sysroot identities");
    let mut request = f.request();
    request.step.sysroots = (0..9)
        .map(|n| plan::ReadOnlyMaterial {
            id: format!("root{n}"),
            files: BTreeMap::from([("h.h".into(), vec![0])]),
        })
        .collect();
    assert!(plan::prepare(request).is_err());
    assert_eq!(f.snapshot(), before);
}

#[test]
fn readonly_aggregate_bytes_accept32mib_and_reject_first_extra_before_host_use() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    let mut request = f.request();
    request.step.sysroots[0].files =
        BTreeMap::from([("header.h".into(), vec![0; 32 * 1024 * 1024])]);
    plan::prepare(request).expect("exact retained readonly byte budget");
    let mut request = f.request();
    request.step.sysroots[0].files =
        BTreeMap::from([("header.h".into(), vec![0; 32 * 1024 * 1024 + 1])]);
    assert_eq!(
        plan::prepare(request).expect_err("first extra readonly byte").reason(),
        "TRUST-NATIVE"
    );
    f.plan();
    assert_eq!(f.snapshot(), before);
}

#[test]
fn approved_dependency_inventory_accepts64_and_rejects65_before_any_host_session() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    let mut operator = f.approval();
    operator.dependency_symbols = (0..64).map(|index| format!("dependency_{index:03}")).collect();
    super::approval::admit(f.plan(), zryna_package::SourceTrustMode::TrustedNative, operator)
        .expect("64 approved dependency names remain subject to full later output audit");
    let mut operator = f.approval();
    operator.dependency_symbols = (0..65).map(|index| format!("dependency_{index:03}")).collect();
    assert_eq!(
        super::approval::admit(f.plan(), zryna_package::SourceTrustMode::TrustedNative, operator)
            .expect_err("first extra dependency")
            .reason(),
        "TRUST-NATIVE"
    );
    assert_eq!(f.snapshot(), before);
}
