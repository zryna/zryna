//! Plan-only identity fixture; these declared hashes do not approve real host tools.
use crate::package_build::*;
pub(super) fn configuration() -> BuildPlanConfiguration {
    let target = BuildTargetId::NativeLinuxX86_64;
    BuildPlanConfiguration {
        compiler: BuildCompilerIdentity {
            tool: "zryna".into(),
            version: "0.1.0".into(),
            sha256: "c".repeat(64),
            protocol: "syntax-v4".into(),
        },
        profile: BuildProfileIdentity {
            id: "data-ownership-v1".into(),
            configuration_sha256: "d".repeat(64),
        },
        execution_policy: ExecutionPolicyIdentity {
            id: "native-recipe-policy".into(),
            version: "0".into(),
            configuration_sha256: "e".repeat(64),
        },
        host: BuildHostIdentity {
            triple: "x86_64-unknown-linux-gnu".into(),
            environment: [
                ("HOME", "/nonexistent"),
                ("LANG", "C"),
                ("LC_ALL", "C"),
                ("LD_LIBRARY_PATH", "/lib/x86_64-linux-gnu"),
                ("PATH", "/usr/bin"),
                ("SOURCE_DATE_EPOCH", "0"),
                ("TMPDIR", "/tmp"),
                ("TZ", "UTC"),
            ]
            .into_iter()
            .map(|(name, value)| BuildEnvironmentEntry { name: name.into(), value: value.into() })
            .collect(),
        },
        targets: vec![BuildTarget {
            id: target.clone(),
            triple: "x86_64-unknown-linux-gnu".into(),
            abi: "zryna-native-scalar-v1".into(),
            features: vec![],
            runtime: BuildRuntimeIdentity {
                name: "zryna-native-runtime".into(),
                version: "1".into(),
                sha256: "f".repeat(64),
            },
            composition: BuildCompositionIdentity {
                contract: "zryna.cross-target-profiles.v1".into(),
                row: "NATIVE-HOST".into(),
                host_policy_sha256: "1".repeat(64),
                approved_request_sha256: "2".repeat(64),
            },
        }],
        host_tools: vec![
            BuildHostToolIdentity {
                name: "gcc".into(),
                version: "14".into(),
                sha256: "3".repeat(64),
                runs_on: "x86_64-unknown-linux-gnu".into(),
                targets: vec![target.clone()],
            },
            BuildHostToolIdentity {
                name: "zryna".into(),
                version: "0.1.0".into(),
                sha256: "c".repeat(64),
                runs_on: "x86_64-unknown-linux-gnu".into(),
                targets: vec![target.clone()],
            },
        ],
        outputs: vec![BuildOutput { path: "native/adapter.o".into(), target }],
    }
}
