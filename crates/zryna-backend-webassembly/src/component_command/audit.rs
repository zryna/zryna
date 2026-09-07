//! Audit final bytes before sealing: bounded syntax, exact execution graph, then WIT equality.

use std::ops::Range;

use wasmparser::{
    CanonicalFunction, ComponentAlias, ComponentExternalKind, ComponentInstance, ComponentTypeRef,
    Encoding, ExternalKind, Instance, Parser, Payload, Validator, WasmFeatures,
};
use wit_parser::decoding::DecodedWasm;
use zryna_diagnostics::Diagnostic;

use crate::{ValidatedWebAssemblyArtifact, wit_world_audit::AuthenticatedCommandWorld};

use super::{
    bridge::CommandInvocation,
    bridge_audit, graph_budget,
    type_budget::{self, TypeBudget},
    type_graph,
};

const MAX_BYTES: usize = 1024 * 1024;

pub(super) fn audit(
    bytes: &[u8],
    core: &ValidatedWebAssemblyArtifact,
    invocation: &CommandInvocation,
    world: &AuthenticatedCommandWorld,
) -> Result<(), Diagnostic> {
    if bytes.len() > MAX_BYTES {
        return Err(invalid("command component exceeds 1048576 bytes"));
    }
    let (modules, _) = topology(bytes)?;
    if modules[0] != core.bytes() {
        return Err(invalid("command component substituted the retained scalar core"));
    }
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(modules[0])
        .map_err(|_| invalid("retained scalar core is not WebAssembly 1.0"))?;
    crate::audit_profile(modules[0])?;
    bridge_audit::audit(modules[1], invocation)?;
    Validator::new_with_features(WasmFeatures::WASM1.union(WasmFeatures::COMPONENT_MODEL))
        .validate_all(bytes)
        .map_err(|_| invalid("command component failed complete binary and type validation"))?;
    let DecodedWasm::Component(resolve, decoded_world) = wit_parser::decoding::decode(bytes)
        .map_err(|_| invalid("command component public type graph could not be decoded"))?
    else {
        return Err(invalid(
            "command bytes describe a WIT package instead of an executable component",
        ));
    };
    type_graph::compare(world, &resolve, decoded_world)
}

