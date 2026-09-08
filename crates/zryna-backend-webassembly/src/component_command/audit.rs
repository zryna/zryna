//! Audit final bytes before sealing: bounded syntax, exact execution graph, then WIT equality.

use std::ops::Range;

use wasmparser::{
    CanonicalFunction, ComponentAlias, ComponentAliasSectionReader,
    ComponentCanonicalSectionReader, ComponentExportSectionReader, ComponentExternalKind,
    ComponentImportSectionReader, ComponentInstance, ComponentInstanceSectionReader,
    ComponentTypeRef, ComponentTypeSectionReader, Encoding, ExternalKind, Instance,
    InstanceSectionReader, Parser, Payload, Validator, WasmFeatures,
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
const CORE_RUN: u8 = 1;
const LIFTED: u8 = 2;
const EXPORTED_INSTANCE: u8 = 4;
const EXPORTED: u8 = 8;

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
    let mut topology = Topology::default();
    for payload in Parser::new(0).parse_all(bytes) {
        topology.payload(bytes, payload.map_err(malformed)?)?;
    }
    topology.finish()
}

#[derive(Default)]
struct Topology<'a> {
    modules: Vec<&'a [u8]>,
    inside_module: bool,
    sections: usize,
    budget: TypeBudget,
    imports: u32,
    instances: u32,
    aliases: u32,
    progress: u8,
}

impl<'a> Topology<'a> {
    fn payload(&mut self, bytes: &'a [u8], payload: Payload<'a>) -> Result<(), Diagnostic> {
        if self.inside_module {
            return self.core_payload(&payload);
        }
        self.sections += 1;
        if self.sections > 256 {
            return Err(invalid("command component exceeds 256 outer payloads"));
        }
        match payload {
            Payload::Version { encoding: Encoding::Component, .. } | Payload::End(_) => Ok(()),
            Payload::ModuleSection { unchecked_range, .. } if self.modules.len() < 2 => {
                self.modules.push(slice(bytes, unchecked_range)?);
                self.inside_module = true;
                Ok(())
            }
            Payload::ComponentTypeSection(items) => self.types(bytes, &items),
            Payload::ComponentImportSection(items) if self.modules.is_empty() => {
                self.imports(items)
            }
            Payload::InstanceSection(items) => self.instances(bytes, items),
            Payload::ComponentAliasSection(items) => self.aliases(items),
            Payload::ComponentCanonicalSection(items)
                if !self.has(LIFTED) && items.count() == 1 =>
            {
                self.canonical(bytes, items)
            }
            Payload::ComponentInstanceSection(items)
                if !self.has(EXPORTED_INSTANCE) && items.count() == 1 =>
            {
                self.component_instance(bytes, items)
            }
            Payload::ComponentExportSection(items) if !self.has(EXPORTED) && items.count() == 1 => {
                self.exports(items)
            }
            _ => Err(invalid("command component has a start, nesting or an unsupported section")),
        }
    }

