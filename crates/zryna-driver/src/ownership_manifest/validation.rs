use sha2::{Digest, Sha256};
use zryna_diagnostics::Diagnostic;
use zryna_diagnostics::Severity;
use zryna_source::NormalizedSourcePath;

use super::{
    DATA_OWNERSHIP_CANDIDATE_PROFILE, GRAPH_DOMAIN, MANIFEST_VERSION, ManifestArtifact,
    ManifestCommand, ManifestEdge, ManifestLocation, OwnershipManifestResult, OwnershipManifestV3,
    OwnershipTarget, PROTOCOL_VERSION, TargetMetadata, decode_hex, error, hex, is_hex, push_text,
    push_u32,
};

pub(super) fn validate(manifest: &OwnershipManifestV3) -> Result<(), Diagnostic> {
    if manifest.version != MANIFEST_VERSION
        || manifest.profile != DATA_OWNERSHIP_CANDIDATE_PROFILE
        || manifest.protocol_version != PROTOCOL_VERSION
        || manifest.sources.is_empty()
        || manifest.sources.len() > crate::MAX_MODULE_FILES
        || manifest.edges.len() > crate::MAX_MODULE_IMPORT_EDGES
        || manifest.targets.is_empty()
        || manifest.targets.len() > 3
        || manifest.targets != manifest.artifacts.iter().map(|item| item.target).collect::<Vec<_>>()
        || manifest.targets.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(error("manifest v3 identity or canonical inventory is invalid"));
    }
    crate::javascript::validate_artifact_stem(&manifest.stem)?;
    let entry = NormalizedSourcePath::new(manifest.entrypoint.clone())
        .map_err(|_| error("manifest entrypoint is not normalized"))?;
    if manifest.sources.iter().enumerate().any(|(index, source)| {
        u32::try_from(index).ok() != Some(source.id)
            || NormalizedSourcePath::new(source.path.clone()).is_err()
            || !is_hex(&source.sha256)
    }) || !manifest.sources.windows(2).all(|pair| pair[0].path < pair[1].path)
        || !manifest.sources.iter().any(|source| source.path == entry.as_str())
        || !is_hex(&manifest.graph_sha256)
        || !is_hex(&manifest.layouts.type_universe)
        || !is_hex(&manifest.layouts.linear32)
        || !is_hex(&manifest.layouts.linux_x86_64)
        || manifest.runtime_abi.identifier != "zryna-ownership-runtime-v1"
        || manifest.runtime_abi.version != 1
        || !is_hex(&manifest.runtime_abi.sha256)
    {
        return Err(error("manifest v3 source, layout, or runtime identity is invalid"));
    }
    validate_graph(manifest)?;
    validate_artifacts(manifest)?;
    validate_invocation_and_diagnostics(manifest)?;
    validate_results(manifest.command, &manifest.targets, &manifest.results)
}

fn validate_graph(manifest: &OwnershipManifestV3) -> Result<(), Diagnostic> {
    let paths = manifest.sources.iter().map(|source| source.path.as_str()).collect::<Vec<_>>();
    if manifest.edges.iter().any(|edge| !valid_edge(edge, &paths))
        || !manifest.edges.windows(2).all(|pair| {
            (&pair[0].importer, &pair[0].specifier, &pair[0].imported, &pair[0].local)
                < (&pair[1].importer, &pair[1].specifier, &pair[1].imported, &pair[1].local)
        })
    {
        return Err(error("manifest v3 module edges are invalid or out of order"));
    }
    let mut bytes = GRAPH_DOMAIN.to_vec();
    push_u32(&mut bytes, 1)?;
    push_text(&mut bytes, &manifest.entrypoint)?;
    push_u32(&mut bytes, manifest.sources.len())?;
    for source in &manifest.sources {
        push_text(&mut bytes, &source.path)?;
        bytes.extend_from_slice(&decode_hex(&source.sha256)?);
    }
    push_u32(&mut bytes, manifest.edges.len())?;
    for edge in &manifest.edges {
        for value in [&edge.importer, &edge.specifier, &edge.imported, &edge.local] {
            push_text(&mut bytes, value)?;
        }
    }
    if hex(&Sha256::digest(bytes).into()) != manifest.graph_sha256 {
        return Err(error("manifest v3 graph digest does not match its canonical source closure"));
    }
    Ok(())
}

fn valid_edge(edge: &ManifestEdge, paths: &[&str]) -> bool {
    if !paths.contains(&edge.importer.as_str()) || !paths.contains(&edge.target.as_str()) {
        return false;
    }
    let Ok(importer) = NormalizedSourcePath::new(edge.importer.clone()) else {
        return false;
    };
    let Ok(target) = NormalizedSourcePath::new(edge.target.clone()) else {
        return false;
    };
    zryna_source::resolve_explicit_zry_import(&importer, &edge.specifier).ok().as_ref()
        == Some(&target)
}

