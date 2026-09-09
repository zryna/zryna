//! Independent final-byte audit for the public scalar component artifact.

use wasmparser::{
    CanonicalFunction, ComponentAlias, ComponentExternalKind, ComponentType, ComponentTypeRef,
    ComponentValType, Encoding, ExternalKind, Instance, Parser, Payload, PrimitiveValType,
    Validator, WasmFeatures,
};
use zryna_diagnostics::Diagnostic;

use crate::{ValidatedWebAssemblyArtifact, wit_world_audit::AuthenticatedBrowserWorld};

use super::{METADATA_SECTION, ScalarExport};

pub(super) const MAX_COMPONENT_BYTES: usize = 1024 * 1024;

pub(super) fn audit(
    bytes: &[u8],
    core: &ValidatedWebAssemblyArtifact,
    exports: &[ScalarExport],
    metadata: &str,
    world: &AuthenticatedBrowserWorld,
) -> Result<(), Diagnostic> {
    check_byte_budget(bytes.len())?;
    let topology = Topology::read(bytes, metadata, exports.len())?;
    if topology.module != Some(core.bytes()) {
        return Err(invalid("scalar component substituted the retained core module"));
    }
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(core.bytes())
        .map_err(|_| invalid("retained scalar core is not WebAssembly 1.0"))?;
    crate::audit_profile(core.bytes())?;
    Validator::new_with_features(WasmFeatures::WASM1.union(WasmFeatures::COMPONENT_MODEL))
        .validate_all(bytes)
        .map_err(|_| invalid("scalar component failed complete binary and type validation"))?;
    topology.compare(exports)?;
    let browser = world
        .audit()
        .worlds()
        .first()
        .filter(|candidate| candidate.identity() == super::BROWSER_WORLD)
        .ok_or_else(|| invalid("scalar component lost its browser world identity"))?;
    if !browser.explicit_imports().is_empty()
        || !browser.resolved_imports().is_empty()
        || !browser.exports().is_empty()
    {
        return Err(invalid("scalar component browser capability world is not empty"));
    }
    Ok(())
}

pub(super) fn check_byte_budget(bytes: usize) -> Result<(), Diagnostic> {
    if bytes > MAX_COMPONENT_BYTES {
        Err(invalid("scalar component exceeds 1048576 bytes"))
    } else {
        Ok(())
    }
}

