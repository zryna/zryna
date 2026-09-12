use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::{BuildPlanConfiguration, PackageBuildError, PackageBuildRequest};

mod validation;
use validation::validate_configuration;

const PLAN_DOMAIN: &[u8] = b"ZRYNA-RESOLVED-BUILD-PLAN-V0\0plan\0";
const TARGET_DOMAIN: &[u8] = b"ZRYNA-RESOLVED-BUILD-PLAN-V0\0target\0";
const SCHEMA: &str = "../../schemas/zryna-resolved-build-plan-v0.schema.json";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
/// Closed resolved-plan target vocabulary.
pub enum BuildTargetId {
    /// ECMAScript module output.
    #[serde(rename = "javascript")]
    JavaScript,
    /// Linux x86-64 native output.
    #[serde(rename = "native-linux-x86_64")]
    NativeLinuxX86_64,
    /// Core WebAssembly output.
    #[serde(rename = "webassembly")]
    WebAssembly,
}

impl BuildTargetId {
    #[must_use]
    /// Returns the canonical resolved-plan spelling.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::JavaScript => "javascript",
            Self::NativeLinuxX86_64 => "native-linux-x86_64",
            Self::WebAssembly => "webassembly",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Exact compiler identity bound into the plan.
pub struct BuildCompilerIdentity {
    /// Declared host-tool name.
    pub tool: String,
    /// Exact compiler version.
    pub version: String,
    /// Raw compiler executable SHA-256 digest.
    pub sha256: String,
    /// Exact frontend protocol identity.
    pub protocol: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Exact language-profile identity bound into the plan.
pub struct BuildProfileIdentity {
    /// Exact language profile.
    pub id: String,
    /// Exact output-relevant profile configuration digest.
    pub configuration_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Opaque execution-policy identity bound into the plan.
pub struct ExecutionPolicyIdentity {
    /// Opaque policy identity.
    pub id: String,
    /// Exact policy version.
    pub version: String,
    /// Exact output-relevant policy configuration digest.
    pub configuration_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// One output-relevant environment entry.
pub struct BuildEnvironmentEntry {
    /// Declared environment name.
    pub name: String,
    /// Exact output-relevant value.
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// Exact host and declared environment identity.
pub struct BuildHostIdentity {
    /// Exact compiler host triple.
    pub triple: String,
    /// Complete sorted output-relevant environment.
    pub environment: Vec<BuildEnvironmentEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// Exact target runtime identity.
pub struct BuildRuntimeIdentity {
    /// Exact runtime name.
    pub name: String,
    /// Exact runtime version.
    pub version: String,
    /// Raw runtime material SHA-256 digest.
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Exact cross-target composition identity and grants.
pub struct BuildCompositionIdentity {
    /// Accepted cross-target composition contract.
    pub contract: String,
    /// Compatible composition row.
    pub row: String,
    /// Exact host-policy digest.
    pub host_policy_sha256: String,
    /// Exact approved-request digest.
    pub approved_request_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// One exact selected target and its ABI/runtime composition.
pub struct BuildTarget {
    /// Resolved target id.
    pub id: BuildTargetId,
    /// Exact target triple.
    pub triple: String,
    /// Exact ABI identity.
    pub abi: String,
    /// Closed source-only feature set, currently empty.
    pub features: Vec<String>,
    /// Exact target runtime identity.
    pub runtime: BuildRuntimeIdentity,
    /// Exact accepted composition identity.
    pub composition: BuildCompositionIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// One independently declared host tool.
pub struct BuildHostToolIdentity {
    /// Declared tool name.
    pub name: String,
    /// Exact tool version.
    pub version: String,
    /// Raw executable SHA-256 digest.
    pub sha256: String,
    /// Exact host triple on which the tool runs.
    pub runs_on: String,
    /// Sorted targets admitted by the tool.
    pub targets: Vec<BuildTargetId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// One declared target-qualified final output.
pub struct BuildOutput {
    /// Exact target-qualified portable output path.
    pub path: String,
    /// Target that owns the output.
    pub target: BuildTargetId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical resolved plan bytes and their separate plan/target cache identities.
pub struct BuildPlanIdentity {
    bytes: Vec<u8>,
    cache_key: String,
    target_cache_keys: BTreeMap<BuildTargetId, String>,
}

impl BuildPlanIdentity {
    #[must_use]
    /// Returns exact canonical newline-terminated plan bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    /// Returns the domain-separated complete plan cache key.
    pub fn cache_key(&self) -> &str {
        &self.cache_key
    }