fn topology(bytes: &[u8]) -> Result<([&[u8]; 2], usize), Diagnostic> {
    let mut modules = Vec::new();
    let mut inside_module = false;
    let mut sections = 0;
    let mut budget = TypeBudget::default();
    let mut imports = 0;
    let mut instances = 0;
    let mut aliases = 0;
    let mut core_run = false;
    let mut lifted = false;
    let mut exported_instance = false;
    let mut exported = false;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(malformed)?;
        if inside_module {
            match payload {
                Payload::End(_) => inside_module = false,
                Payload::Version { encoding: Encoding::Module, .. }
                | Payload::TypeSection(_)
                | Payload::ImportSection(_)
                | Payload::FunctionSection(_)
                | Payload::ExportSection(_)
                | Payload::CodeSectionStart { .. }
                | Payload::CodeSectionEntry(_) => {}
                _ => {
                    return Err(invalid(
                        "command core module has memory, tables, a start or another unsupported section",
                    ));
                }
            }
            continue;
        }
        sections += 1;
        if sections > 256 {
            return Err(invalid("command component exceeds 256 outer payloads"));
        }
        match payload {
            Payload::Version { encoding: Encoding::Component, .. } | Payload::End(_) => {}
            Payload::ModuleSection { unchecked_range, .. } if modules.len() < 2 => {
                modules.push(slice(bytes, unchecked_range)?);
                inside_module = true;
            }
            Payload::ComponentTypeSection(types) => {
                let range = types.range();
                budget.section(slice(bytes, range.clone())?, range.start)?;
            }
            Payload::ComponentImportSection(items) if modules.is_empty() => {
                if items.count() > 16 - imports {
                    return Err(invalid("command component has extra imports"));
                }
                for import in items {
                    let import = import.map_err(malformed)?;
                    type_budget::external_name(import.name)?;
                    let ComponentTypeRef::Instance(index) = import.ty else {
                        return Err(invalid("command import is not an interface instance"));
                    };
                    budget.interface_use(index, true)?;
                    imports += 1;
                }
            }
            Payload::InstanceSection(items) => {
                if items.count() > 2 - instances {
                    return Err(invalid("command component has extra core instances"));
                }
                let range = items.range();
                graph_budget::core_instances(slice(bytes, range.clone())?, range.start)?;
                for instance in items {
                    match instance.map_err(malformed)? {
                        Instance::Instantiate { module_index: 0, args }
                            if instances == 0 && args.is_empty() => {}
                        Instance::Instantiate { module_index: 1, args }
                            if instances == 1
                                && args.len() == 1
                                && args[0].name == "scalar-core"
                                && args[0].index == 0 => {}
                        _ => return Err(invalid("command core instantiation graph differs")),
                    }
                    instances += 1;
                }
            }
            Payload::ComponentAliasSection(items) => {
                if items.count() > 4096 - aliases {
                    return Err(invalid("command component exceeds 4096 aliases"));
                }
                aliases += items.count();
                for alias in items {
                    match alias.map_err(malformed)? {
                        ComponentAlias::CoreInstanceExport {
                            kind: ExternalKind::Func,
                            instance_index: 1,
                            name: "run",
                        } if !core_run => core_run = true,
                        alias => budget.alias(alias)?,
                    }
                }
            }
            Payload::ComponentCanonicalSection(items) if !lifted && items.count() == 1 => {
                let range = items.range();
                graph_budget::canonical(slice(bytes, range.clone())?, range.start)?;
                for function in items {
                    match function.map_err(malformed)? {
                        CanonicalFunction::Lift { core_func_index: 0, options, .. }
                            if core_run && options.is_empty() =>
                        {
                            lifted = true
                        }
                        _ => {
                            return Err(invalid(
                                "command canonical lift differs or enables additional operations",
                            ));
                        }
                    }
                }
            }
            Payload::ComponentInstanceSection(items)
                if !exported_instance && items.count() == 1 =>
            {
                let range = items.range();
                graph_budget::component_instance(slice(bytes, range.clone())?, range.start)?;
                for instance in items {
                    match instance.map_err(malformed)? {
                        ComponentInstance::FromExports(items) if lifted && items.len() == 1 => {
                            let export = &items[0];
                            type_budget::external_name(export.name)?;
                            if export.name.name != "run"
                                || export.kind != ComponentExternalKind::Func
                                || export.index != 0
                                || export.ty.is_some()
                            {
                                return Err(invalid(
                                    "command run instance exports a different function",
                                ));
                            }
                            exported_instance = true;
                        }
                        _ => return Err(invalid("command component instance graph differs")),
                    }
                }
            }
            Payload::ComponentExportSection(items) if !exported && items.count() == 1 => {
                for export in items {
                    let export = export.map_err(malformed)?;
                    type_budget::external_name(export.name)?;
                    if !exported_instance
                        || export.name.name != "wasi:cli/run@0.2.12"
                        || export.kind != ComponentExternalKind::Instance
                        || export.index != imports
                        || !matches!(export.ty, Some(ComponentTypeRef::Instance(_)))
                    {
                        return Err(invalid("command component exports a different run interface"));
                    }
                    if let Some(ComponentTypeRef::Instance(index)) = export.ty {
                        budget.interface_use(index, false)?;
                    }
                    exported = true;
                }
            }
            _ => {
                return Err(invalid(
                    "command component has a start, nesting or an unsupported section",
                ));
            }
        }
    }
    if inside_module || imports != 16 || instances != 2 || !core_run || !lifted || !exported {
        return Err(invalid("command component execution graph is incomplete"));
    }
    let modules = modules
        .try_into()
        .map_err(|_| invalid("command component does not retain exactly two core modules"))?;
    Ok((modules, budget.identities()))
}

#[cfg(test)]
pub(super) fn identity_budget_used(bytes: &[u8]) -> Result<usize, Diagnostic> {
    topology(bytes).map(|(_, identities)| identities)
}

fn slice(bytes: &[u8], range: Range<u64>) -> Result<&[u8], Diagnostic> {
    let start = usize::try_from(range.start)
        .map_err(|_| invalid("command section offset is out of range"))?;
    let end =
        usize::try_from(range.end).map_err(|_| invalid("command section end is out of range"))?;
    bytes.get(start..end).ok_or_else(|| invalid("command section extends past the retained bytes"))
}

fn malformed(_: wasmparser::BinaryReaderError) -> Diagnostic {
    invalid("command component syntax is malformed")
}

fn invalid(message: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4015",
        None,
        message,
        "rebuild the command from the matching verified core, WIT and private bridge",
    )
}
