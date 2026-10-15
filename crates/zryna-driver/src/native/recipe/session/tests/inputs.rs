use sha2::{Digest as _, Sha256};

use super::super::super::archive::RootKind;
use super::super::{inputs, toolroot};
use super::support;

#[test]
fn original_header_inventory_and_all_five_tools_bind_before_host_effects() {
    let root = support::root();
    let fixture = support::fixture(&root, None);
    let before = fixture.snapshot();
    let approved = support::admit(support::plan(&fixture), &root);
    let original = inputs::prepare(&approved, &root).expect("effect-free positive");
    assert_eq!(original.len(), 3);
    assert_eq!(original["library.c"], approved.plan().source_bytes());
    for name in root.tools.keys() {
        let mut changed = root.clone();
        let sha: [u8; 32] = Sha256::digest(b"u").into();
        changed.tools.insert(name.clone(), toolroot::hex(&sha));
        changed.entries.get_mut(name).expect("tool").kind =
            RootKind::File { bytes: 1, sha256: sha };
        let independently_rebound = support::admit(support::plan(&fixture), &changed);
        assert_eq!(
            inputs::prepare(&independently_rebound, &changed)
                .expect_err("tool absent from original host plan")
                .detail(),
            "exact compiler and complete environment authority required"
        );
    }
    assert_eq!(inputs::prepare(&approved, &root).expect("pristine recovery"), original);
    assert_eq!(fixture.snapshot(), before);
}

#[test]
fn independently_reapproved_wrong_header_missing_and_extra_files_are_refused() {
    let root = support::root();
    let fixture = support::fixture(&root, None);
    let before = fixture.snapshot();
    for mutation in 0..3 {
        let mut request = support::plan(&fixture);
        match mutation {
            0 => request.step.sysroots[0].files.get_mut("library.h").expect("header").push(b'\n'),
            1 => {
                request.step.sysroots[0].files.remove("library.h");
            }
            _ => {
                request.step.sysroots[0].files.insert("unexpected.h".into(), b"x".to_vec());
            }
        }
        let approved = support::admit(request, &root);
        let error = inputs::prepare(&approved, &root)
            .expect_err("reapproval cannot replace original FFI header/inventory");
        assert_eq!(
            error.detail(),
            match mutation {
                0 => "actual compile header differs from original FFI issuer",
                1 => "complete fixed-layout input inventory exceeds volume bound",
                _ => "only exact declared fixed-layout readonly headers are supported",
            }
        );
    }
    let approved = support::admit(support::plan(&fixture), &root);
    inputs::prepare(&approved, &root).expect("pristine recovers");
    assert_eq!(fixture.snapshot(), before);
}

#[test]
fn reapproved_environment_and_policy_changes_still_refuse_fixed_mechanism() {
    let root = support::root();
    for mutate_policy in [false, true] {
        let mut fixture = support::fixture(&root, None);
        let before = fixture.snapshot();
        if mutate_policy {
            fixture.configuration.execution_policy.configuration_sha256 = "0".repeat(64);
        } else {
            fixture
                .configuration
                .host
                .environment
                .iter_mut()
                .find(|entry| entry.name == "HOME")
                .expect("HOME")
                .value = "/different".into();
        }
        let approved = support::admit(support::plan(&fixture), &root);
        assert!(inputs::prepare(&approved, &root).is_err());
        assert_eq!(fixture.snapshot(), before);
    }
}

#[test]
fn image_root_and_genuine_source_approval_transplants_are_refused() {
    let root = support::root();
    let fixture = support::fixture(&root, None);
    let approved = support::admit(support::plan(&fixture), &root);
    let before = fixture.snapshot();
    let mut image = root.clone();
    image.image_sha256 = "c".repeat(64);
    assert!(inputs::prepare(&approved, &image).is_err());
    let mut daemon = root.clone();
    daemon.engine_uid = 0;
    assert!(inputs::prepare(&approved, &daemon).is_err());
    let other = support::fixture(&root, Some(b"int different_source;\n"));
    let other_plan = crate::native::recipe::plan::prepare(support::plan(&other))
        .expect("second genuine source plan");
    assert!(
        crate::native::recipe::approval::admit(
            other_plan,
            zryna_package::SourceTrustMode::TrustedNative,
            approved.operator().clone()
        )
        .is_err()
    );
    inputs::prepare(&approved, &root).expect("original source and image recover");
    assert_eq!(fixture.snapshot(), before);
}
