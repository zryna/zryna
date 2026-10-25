//! Closed metadata validation; no decoded document can mint a compiler or runtime capability.
use super::model::{GenericCommand, GenericManifestWire, GenericTarget};
use std::collections::BTreeSet;
use zryna_abi::{ScalarOutcome, ScalarTrapCode};
use zryna_source::NormalizedSourcePath;

pub(super) fn check(manifest: &GenericManifestWire) -> Result<(), &'static str> {
    if manifest.format != super::FORMAT
        || manifest.profile != super::PROFILE
        || manifest.syntax_protocol != 5
    {
        return Err("generic manifest header does not match its exact version");
    }
    crate::javascript::validate_artifact_stem(&manifest.artifact_stem)
        .map_err(|_| "generic artifact stem is not portable")?;
    source(manifest)?;
    exports(manifest)?;
    artifacts(manifest)?;
    invocation(manifest)
}

fn hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn source(manifest: &GenericManifestWire) -> Result<(), &'static str> {
    let source = &manifest.source;
    if source.modules.is_empty() || source.modules.len() > crate::MAX_MODULE_FILES {
        return Err("generic source inventory is empty");
    }
    let mut paths = BTreeSet::new();
    let mut portable = BTreeSet::new();
    let mut source_bytes = 0u64;
    for (index, module) in source.modules.iter().enumerate() {
        source_bytes = source_bytes.checked_add(module.bytes).ok_or("source byte overflow")?;
        if usize::try_from(module.id).ok() != Some(index)
            || NormalizedSourcePath::new(module.path.clone()).is_err()
            // The shared import grammar admits dotfile dependencies such as .zry.
            || !module.path.as_bytes().ends_with(b".zry")
            || !hash(&module.sha256)
            || !paths.insert(module.path.as_str())
            || !portable.insert(module.path.to_ascii_lowercase())
            || module.bytes
                > u64::try_from(zryna_source::MAX_SOURCE_FILE_BYTES)
                    .map_err(|_| "source file bound")?
            || source_bytes
                > u64::try_from(crate::MAX_MODULE_SOURCE_BYTES)
                    .map_err(|_| "source project bound")?
        {
            return Err("generic source module identity is inconsistent");
        }
    }
    if source.modules.windows(2).any(|pair| pair[0].path >= pair[1].path)
        || source
            .modules
            .get(usize::try_from(source.entry_module).map_err(|_| "entry index")?)
            .is_none_or(|module| module.path != source.entrypoint)
    {
        return Err("generic entry module or canonical source order is inconsistent");
    }
    for digest in [
        &source.graph_sha256,
        &source.execution_sha256,
        &source.type_universe_sha256,
        &source.linear32_sha256,
        &source.linux_x86_64_sha256,
        &source.runtime_abi_sha256,
    ] {
        if !hash(digest) {
            return Err("generic derivation digest is malformed");
        }
    }
    let mut edges = BTreeSet::new();
    if source.edges.len() > crate::MAX_MODULE_IMPORT_EDGES {
        return Err("generic named-import inventory exceeds its existing bound");
    }
    for edge in &source.edges {
        let importer = NormalizedSourcePath::new(edge.importer.clone())
            .map_err(|_| "generic import source path is invalid")?;
        let resolved = zryna_source::resolve_explicit_zry_import(&importer, &edge.specifier)
            .map_err(|_| "generic import specifier is invalid")?;
        if !paths.contains(edge.importer.as_str())
            || !paths.contains(edge.target.as_str())
            || resolved.as_str() != edge.target
            || edge.specifier.is_empty()
            || edge.imported.is_empty()
            || edge.local.is_empty()
            || [&edge.specifier, &edge.imported, &edge.local]
                .iter()
                .any(|value| value.chars().any(char::is_control))
            || !edges.insert((
                &edge.importer,
                &edge.target,
                &edge.specifier,
                &edge.imported,
                &edge.local,
            ))
        {
            return Err("generic named-import inventory is inconsistent");
        }
    }
    Ok(())
}