    fn core_payload(&mut self, payload: &Payload<'a>) -> Result<(), Diagnostic> {
        match payload {
            Payload::End(_) => self.inside_module = false,
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
        Ok(())
    }

    fn types(
        &mut self,
        bytes: &[u8],
        items: &ComponentTypeSectionReader<'a>,
    ) -> Result<(), Diagnostic> {
        let range = items.range();
        self.budget.section(slice(bytes, range.clone())?, range.start)
    }

    fn imports(&mut self, items: ComponentImportSectionReader<'a>) -> Result<(), Diagnostic> {
        if items.count() > 16 - self.imports {
            return Err(invalid("command component has extra imports"));
        }
        for import in items {
            let import = import.map_err(malformed)?;
            type_budget::external_name(import.name)?;
            let ComponentTypeRef::Instance(index) = import.ty else {
                return Err(invalid("command import is not an interface instance"));
            };
            self.budget.interface_use(index, true)?;
            self.imports += 1;
        }
        Ok(())
    }

    fn instances(
        &mut self,
        bytes: &[u8],
        items: InstanceSectionReader<'a>,
    ) -> Result<(), Diagnostic> {
        if items.count() > 2 - self.instances {
            return Err(invalid("command component has extra core instances"));
        }
        let range = items.range();
        graph_budget::core_instances(slice(bytes, range.clone())?, range.start)?;
        for instance in items {
            match instance.map_err(malformed)? {
                Instance::Instantiate { module_index: 0, args }
                    if self.instances == 0 && args.is_empty() => {}
                Instance::Instantiate { module_index: 1, args }
                    if self.instances == 1
                        && args.len() == 1
                        && args[0].name == "scalar-core"
                        && args[0].index == 0 => {}
                _ => return Err(invalid("command core instantiation graph differs")),
            }
            self.instances += 1;
        }
        Ok(())
    }

    fn aliases(&mut self, items: ComponentAliasSectionReader<'a>) -> Result<(), Diagnostic> {
        if items.count() > 4096 - self.aliases {
            return Err(invalid("command component exceeds 4096 aliases"));
        }
        self.aliases += items.count();
        for alias in items {
            match alias.map_err(malformed)? {
                ComponentAlias::CoreInstanceExport {
                    kind: ExternalKind::Func,
                    instance_index: 1,
                    name: "run",
                } if !self.has(CORE_RUN) => self.mark(CORE_RUN),
                alias => self.budget.alias(&alias)?,
            }
        }
        Ok(())
    }

    fn canonical(
        &mut self,
        bytes: &[u8],
        items: ComponentCanonicalSectionReader<'a>,
    ) -> Result<(), Diagnostic> {
        let range = items.range();
        graph_budget::canonical(slice(bytes, range.clone())?, range.start)?;
        for function in items {
            match function.map_err(malformed)? {
                CanonicalFunction::Lift { core_func_index: 0, options, .. }
                    if self.has(CORE_RUN) && options.is_empty() =>
                {
                    self.mark(LIFTED);
                }
                _ => {
                    return Err(invalid(
                        "command canonical lift differs or enables additional operations",
                    ));
                }
            }
        }
        Ok(())
    }

    fn component_instance(
        &mut self,
        bytes: &[u8],
        items: ComponentInstanceSectionReader<'a>,
    ) -> Result<(), Diagnostic> {
        let range = items.range();
        graph_budget::component_instance(slice(bytes, range.clone())?, range.start)?;
        for instance in items {
            match instance.map_err(malformed)? {
                ComponentInstance::FromExports(items) if self.has(LIFTED) && items.len() == 1 => {
                    let export = &items[0];
                    type_budget::external_name(export.name)?;
                    if export.name.name != "run"
                        || export.kind != ComponentExternalKind::Func
                        || export.index != 0
                        || export.ty.is_some()
                    {
                        return Err(invalid("command run instance exports a different function"));
                    }
                    self.mark(EXPORTED_INSTANCE);
                }
                _ => return Err(invalid("command component instance graph differs")),
            }
        }
        Ok(())
    }

    fn exports(&mut self, items: ComponentExportSectionReader<'a>) -> Result<(), Diagnostic> {
        for export in items {
            let export = export.map_err(malformed)?;
            type_budget::external_name(export.name)?;
            if !self.has(EXPORTED_INSTANCE)
                || export.name.name != "wasi:cli/run@0.2.12"
                || export.kind != ComponentExternalKind::Instance
                || export.index != self.imports
                || !matches!(export.ty, Some(ComponentTypeRef::Instance(_)))
            {
                return Err(invalid("command component exports a different run interface"));
            }
            if let Some(ComponentTypeRef::Instance(index)) = export.ty {
                self.budget.interface_use(index, false)?;
            }
            self.mark(EXPORTED);
        }
        Ok(())
    }

    fn finish(self) -> Result<([&'a [u8]; 2], usize), Diagnostic> {
        if self.inside_module
            || self.imports != 16
            || self.instances != 2
            || !self.has(CORE_RUN)
            || !self.has(LIFTED)
            || !self.has(EXPORTED)
        {
            return Err(invalid("command component execution graph is incomplete"));
        }
        let modules = self
            .modules
            .try_into()
            .map_err(|_| invalid("command component does not retain exactly two core modules"))?;
        Ok((modules, self.budget.identities()))
    }

    const fn has(&self, phase: u8) -> bool {
        self.progress & phase != 0
    }

    fn mark(&mut self, phase: u8) {
        self.progress |= phase;
    }
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
