//! The same canonical source-plan projection shared by package builds and native admission.
use super::{BuildPlanConfiguration, PackageBuildError};
use crate::PackageResolutionSuccess;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn source_plan(
    resolution: &PackageResolutionSuccess,
    configuration: &BuildPlanConfiguration,
) -> Result<Value, PackageBuildError> {
    let graph = resolution.graph();
    let identities: BTreeMap<_, _> = graph
        .packages()
        .iter()
        .map(|package| (package.instance().manifest_id(), package.instance().source_sha256()))
        .collect();
    let packages = graph
        .packages()
        .iter()
        .map(|package| {
            let dependencies = package
                .dependencies()
                .iter()
                .map(|(alias, id)| {
                    let source = identities.get(id.as_str()).ok_or_else(|| {
                        PackageBuildError::plan("dependency identity is absent from resolved graph")
                    })?;
                    Ok(json!({
                        "alias": alias,
                        "graphRole": "target/runtime",
                        "package": { "id": id, "sourceSha256": source }
                    }))
                })
                .collect::<Result<Vec<_>, PackageBuildError>>()?;
            Ok(json!({
                "dependencies": dependencies,
                "graphRole": "target/runtime",
                "package": {
                    "id": package.instance().manifest_id(),
                    "sourceSha256": package.instance().source_sha256()
                }
            }))
        })
        .collect::<Result<Vec<_>, PackageBuildError>>()?;
    let sources = resolution
        .sources()
        .iter()
        .flat_map(|package| {
            package.files().iter().map(move |file| {
                json!({
                    "graphRole": "target/runtime",
                    "package": {
                        "id": package.package_id(),
                        "sourceSha256": package.source_sha256()
                    },
                    "path": file.path(),
                    "sha256": file.sha256(),
                    "size": file.bytes().len()
                })
            })
        })
        .collect::<Vec<_>>();
    let root = graph
        .packages()
        .iter()
        .find(|package| package.instance().manifest_id() == graph.root())
        .ok_or_else(|| PackageBuildError::plan("resolved root package is absent"))?;
    Ok(json!({
        "compiler": configuration.compiler,
        "executionPolicy": configuration.execution_policy,
        "host": configuration.host,
        "hostTools": configuration.host_tools,
        "outputs": configuration.outputs,
        "packageLockSha256": graph.lock_sha256(),
        "packages": packages,
        "profile": configuration.profile,
        "rootPackage": {
            "id": root.instance().manifest_id(),
            "sourceSha256": root.instance().source_sha256()
        },
        "sources": sources,
        "targets": configuration.targets
    }))
}