fn exports(manifest: &GenericManifestWire) -> Result<(), &'static str> {
    if manifest.exports.len() > zryna_abi::MAX_ABI_EXPORTS {
        return Err("generic scalar export count is invalid");
    }
    let mut total = 0usize;
    let mut names = BTreeSet::new();
    for export in &manifest.exports {
        total = total.checked_add(export.parameters.len()).ok_or("scalar parameter overflow")?;
        if export.name.is_empty()
            || export.name.len() > zryna_abi::MAX_LOGICAL_EXPORT_NAME_BYTES
            || export.name.chars().any(char::is_control)
            || !names.insert(&export.name)
            || export.parameters.len() > zryna_abi::MAX_ABI_PARAMETERS_PER_EXPORT
            || total > zryna_abi::MAX_ABI_PARAMETERS_PER_MODULE
        {
            return Err("generic scalar export signature is invalid");
        }
    }
    if manifest.exports.windows(2).any(|pair| pair[0].name >= pair[1].name) {
        return Err("generic scalar exports are not canonical");
    }
    Ok(())
}

fn artifacts(manifest: &GenericManifestWire) -> Result<(), &'static str> {
    if !matches!(manifest.artifacts.len(), 1 | 3)
        || manifest.artifacts.windows(2).any(|pair| pair[0].target >= pair[1].target)
    {
        return Err("generic target inventory is not a complete supported selection");
    }
    for artifact in &manifest.artifacts {
        let (kind, extension) = artifact.target.material();
        let expected =
            format!("{}/{}.{}", artifact.target.as_str(), manifest.artifact_stem, extension);
        if artifact.kind != kind
            || artifact.path != expected
            || artifact.bytes == 0
            || !hash(&artifact.sha256)
        {
            return Err("generic target artifact metadata is inconsistent");
        }
    }
    let native =
        manifest.artifacts.iter().find(|artifact| artifact.target == GenericTarget::Native);
    match (native, &manifest.native) {
        (Some(artifact), Some(record)) => {
            if record.target != "x86_64-unknown-linux-gnu"
                || record.ordinary_object_sha256 != artifact.sha256
                || record.template_sha256
                    != "df9be4eee10a50225414335502f6b8153874b8d6d4b9ea0903932f505d77f980"
                || record.header_sha256
                    != "24669a8f02db050f00dd351738f337f5f1d7e5bfa4387c0e942372991945c070"
                || !hash(&record.rendered_sha256)
                || !matches!(record.required_tls_bytes, 0 | 33_554_448)
            {
                return Err("generic native runtime/object metadata is inconsistent");
            }
            match (manifest.command, &record.execution) {
                (GenericCommand::Build, None) => {}
                (GenericCommand::Run, Some(execution)) => {
                    if execution.ordinary_object != record.ordinary_object_sha256
                        || [
                            &execution.ordinary_object,
                            &execution.runtime_object,
                            &execution.invocation_object,
                            &execution.harness,
                            &execution.executable,
                            &execution.dependency_receipt,
                        ]
                        .iter()
                        .any(|value| !hash(value))
                    {
                        return Err("generic native execution provenance is inconsistent");
                    }
                }
                _ => return Err("generic native execution record disagrees with command"),
            }
        }
        (None, None) => {}
        _ => return Err("generic native metadata does not match its target selection"),
    }
    Ok(())
}

fn invocation(manifest: &GenericManifestWire) -> Result<(), &'static str> {
    let record = match (manifest.command, &manifest.invocation) {
        (GenericCommand::Build, None) => return Ok(()),
        (GenericCommand::Run, Some(record)) => record,
        _ => return Err("generic command and invocation record disagree"),
    };
    let export = manifest
        .exports
        .iter()
        .find(|export| export.name == record.export)
        .ok_or("generic invocation selects no scalar export")?;
    if record.arguments.len() != export.parameters.len()
        || record.arguments.iter().zip(&export.parameters).any(|(value, ty)| value.ty() != *ty)
        || record.outcomes.len() != manifest.artifacts.len()
    {
        return Err("generic invocation arguments or result inventory are inconsistent");
    }
    for (outcome, artifact) in record.outcomes.iter().zip(&manifest.artifacts) {
        if outcome.target != artifact.target {
            return Err("generic invocation target order");
        }
        match outcome.outcome {
            ScalarOutcome::Returned { value } if value.ty() == export.result => {}
            ScalarOutcome::Trapped {
                code:
                    ScalarTrapCode::Bounds
                    | ScalarTrapCode::Allocation
                    | ScalarTrapCode::Capacity
                    | ScalarTrapCode::Refcount
                    | ScalarTrapCode::Utf8,
            } => {}
            _ => return Err("generic invocation contains an untyped result or host failure"),
        }
    }
    Ok(())
}
