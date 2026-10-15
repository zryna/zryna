//! Actual executable and retained runtime identities, without a native profile activation.
use sha2::{Digest as _, Sha256};
use std::io::Read as _;
use zryna_driver::{
    BuildCompilerIdentity, BuildCompositionIdentity, BuildEnvironmentEntry, BuildHostIdentity,
    BuildHostToolIdentity, BuildOutput, BuildPlanConfiguration, BuildProfileIdentity,
    BuildRuntimeIdentity, BuildTarget, BuildTargetId, NativeRecipeRequirements,
    NativeToolRootApproval, native_recipe_environment, native_recipe_policy_identity,
    native_tool_root_identity,
};

pub(super) fn build(
    root: &NativeToolRootApproval,
    gcc_version: &str,
    requirements: &NativeRecipeRequirements,
) -> BuildPlanConfiguration {
    let executable = std::env::current_exe().expect("actual conformance compiler executable");
    let file = std::fs::File::open(executable).expect("actual executable bytes");
    let metadata = file.metadata().expect("actual executable metadata");
    assert!(metadata.is_file() && metadata.len() > 0 && metadata.len() <= 512 * 1024 * 1024);
    let mut compiler_sha = Sha256::new();
    let mut reader = file.take(512 * 1024 * 1024 + 1);
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer).expect("actual compiler digest");
        if count == 0 {
            break;
        }
        compiler_sha.update(&buffer[..count]);
    }
    assert_eq!(reader.limit(), 512 * 1024 * 1024 + 1 - metadata.len());
    let compiler_sha = format!("{:x}", compiler_sha.finalize());
    let target = BuildTargetId::NativeLinuxX86_64;
    let mut host_tools: Vec<_> = root
        .tools
        .iter()
        .map(|(path, sha256)| {
            let name = path.rsplit('/').next().expect("approved tool basename");
            BuildHostToolIdentity {
                name: name.into(),
                version: if matches!(name, "gcc" | "cc1") {
                    gcc_version.into()
                } else {
                    "sha256-pinned-root-v0".into()
                },
                sha256: sha256.clone(),
                runs_on: "x86_64-unknown-linux-gnu".into(),
                targets: vec![target.clone()],
            }
        })
        .collect();
    host_tools.push(BuildHostToolIdentity {
        name: "zryna-conformance".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        sha256: compiler_sha.clone(),
        runs_on: "x86_64-unknown-linux-gnu".into(),
        targets: vec![target.clone()],
    });
    host_tools.sort_by(|a, b| a.name.cmp(&b.name));
    let mut environment: Vec<_> = native_recipe_environment()
        .into_iter()
        .map(|entry| {
            let (name, value) = entry.split_once('=').expect("closed environment");
            BuildEnvironmentEntry { name: name.into(), value: value.into() }
        })
        .collect();
    environment.sort_by(|a, b| a.name.cmp(&b.name));
    let policy = native_recipe_policy_identity();
    let runtime_sha256 =
        requirements.private_runtime_source_sha256().expect("actual checked runtime source");
    BuildPlanConfiguration {
        compiler: BuildCompilerIdentity {
            tool: "zryna-conformance".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            sha256: compiler_sha,
            protocol: "native-c-source-v0".into(),
        },
        profile: BuildProfileIdentity {
            id: "data-ownership-v1".into(),
            configuration_sha256: format!(
                "{:x}",
                Sha256::digest(include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../spec/language/DATA_OWNERSHIP_V1.md"
                )))
            ),
        },
        execution_policy: policy.clone(),
        host: BuildHostIdentity { triple: "x86_64-unknown-linux-gnu".into(), environment },
        targets: vec![BuildTarget {
            id: target.clone(),
            triple: "x86_64-unknown-linux-gnu".into(),
            abi: "zryna-native-scalar-v1".into(),
            features: vec![],
            runtime: BuildRuntimeIdentity {
                name: "zryna-native-runtime".into(),
                version: "1".into(),
                sha256: super::hex(runtime_sha256),
            },
            composition: BuildCompositionIdentity {
                contract: "zryna.cross-target-profiles.v1".into(),
                row: "NATIVE-HOST".into(),
                host_policy_sha256: policy.configuration_sha256,
                approved_request_sha256: native_tool_root_identity(root)
                    .expect("operator root identity"),
            },
        }],
        host_tools,
        outputs: vec![BuildOutput { path: "native/adapter.o".into(), target }],
    }
}
