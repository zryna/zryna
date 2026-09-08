//! Preserve every authenticated world import while connecting the two private core modules.

use wasm_encoder::{
    ComponentBuilder, ComponentExportKind, ComponentExportSection, ComponentInstanceSection,
    ComponentSection, ComponentTypeRef, ComponentValType, ExportKind, ModuleArg,
    reencode::{ReencodeComponent, RoundtripReencoder},
};
use wasmparser::{ComponentType, ComponentTypeDeclaration, Parser, Payload};
use zryna_diagnostics::Diagnostic;

use crate::{ValidatedWebAssemblyArtifact, wit_world_audit::AuthenticatedCommandWorld};

use super::bridge::{CORE_INSTANCE, RUN_EXPORT};

pub(super) const RUN_INTERFACE: &str = "wasi:cli/run@0.2.12";
pub(super) const MAX_COMPONENT_BYTES: usize = 1024 * 1024;

pub(super) fn encode(
    world: &AuthenticatedCommandWorld,
    core: &ValidatedWebAssemblyArtifact,
    bridge: &[u8],
) -> Result<Vec<u8>, Diagnostic> {
    // This encodes the exact world type without the componentizer's unused-import pruning
    // or semver merging. Its private metadata/producers sections never enter the artifact.
    let metadata = wit_component::metadata::encode(
        world.resolve(),
        world.world(),
        wit_component::StringEncoding::UTF8,
        None,
    )
    .map_err(|_| invalid("authenticated command world type could not be encoded"))?;
    let declarations = world_declarations(&metadata)?;
    let mut component = ComponentBuilder::default();
    let run_interface_type = import_world(&mut component, declarations)?;

    let core_module = component.core_module_raw(None, core.bytes());
    let bridge_module = component.core_module_raw(None, bridge);
    let core_instance = component.core_instantiate(None, core_module, []);
    let bridge_instance = component.core_instantiate(
        None,
        bridge_module,
        [(CORE_INSTANCE, ModuleArg::Instance(core_instance))],
    );
    let core_run = component.core_alias_export(None, bridge_instance, RUN_EXPORT, ExportKind::Func);
    let (result_type, result) = component.type_defined(None);
    result.result(None, None);
    let (run_type, mut function) = component.type_function(None);
    function.params([] as [(&str, ComponentValType); 0]);
    function.result(Some(ComponentValType::Type(result_type)));
    let run = component.lift_func(None, core_run, run_type, []);

    let run_instance = component.instance_count();
    let mut bytes = component.finish();
    let mut instances = ComponentInstanceSection::new();
    instances.export_items([(RUN_EXPORT, ComponentExportKind::Func, run)]);
    instances.append_to_component(&mut bytes);
    let mut exports = ComponentExportSection::new();
    exports.export(
        RUN_INTERFACE,
        ComponentExportKind::Instance,
        run_instance,
        Some(ComponentTypeRef::Instance(run_interface_type)),
    );
    exports.append_to_component(&mut bytes);
    if bytes.len() > MAX_COMPONENT_BYTES {
        return Err(invalid("command component exceeds 1048576 bytes"));
    }
    Ok(bytes)
}

fn world_declarations(bytes: &[u8]) -> Result<Box<[ComponentTypeDeclaration<'_>]>, Diagnostic> {
    let mut world = None;
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::ComponentTypeSection(types) =
            payload.map_err(|_| invalid("encoded world metadata is malformed"))?
        {
            for ty in types {
                let ComponentType::Component(wrapper) =
                    ty.map_err(|_| invalid("encoded world wrapper is malformed"))?
                else {
                    return Err(invalid("encoded world wrapper is not a component type"));
                };
                let mut wrapper = Vec::from(wrapper).into_iter();
                let Some(ComponentTypeDeclaration::Type(ComponentType::Component(declarations))) =
                    wrapper.next()
                else {
                    return Err(invalid("encoded world wrapper has an unexpected body"));
                };
                match wrapper.next() {
                    Some(ComponentTypeDeclaration::Export { name, ty })
                        if name.name == "zryna:capability-profiles/command@0.1.0"
                            && ty == wasmparser::ComponentTypeRef::Component(0) => {}
                    _ => return Err(invalid("encoded world wrapper has a different identity")),
                }
                if wrapper.next().is_some() || world.replace(declarations).is_some() {
                    return Err(invalid("encoded world metadata contains extra declarations"));
                }
            }
        }
    }
    world.ok_or_else(|| invalid("encoded command world type is missing"))
}

fn import_world(
    component: &mut ComponentBuilder,
    declarations: Box<[ComponentTypeDeclaration<'_>]>,
) -> Result<u32, Diagnostic> {
    let mut reencoder = RoundtripReencoder;
    let mut run_type = None;
    for declaration in Vec::from(declarations) {
        match declaration {
            ComponentTypeDeclaration::Type(ty) => {
                let (_, encoder) = component.ty(None);
                reencoder
                    .parse_component_type(encoder, ty)
                    .map_err(|_| invalid("command world type reencoding failed"))?;
            }
            ComponentTypeDeclaration::Alias(alias) => {
                let alias = reencoder
                    .component_alias(alias)
                    .map_err(|_| invalid("command world type alias reencoding failed"))?;
                component.alias(None, alias);
            }
            ComponentTypeDeclaration::Import(import) => {
                let ty = reencoder
                    .component_type_ref(import.ty)
                    .map_err(|_| invalid("command world import type reencoding failed"))?;
                component.import(import.name, ty);
            }
            ComponentTypeDeclaration::Export { name, ty } => match ty {
                wasmparser::ComponentTypeRef::Instance(index)
                    if name.name == RUN_INTERFACE && run_type.replace(index).is_none() => {}
                _ => return Err(invalid("command world has an unexpected export")),
            },
            ComponentTypeDeclaration::CoreType(_) => {
                return Err(invalid("command world unexpectedly declares a core type"));
            }
        }
    }
    run_type.ok_or_else(|| invalid("command run interface is missing"))
}

fn invalid(message: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4011",
        None,
        message,
        "restore the authenticated command world and bounded private bridge",
    )
}