struct Topology<'a> {
    module: Option<&'a [u8]>,
    inside_module: bool,
    metadata: bool,
    instance: bool,
    aliases: Vec<(u32, &'a str)>,
    types: Vec<wasmparser::ComponentFuncType<'a>>,
    lifts: Vec<(u32, u32)>,
    exports: Vec<(&'a str, u32, u32)>,
    outer_payloads: usize,
    interface_limit: usize,
}

impl<'a> Topology<'a> {
    fn read(
        bytes: &'a [u8],
        expected_metadata: &str,
        interface_limit: usize,
    ) -> Result<Self, Diagnostic> {
        let mut topology = Self {
            module: None,
            inside_module: false,
            metadata: false,
            instance: false,
            aliases: Vec::new(),
            types: Vec::new(),
            lifts: Vec::new(),
            exports: Vec::new(),
            outer_payloads: 0,
            interface_limit,
        };
        for payload in Parser::new(0).parse_all(bytes) {
            topology.payload(
                bytes,
                expected_metadata,
                payload.map_err(|_| invalid("scalar component syntax is malformed"))?,
            )?;
        }
        if topology.inside_module || !topology.metadata || !topology.instance {
            return Err(invalid("scalar component topology is incomplete"));
        }
        Ok(topology)
    }

    fn payload(
        &mut self,
        bytes: &'a [u8],
        expected_metadata: &str,
        payload: Payload<'a>,
    ) -> Result<(), Diagnostic> {
        if self.inside_module {
            if matches!(payload, Payload::End(_)) {
                self.inside_module = false;
            }
            return Ok(());
        }
        self.outer_payloads += 1;
        if self.outer_payloads > 32 {
            return Err(invalid("scalar component exceeds 32 outer payloads"));
        }
        match payload {
            Payload::Version { encoding: Encoding::Component, .. } | Payload::End(_) => Ok(()),
            Payload::CustomSection(section)
                if !self.metadata
                    && section.name() == METADATA_SECTION
                    && section.data() == expected_metadata.as_bytes() =>
            {
                self.metadata = true;
                Ok(())
            }
            Payload::ModuleSection { unchecked_range, .. } if self.module.is_none() => {
                self.module = Some(slice(bytes, unchecked_range)?);
                self.inside_module = true;
                Ok(())
            }
            Payload::InstanceSection(items) if !self.instance && items.count() == 1 => {
                let mut items = items.into_iter();
                match items.next().transpose().map_err(|_| invalid("invalid core instance"))? {
                    Some(Instance::Instantiate { module_index: 0, args }) if args.is_empty() => {
                        self.instance = true;
                        Ok(())
                    }
                    _ => Err(invalid("scalar core instantiation topology differs")),
                }
            }
            Payload::ComponentAliasSection(items) => {
                for alias in items {
                    check_interface_budget(self.aliases.len(), self.interface_limit)?;
                    match alias.map_err(|_| invalid("invalid scalar component alias"))? {
                        ComponentAlias::CoreInstanceExport {
                            kind: ExternalKind::Func,
                            instance_index,
                            name,
                        } => self.aliases.push((instance_index, name)),
                        _ => return Err(invalid("scalar component contains a non-function alias")),
                    }
                }
                Ok(())
            }
            Payload::ComponentTypeSection(items) => {
                for ty in items {
                    check_interface_budget(self.types.len(), self.interface_limit)?;
                    match ty.map_err(|_| invalid("invalid scalar component type"))? {
                        ComponentType::Func(function) => self.types.push(function),
                        _ => return Err(invalid("scalar component contains a non-function type")),
                    }
                }
                Ok(())
            }
            Payload::ComponentCanonicalSection(items) => {
                for function in items {
                    check_interface_budget(self.lifts.len(), self.interface_limit)?;
                    match function.map_err(|_| invalid("invalid canonical operation"))? {
                        CanonicalFunction::Lift { core_func_index, type_index, options }
                            if options.is_empty() =>
                        {
                            self.lifts.push((core_func_index, type_index));
                        }
                        _ => return Err(invalid("scalar component canonical topology differs")),
                    }
                }
                Ok(())
            }
            Payload::ComponentExportSection(items) => {
                for export in items {
                    check_interface_budget(self.exports.len(), self.interface_limit)?;
                    let export = export.map_err(|_| invalid("invalid scalar component export"))?;
                    let Some(ComponentTypeRef::Func(type_index)) = export.ty else {
                        return Err(invalid(
                            "scalar component export lacks an exact function type",
                        ));
                    };
                    if export.kind != ComponentExternalKind::Func {
                        return Err(invalid("scalar component contains a non-function export"));
                    }
                    self.exports.push((export.name.name, export.index, type_index));
                }
                Ok(())
            }
            _ => Err(invalid(
                "scalar component has imports, a start, nesting or an unsupported section",
            )),
        }
    }

    fn compare(&self, expected: &[ScalarExport]) -> Result<(), Diagnostic> {
        if self.aliases.len() != expected.len()
            || self.types.len() != expected.len()
            || self.lifts.len() != expected.len()
            || self.exports.len() != expected.len()
        {
            return Err(invalid("scalar component interface count differs from verified IR"));
        }
        for (index, expected) in expected.iter().enumerate() {
            let index_u32 = u32::try_from(index)
                .map_err(|_| invalid("scalar component export index is out of range"))?;
            let (instance, alias) = self.aliases[index];
            if instance != 0 || alias != expected.webassembly {
                return Err(invalid("scalar component core export alias differs"));
            }
            let ty = &self.types[index];
            if ty.async_
                || ty.params.len() != expected.arity
                || ty.params.iter().enumerate().any(|(parameter, (name, ty))| {
                    *name != format!("arg{parameter}")
                        || *ty != ComponentValType::Primitive(PrimitiveValType::S32)
                })
                || ty.result != Some(ComponentValType::Primitive(PrimitiveValType::S32))
            {
                return Err(invalid("scalar component canonical function type differs"));
            }
            if self.lifts[index] != (index_u32, index_u32) {
                return Err(invalid("scalar component canonical lift index differs"));
            }
            if self.exports[index] != (expected.logical.as_str(), index_u32, index_u32) {
                return Err(invalid("scalar component public export differs"));
            }
        }
        Ok(())
    }
}

pub(super) fn check_interface_budget(current: usize, limit: usize) -> Result<(), Diagnostic> {
    if current >= limit {
        Err(invalid("scalar component exceeds its verified interface resource limit"))
    } else {
        Ok(())
    }
}

fn slice(bytes: &[u8], range: std::ops::Range<u64>) -> Result<&[u8], Diagnostic> {
    let start = usize::try_from(range.start)
        .map_err(|_| invalid("scalar component section start is out of range"))?;
    let end = usize::try_from(range.end)
        .map_err(|_| invalid("scalar component section end is out of range"))?;
    bytes.get(start..end).ok_or_else(|| invalid("scalar component section exceeds retained bytes"))
}

fn invalid(message: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4021",
        None,
        message,
        "rebuild the component from the exact verified core and pinned browser world",
    )
}
