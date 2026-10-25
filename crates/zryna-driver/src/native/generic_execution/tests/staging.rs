//! Reject provider replacement, missing bytes and extra lookup directories without executing.

use super::*;
use crate::native::NativeStage;
use std::os::unix::fs::PermissionsExt as _;

#[test]
fn copied_provider_staging_rejects_wrong_missing_replaced_and_extra_entries() {
    with_host(|root, tools| {
        let parent =
            NativeStage::create(root, "generic-provider-controls").expect("ordinary held stage");
        let dependencies =
            dependencies::Dependencies::capture(&parent, tools, NativeProcessLimits::default())
                .expect("actual captured production GNU provider closure");
        for mutation in ["wrong", "missing", "replaced", "extra"] {
            let providers = provider_stage::ProviderStage::create(&parent, &dependencies)
                .expect("exact two private provider copies");
            providers.revalidate(&dependencies).expect("pristine staged identity");
            let loader = providers.loader();
            match mutation {
                "wrong" => {
                    fs::write(&loader, b"hostile replacement bytes").expect("owned fault control");
                }
                "missing" => fs::remove_file(&loader).expect("owned missing-file control"),
                "replaced" => {
                    let bytes = fs::read(&loader).expect("original exact copied loader");
                    let replacement = providers.directory().join("replacement");
                    fs::write(&replacement, bytes)
                        .expect("distinct replacement inode before unlink");
                    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o700))
                        .expect("same loader mode");
                    fs::rename(replacement, &loader).expect("same bytes with different inode");
                }
                "extra" => fs::create_dir(providers.directory().join("glibc-hwcaps"))
                    .expect("owned extra lookup-directory control"),
                _ => unreachable!(),
            }
            assert!(providers.revalidate(&dependencies).is_err(), "{mutation}");
            if mutation == "extra" {
                assert!(
                    !providers.cleanup().is_empty(),
                    "unclean provider stage cannot be successful"
                );
                fs::remove_dir(providers.directory().join("glibc-hwcaps"))
                    .expect("remove owned extra directory");
            }
            assert!(providers.cleanup().is_empty(), "known provider entries and directory cleaned");
        }
        assert!(parent.cleanup().is_empty(), "providers removed before parent cleanup");
    });
}
