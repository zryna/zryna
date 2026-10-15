use std::collections::BTreeMap;

use sha2::{Digest as _, Sha256};
use zryna_package::SourceTrustMode;

use crate::native::recipe::archive::{Metadata, RootEntry, RootKind};
use crate::native::recipe::tests::support::Fixture;
use crate::native::recipe::{approval, plan};
use crate::package_build::{BuildEnvironmentEntry, BuildHostToolIdentity, BuildTargetId};

use super::super::{config, toolroot};

pub(super) fn root() -> toolroot::NativeToolRootApproval {
    // Synthetic one-byte tool identities test data validation only, never executable tools.
    let sha: [u8; 32] = Sha256::digest(b"t").into();
    let mut entries = BTreeMap::new();
    for name in
        ["", "/lib64", "/tool", "/tool/bin", "/tool/libexec", "/input", "/output", "/control"]
    {
        let (uid, gid, mode) = match name {
            "/input" | "/control" => (0, 0, 0o555),
            "/output" => (65534, 65534, 0o700),
            _ => (0, 0, 0o755),
        };
        entries.insert(
            name.to_owned(),
            RootEntry { metadata: Metadata { uid, gid, mode }, kind: RootKind::Directory },
        );
    }
    let tools = [
        "/lib64/ld-linux-x86-64.so.2",
        "/supervisor",
        "/tool/bin/as",
        "/tool/bin/gcc",
        "/tool/libexec/cc1",
    ]
    .into_iter()
    .map(|name| {
        entries.insert(
            name.to_owned(),
            RootEntry {
                metadata: Metadata { uid: 0, gid: 0, mode: 0o555 },
                kind: RootKind::File { bytes: 1, sha256: sha },
            },
        );
        (name.to_owned(), toolroot::hex(&sha))
    })
    .collect();
    toolroot::NativeToolRootApproval {
        engine_uid: 65534,
        image_sha256: "b".repeat(64),
        entries,
        tools,
    }
}

pub(super) fn fixture(root: &toolroot::NativeToolRootApproval, c_source: Option<&[u8]>) -> Fixture {
    let mut fixture = c_source.map_or_else(Fixture::new, Fixture::with_c_source);
    fixture.configuration.execution_policy = config::policy_identity();
    fixture.configuration.targets[0].composition.host_policy_sha256 =
        config::policy_identity().configuration_sha256;
    fixture.configuration.targets[0].composition.approved_request_sha256 =
        toolroot::identity(root).expect("operator data identity only");
    fixture.configuration.host.environment = config::ENV
        .iter()
        .map(|entry| {
            let (name, value) = entry.split_once('=').expect("fixed environment");
            BuildEnvironmentEntry { name: name.into(), value: value.into() }
        })
        .collect();
    fixture.configuration.host.environment.sort_by(|a, b| a.name.cmp(&b.name));
    fixture.configuration.host_tools.retain(|tool| tool.name == "zryna");
    fixture.configuration.host_tools.extend(root.tools.iter().map(|(path, sha)| {
        BuildHostToolIdentity {
            name: path.rsplit('/').next().expect("tool basename").into(),
            version: "unit-data-only".into(),
            sha256: sha.clone(),
            runs_on: "x86_64-unknown-linux-gnu".into(),
            targets: vec![BuildTargetId::NativeLinuxX86_64],
        }
    }));
    fixture.configuration.host_tools.sort_by(|a, b| a.name.cmp(&b.name));
    fixture
}

pub(super) fn plan(fixture: &Fixture) -> plan::CompilePlanRequest<'_> {
    let mut request = fixture.request();
    request.step.sysroots[0]
        .files
        .insert("sqlite3.h".into(), b"/* uncompiled unit header */\n".to_vec());
    request
}

pub(super) fn admit(
    request: plan::CompilePlanRequest<'_>,
    root: &toolroot::NativeToolRootApproval,
) -> approval::ApprovedNativeRecipe {
    let configuration = request.configuration;
    let plan = plan::prepare(request).expect("genuine retained source and FFI plan");
    let operator = approval::OperatorApproval {
        source_plan_key: plan.source_plan().cache_key().into(),
        recipe_sha256: plan.recipe_sha256().into(),
        tool_root_sha256: toolroot::identity(root).expect("valid operator data"),
        image_sha256: root.image_sha256.clone(),
        policy: configuration.execution_policy.clone(),
        approved_tools: configuration.host_tools.clone(),
        output_path: plan.output_path().into(),
        output_bytes: 1,
        output_sha256: Sha256::digest(b"o").into(),
        dependency_symbols: vec![],
    };
    approval::admit(plan, SourceTrustMode::TrustedNative, operator)
        .expect("admission only, never execution")
}
