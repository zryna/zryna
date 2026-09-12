use super::{BuildPlanConfiguration, BuildTarget, BuildTargetId};
use crate::{PackageBuildError, PackageResolutionSuccess};

pub(super) fn validate_configuration(
    resolution: &PackageResolutionSuccess,
    configuration: &BuildPlanConfiguration,
) -> Result<(), PackageBuildError> {
    if configuration.profile.id != resolution.graph().compatibility().profile
        || configuration.compiler.version != resolution.graph().compatibility().compiler
    {
        return Err(PackageBuildError::plan(
            "compiler or profile identity differs from the authenticated package graph",
        ));
    }
    validate_name(&configuration.compiler.tool)?;
    validate_name(&configuration.compiler.protocol)?;
    validate_version(&configuration.compiler.version)?;
    validate_digest(&configuration.compiler.sha256)?;
    validate_digest(&configuration.profile.configuration_sha256)?;
    validate_name(&configuration.execution_policy.id)?;
    validate_version(&configuration.execution_policy.version)?;
    validate_digest(&configuration.execution_policy.configuration_sha256)?;
    validate_triple(&configuration.host.triple)?;
    ordered_unique(&configuration.host.environment, |entry| &entry.name, 8, "environment")?;
    for entry in &configuration.host.environment {
        if entry.name.is_empty()
            || entry.name.len() > 64
            || !entry.name.as_bytes()[0].is_ascii_uppercase()
            || !entry
                .name
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
            || entry.value.chars().count() > 160
        {
            return Err(PackageBuildError::plan(
                "environment entry name or value is outside its closed bounds",
            ));
        }
    }
    let environment = configuration
        .host
        .environment
        .iter()
        .map(|entry| (entry.name.as_str(), entry.value.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    if environment.get("LC_ALL") != Some(&"C")
        || environment.get("TZ") != Some(&"UTC")
        || !environment.get("SOURCE_DATE_EPOCH").is_some_and(|value| valid_epoch(value))
    {
        return Err(PackageBuildError::plan(
            "reproduction environment must declare LC_ALL, TZ, and SOURCE_DATE_EPOCH",
        ));
    }
    validate_targets(resolution, configuration)?;
    validate_tools(configuration)?;
    validate_outputs(configuration)
}

fn validate_targets(
    resolution: &PackageResolutionSuccess,
    configuration: &BuildPlanConfiguration,
) -> Result<(), PackageBuildError> {
    if configuration.targets.is_empty() || configuration.targets.len() > 3 {
        return Err(PackageBuildError::plan("target count is outside 1..=3"));
    }
    ordered_unique(&configuration.targets, |target| target.id.as_str(), 3, "targets")?;
    for target in &configuration.targets {
        if !resolution.graph().compatibility().targets.iter().any(|id| id == target.id.as_str()) {
            return Err(PackageBuildError::plan("target is not covered by the package graph"));
        }
        validate_target(target)?;
    }
    Ok(())
}

fn validate_tools(configuration: &BuildPlanConfiguration) -> Result<(), PackageBuildError> {
    ordered_unique(&configuration.host_tools, |tool| &tool.name, 8, "host tools")?;
    for tool in &configuration.host_tools {
        validate_name(&tool.name)?;
        validate_version(&tool.version)?;
        validate_digest(&tool.sha256)?;
        validate_triple(&tool.runs_on)?;
        if tool.targets.is_empty()
            || tool.targets.len() > 3
            || tool.targets.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(PackageBuildError::plan(
                "host-tool targets are not bounded, sorted, and unique",
            ));
        }
        if tool
            .targets
            .iter()
            .any(|target| !configuration.targets.iter().any(|item| &item.id == target))
        {
            return Err(PackageBuildError::plan("host tool names a target absent from the plan"));
        }
    }
    let compiler = configuration
        .host_tools
        .iter()
        .find(|tool| tool.name == configuration.compiler.tool)
        .ok_or_else(|| PackageBuildError::plan("compiler is absent from declared host tools"))?;
    if compiler.version != configuration.compiler.version
        || compiler.sha256 != configuration.compiler.sha256
        || compiler.runs_on != configuration.host.triple
        || configuration.targets.iter().any(|target| !compiler.targets.contains(&target.id))
    {
        return Err(PackageBuildError::plan(
            "compiler host-tool identity or target coverage differs",
        ));
    }
    Ok(())
}

fn validate_outputs(configuration: &BuildPlanConfiguration) -> Result<(), PackageBuildError> {
    if configuration.outputs.is_empty() || configuration.outputs.len() > 16 {
        return Err(PackageBuildError::plan("output count is outside 1..=16"));
    }
    for pair in configuration.outputs.windows(2) {
        if (&pair[0].target, &pair[0].path) >= (&pair[1].target, &pair[1].path) {
            return Err(PackageBuildError::plan("outputs must be sorted and unique"));
        }
    }
    let mut paths = Vec::<&str>::with_capacity(configuration.outputs.len());
    for output in &configuration.outputs {
        validate_path(&output.path, 160)?;
        if reserved_output_path(&output.path)
            || paths.iter().any(|prior| paths_conflict(prior, &output.path))
        {
            return Err(PackageBuildError::plan(
                "output path is reserved, duplicated, or conflicts with another output",
            ));
        }
        paths.push(output.path.as_str());
        if !configuration.targets.iter().any(|target| target.id == output.target) {
            return Err(PackageBuildError::plan("output names an absent target"));
        }
    }
    Ok(())
}

fn reserved_output_path(path: &str) -> bool {
    ["entry.json", "zryna-resolved-build-plan-v0.json", super::super::PACKAGE_BUILD_MANIFEST_NAME]
        .iter()
        .any(|reserved| {
            path == *reserved
                || path.strip_prefix(*reserved).is_some_and(|suffix| suffix.starts_with('/'))
        })
}

fn paths_conflict(left: &str, right: &str) -> bool {
    left == right
        || right.strip_prefix(left).is_some_and(|suffix| suffix.starts_with('/'))
        || left.strip_prefix(right).is_some_and(|suffix| suffix.starts_with('/'))
}

fn validate_target(target: &BuildTarget) -> Result<(), PackageBuildError> {
    validate_triple(&target.triple)?;
    validate_name(&target.abi)?;
    if !target.features.is_empty() {
        return Err(PackageBuildError::plan("source-only target features must be empty"));
    }
    validate_name(&target.runtime.name)?;
    validate_version(&target.runtime.version)?;
    validate_digest(&target.runtime.sha256)?;
    if target.composition.contract != "zryna.cross-target-profiles.v1"
        || !compatible_row(&target.id, &target.composition.row)
    {
        return Err(PackageBuildError::plan("target composition identity is incompatible"));
    }
    validate_digest(&target.composition.host_policy_sha256)?;
    validate_digest(&target.composition.approved_request_sha256)
}

fn compatible_row(target: &BuildTargetId, row: &str) -> bool {
    match target {
        BuildTargetId::JavaScript => matches!(row, "U-JS" | "JS-BROWSER" | "JS-NODE"),
        BuildTargetId::WebAssembly => {
            matches!(row, "U-WASM" | "WIT-BROWSER" | "WIT-COMMAND" | "WIT-SERVER")
        }
        BuildTargetId::NativeLinuxX86_64 => matches!(row, "U-NATIVE" | "NATIVE-HOST"),
    }
}

fn validate_digest(value: &str) -> Result<(), PackageBuildError> {
    if value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(PackageBuildError::plan("identity contains a malformed SHA-256 digest"))
    }
}