    #[must_use]
    /// Returns the domain-separated cache key for one selected target.
    pub fn target_cache_key(&self, target: &BuildTargetId) -> Option<&str> {
        self.target_cache_keys.get(target).map(String::as_str)
    }
}

pub(super) struct PreparedPlan {
    pub identity: BuildPlanIdentity,
    pub package_order: Vec<String>,
}

pub(super) fn prepare(
    request: &PackageBuildRequest<'_>,
) -> Result<PreparedPlan, PackageBuildError> {
    if request.mode == super::PackageBuildMode::Frozen && request.resolution.published() {
        return Err(PackageBuildError::plan(
            "frozen build requires an exact pre-existing authenticated lock",
        ));
    }
    validate_configuration(request.resolution, &request.configuration)?;
    let source_plan = source_plan(request)?;
    let mut projection = json!({
        "$schema": SCHEMA,
        "format": "zryna.resolved-build-plan.v0",
        "sourcePlan": source_plan,
        "status": "specified-only",
        "version": 0
    });
    let projection_bytes = canonical(&projection)?;
    let cache_key = domain_hash(PLAN_DOMAIN, &projection_bytes);
    projection
        .as_object_mut()
        .ok_or_else(|| PackageBuildError::plan("plan projection is not an object"))?
        .insert("cacheKey".to_owned(), Value::String(cache_key.clone()));
    let bytes = canonical(&projection)?;
    if bytes.len() > 262_144 {
        return Err(PackageBuildError::plan("canonical build plan exceeds 262144 bytes"));
    }
    let target_cache_keys = request
        .configuration
        .targets
        .iter()
        .map(|target| {
            let bytes = canonical(&json!({ "planKey": cache_key, "target": target.id }))?;
            Ok((target.id.clone(), domain_hash(TARGET_DOMAIN, &bytes)))
        })
        .collect::<Result<_, PackageBuildError>>()?;
    Ok(PreparedPlan {
        identity: BuildPlanIdentity { bytes, cache_key, target_cache_keys },
        package_order: dependency_order(request.resolution.graph())?,
    })
}

fn source_plan(request: &PackageBuildRequest<'_>) -> Result<Value, PackageBuildError> {
    let graph = request.resolution.graph();
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
    let sources = request
        .resolution
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
    let configuration = &request.configuration;
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

fn dependency_order(
    graph: &zryna_package::ResolvedGraph,
) -> Result<Vec<String>, PackageBuildError> {
    fn visit(
        id: &str,
        packages: &BTreeMap<&str, &zryna_package::ResolvedPackage>,
        visited: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), PackageBuildError> {
        if visited.contains(id) {
            return Ok(());
        }
        let package = packages
            .get(id)
            .ok_or_else(|| PackageBuildError::plan("dependency is absent from resolved graph"))?;
        for (_, dependency) in package.dependencies() {
            visit(dependency, packages, visited, order)?;
        }
        visited.insert(id.to_owned());
        order.push(id.to_owned());
        Ok(())
    }

    let packages: BTreeMap<_, _> = graph
        .packages()
        .iter()
        .map(|package| (package.instance().manifest_id(), package))
        .collect();
    let mut visited = BTreeSet::new();
    let mut order = Vec::with_capacity(packages.len());
    visit(graph.root(), &packages, &mut visited, &mut order)?;
    if order.len() != packages.len() {
        return Err(PackageBuildError::plan("resolved graph contains an unreachable package"));
    }
    Ok(order)
}

fn canonical(value: &Value) -> Result<Vec<u8>, PackageBuildError> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|_| PackageBuildError::plan("canonical plan serialization failed"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn domain_hash(domain: &[u8], bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