fn validate_artifacts(manifest: &OwnershipManifestV3) -> Result<(), Diagnostic> {
    for artifact in &manifest.artifacts {
        let (kind, extension) = match (manifest.command, artifact.target) {
            (_, OwnershipTarget::JavaScript) => ("ecmascript-module", "mjs"),
            (_, OwnershipTarget::WebAssembly) => ("core-webassembly-module", "wasm"),
            (ManifestCommand::Build, OwnershipTarget::Native) => {
                ("linux-x86-64-relocatable-object", "o")
            }
            (ManifestCommand::Run, OwnershipTarget::Native) => {
                ("linux-x86-64-invocation-executable", "elf")
            }
        };
        if artifact.kind != kind
            || artifact.filename != format!("{}.{}", manifest.stem, extension)
            || artifact.bytes == 0
            || !is_hex(&artifact.sha256)
            || !valid_metadata(manifest.command, artifact)
        {
            return Err(error("manifest v3 artifact identity or target metadata is invalid"));
        }
    }
    Ok(())
}

fn valid_metadata(command: ManifestCommand, artifact: &ManifestArtifact) -> bool {
    match (&artifact.metadata, artifact.target, command) {
        (TargetMetadata::EcmascriptModule { scalar_abi }, OwnershipTarget::JavaScript, _) => {
            scalar_abi == "v1"
        }
        (
            TargetMetadata::CoreWebassembly { memory, imports, scalar_abi },
            OwnershipTarget::WebAssembly,
            _,
        ) => memory == "linear32-v1" && !imports && scalar_abi == "v1",
        (
            TargetMetadata::LinuxX86_64Elf {
                triple,
                format,
                scalar_abi,
                program_object_sha256,
                runtime_object_sha256,
                harness_sha256,
            },
            OwnershipTarget::Native,
            command,
        ) => {
            triple == "x86_64-unknown-linux-gnu"
                && scalar_abi == "v1"
                && is_hex(program_object_sha256)
                && match command {
                    ManifestCommand::Build => {
                        format == "elf-relocatable"
                            && program_object_sha256 == &artifact.sha256
                            && runtime_object_sha256.is_none()
                            && harness_sha256.is_none()
                    }
                    ManifestCommand::Run => {
                        format == "elf-executable"
                            && runtime_object_sha256.as_ref().is_some_and(|value| is_hex(value))
                            && harness_sha256.as_ref().is_some_and(|value| is_hex(value))
                    }
                }
        }
        _ => false,
    }
}

fn validate_invocation_and_diagnostics(manifest: &OwnershipManifestV3) -> Result<(), Diagnostic> {
    let invocation_valid = match (manifest.command, &manifest.invocation) {
        (ManifestCommand::Build, None) => true,
        (ManifestCommand::Run, Some(invocation)) => {
            !invocation.export.is_empty()
                && invocation.export.len() <= zryna_abi::MAX_LOGICAL_EXPORT_NAME_BYTES
                && invocation.arguments.len() <= zryna_abi::MAX_ABI_PARAMETERS_PER_EXPORT
        }
        _ => false,
    };
    let diagnostics_valid = manifest.diagnostics.len() <= 256
        && manifest.diagnostics.iter().all(|item| {
            item.severity == Severity::Warning
                && !item.code.is_empty()
                && item.code.len() <= 64
                && item.message.len() <= 4_096
                && item.guidance.len() <= 4_096
                && match &item.location {
                    ManifestLocation::Global => true,
                    ManifestLocation::WorkspacePath { path } => {
                        NormalizedSourcePath::new(path.clone()).is_ok()
                    }
                    ManifestLocation::Source { file, start, end } => usize::try_from(*file)
                        .ok()
                        .is_some_and(|index| index < manifest.sources.len() && start <= end),
                }
        });
    if !invocation_valid || !diagnostics_valid {
        return Err(error("manifest v3 invocation or diagnostic evidence is invalid"));
    }
    Ok(())
}

pub(super) fn validate_results(
    command: ManifestCommand,
    targets: &[OwnershipTarget],
    results: &[OwnershipManifestResult],
) -> Result<(), Diagnostic> {
    for result in results {
        if matches!(
            result.outcome(),
            zryna_abi::ScalarOutcome::Trapped {
                code: zryna_abi::ScalarTrapCode::Unreachable
                    | zryna_abi::ScalarTrapCode::TargetTrap
            }
        ) {
            return Err(error("candidate manifest requires a typed language trap"));
        }
        let mut words = 0usize;
        for event in result.trace() {
            words += match event {
                crate::OwnershipTraceEvent::Cleanup { module, function, place } => {
                    if *module > 65535 || *function > 65535 || *place > 1_048_576 {
                        return Err(error("candidate cleanup identity exceeds its fixed bound"));
                    }
                    3
                }
                _ => 1,
            };
            if words > 4096 {
                return Err(error("candidate cleanup trace exceeds its fixed bound"));
            }
        }
    }
    let actual = results.iter().map(OwnershipManifestResult::target).collect::<Vec<_>>();
    if (command == ManifestCommand::Build && !results.is_empty())
        || (command == ManifestCommand::Run && actual != targets)
    {
        return Err(error("manifest v3 results do not match command and selected target order"));
    }
    Ok(())
}