fn validate_name(value: &str) -> Result<(), PackageBuildError> {
    if !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        Ok(())
    } else {
        Err(PackageBuildError::plan("identity name is not portable lowercase ASCII"))
    }
}

fn validate_triple(value: &str) -> Result<(), PackageBuildError> {
    if !value.is_empty()
        && value.len() <= 96
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        Ok(())
    } else {
        Err(PackageBuildError::plan("host or target triple is not portable ASCII"))
    }
}

fn validate_path(value: &str, limit: usize) -> Result<(), PackageBuildError> {
    let valid = !value.is_empty()
        && value.len() <= limit
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-/".contains(&byte)
        })
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains("//")
        && value.split('/').all(|part| {
            if matches!(part, "" | "." | "..") || part.ends_with('.') {
                return false;
            }
            let stem = part.split('.').next().unwrap_or(part);
            !matches!(stem, "con" | "prn" | "aux" | "nul")
                && !(stem.len() == 4
                    && (stem.starts_with("com") || stem.starts_with("lpt"))
                    && stem.as_bytes()[3].is_ascii_digit())
        });
    if valid { Ok(()) } else { Err(PackageBuildError::plan("output path is not portable")) }
}

fn validate_version(value: &str) -> Result<(), PackageBuildError> {
    if !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b".+_-".contains(&byte))
    {
        Ok(())
    } else {
        Err(PackageBuildError::plan("version identity is not bounded portable ASCII"))
    }
}

fn valid_epoch(value: &str) -> bool {
    value == "0"
        || (!value.starts_with('0')
            && value.len() <= 10
            && value.bytes().all(|byte| byte.is_ascii_digit()))
}

fn ordered_unique<T>(
    values: &[T],
    key: impl Fn(&T) -> &str,
    limit: usize,
    label: &str,
) -> Result<(), PackageBuildError> {
    if values.len() > limit || values.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
        Err(PackageBuildError::plan(format!("{label} are not bounded, sorted, and unique")))
    } else {
        Ok(())
    }
}
