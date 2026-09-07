//! Admission for fixed, independently authored test fixtures; never a production raw-byte route.

use wasmparser::{
    CanonicalFunction, CanonicalOption, ComponentAlias, ComponentExternalKind, ComponentInstance,
    ComponentTypeRef, Encoding, ExternalKind, Instance, Operator, Parser, Payload, TypeRef,
    Validator, WasmFeatures,
};

pub(super) fn audit(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.len() > 1024 * 1024 {
        return Err("probe byte envelope");
    }
    let mut modules = 0;
    let mut inside = None;
    let mut memory = false;
    let mut realloc = false;
    let mut run = false;
    let mut lower = false;
    let mut lift = false;
    let mut entries = 0;
    let mut imports = 0;
    let mut aliases = 0;
    let mut sections = 0;
    let mut public_instance = false;
    let mut exported = false;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|_| "malformed probe")?;
        if let Some(module) = inside {
            match payload {
                Payload::End(_) => inside = None,
                Payload::Version { encoding: Encoding::Module, .. } => {}
                Payload::TypeSection(types) if types.count() <= 2 => {}
                Payload::FunctionSection(functions) if functions.count() == 1 => {}
                Payload::ImportSection(items) if module == 1 && items.count() == 1 => {
                    for import in items.into_imports() {
                        let import = import.map_err(|_| "probe core import")?;
                        if import.module != "host"
                            || import.name != "deny"
                            || import.ty != TypeRef::Func(0)
                        {
                            return Err("probe core import binding");
                        }
                    }
                }
                Payload::MemorySection(memories)
                    if module == 0 && !memory && memories.count() == 1 =>
                {
                    for ty in memories {
                        let ty = ty.map_err(|_| "probe memory")?;
                        if ty.initial != 1
                            || ty.maximum != Some(1)
                            || ty.memory64
                            || ty.shared
                            || ty.page_size_log2.is_some()
                        {
                            return Err("probe memory exceeds one fixed 64KiB page");
                        }
                    }
                    memory = true;
                }
                Payload::ExportSection(exports) => {
                    if exports.count() != if module == 0 { 2 } else { 1 } {
                        return Err("probe core export count");
                    }
                    for export in exports {
                        let export = export.map_err(|_| "probe core export")?;
                        match (module, export.name, export.kind, export.index) {
                            (0, "memory", ExternalKind::Memory, 0)
                            | (0, "realloc", ExternalKind::Func, 0)
                            | (1, "run", ExternalKind::Func, 1) => {}
                            _ => return Err("probe core export binding"),
                        }
                    }
                }
                Payload::CodeSectionStart { count: 1, .. } => {}
                Payload::CodeSectionEntry(body) => {
                    if body.get_locals_reader().map_err(|_| "probe locals")?.get_count() != 0 {
                        return Err("probe locals");
                    }
                    let mut operators =
                        body.get_operators_reader().map_err(|_| "probe operators")?;
                    while !operators.eof() {
                        match operators.read().map_err(|_| "probe operator")? {
                            Operator::I32Const { .. }
                            | Operator::Call { function_index: 0 }
                            | Operator::Unreachable
                            | Operator::Loop { .. }
                            | Operator::Br { relative_depth: 0 }
                            | Operator::End => {}
                            _ => {
                                return Err(
                                    "probe instruction outside declared call/loop controls",
                                );
                            }
                        }
                    }
                }
                _ => return Err("probe core has extra memory/table/start/section"),
            }
            continue;
        }
        sections += 1;
        if sections > 128 {
            return Err("probe outer payload envelope");
        }
        match payload {
            Payload::Version { encoding: Encoding::Component, .. } | Payload::End(_) => {}
            Payload::ModuleSection { .. } if modules < 2 => {
                inside = Some(modules);
                modules += 1;
            }
            Payload::ComponentTypeSection(types)
                if types.count() <= 64 && types.range().end - types.range().start <= 8192 => {}
            Payload::ComponentImportSection(items) => {
                if items.count() > 2 - imports {
                    return Err("probe interface count");
                }
                for import in items {
                    let import = import.map_err(|_| "probe import")?;
                    if !matches!(
                        import.name.name,
                        "wasi:cli/environment@0.2.12"
                            | "wasi:filesystem/types@0.2.12"
                            | "wasi:filesystem/preopens@0.2.12"
                    ) || !matches!(import.ty, ComponentTypeRef::Instance(_))
                    {
                        return Err("probe interface identity");
                    }
                    imports += 1;
                }
            }
            Payload::InstanceSection(items) => {
                if items.count() > 3 - entries {
                    return Err("probe core index entries");
                }
                for instance in items {
                    match (entries, instance.map_err(|_| "probe instance")?) {
                        (0, Instance::Instantiate { module_index: 0, args }) if args.is_empty() => {
                        }
                        (1, Instance::FromExports(exports))
                            if lower
                                && exports.len() == 1
                                && exports[0].name == "deny"
                                && exports[0].kind == ExternalKind::Func
                                && exports[0].index == 1 => {}
                        (2, Instance::Instantiate { module_index: 1, args })
                            if args.len() == 1 && args[0].name == "host" && args[0].index == 1 => {}
                        _ => {
                            return Err(
                                "probe requires two actual instances and one single-function synthetic binding",
                            );
                        }
                    }
                    entries += 1;
                }
            }
            Payload::ComponentAliasSection(items) => {
                if items.count() > 8 - aliases {
                    return Err("probe alias envelope");
                }
                aliases += items.count();
                for alias in items {
                    match alias.map_err(|_| "probe alias")? {
                        ComponentAlias::CoreInstanceExport {
                            kind: ExternalKind::Memory,
                            instance_index: 0,
                            name: "memory",
                        } if memory => {}
                        ComponentAlias::CoreInstanceExport {
                            kind: ExternalKind::Func,
                            instance_index: 0,
                            name: "realloc",
                        } if !realloc => realloc = true,
                        ComponentAlias::CoreInstanceExport {
                            kind: ExternalKind::Func,
                            instance_index: 2,
                            name: "run",
                        } if !run => run = true,
                        ComponentAlias::InstanceExport {
                            kind: ComponentExternalKind::Func,
                            name: "get-environment",
                            instance_index: 0,
                        }
                        | ComponentAlias::InstanceExport {
                            kind: ComponentExternalKind::Func,
                            name: "get-directories",
                            instance_index: 1,
                        }
                        | ComponentAlias::InstanceExport {
                            kind: ComponentExternalKind::Type,
                            name: "descriptor",
                            instance_index: 0,
                        } => {}
                        _ => return Err("probe alias binding"),
                    }
                }
            }
            Payload::ComponentCanonicalSection(items) if items.count() == 1 => {
                for function in items {
                    match function.map_err(|_| "probe canonical")? {
                        CanonicalFunction::Lower { func_index: 0, options }
                            if !lower
                                && memory
                                && realloc
                                && options.as_ref()
                                    == [
                                        CanonicalOption::UTF8,
                                        CanonicalOption::Memory(0),
                                        CanonicalOption::Realloc(0),
                                    ] =>
                        {
                            lower = true
                        }
                        CanonicalFunction::Lift { core_func_index: 2, options, .. }
                            if !lift && run && options.is_empty() =>
                        {
                            lift = true
                        }
                        _ => return Err("probe canonical options or binding"),
                    }
                }
            }
            Payload::ComponentInstanceSection(items) if !public_instance && items.count() == 1 => {
                for instance in items {
                    match instance.map_err(|_| "probe public instance")? {
                        ComponentInstance::FromExports(exports)
                            if lift
                                && exports.len() == 1
                                && exports[0].name.name == "run"
                                && exports[0].kind == ComponentExternalKind::Func
                                && exports[0].index == 1 =>
                        {
                            public_instance = true
                        }
                        _ => return Err("probe public function binding"),
                    }
                }
            }
            Payload::ComponentExportSection(items) if !exported && items.count() == 1 => {
                for export in items {
                    let export = export.map_err(|_| "probe export")?;
                    if !public_instance
                        || export.name.name != "wasi:cli/run@0.2.12"
                        || export.kind != ComponentExternalKind::Instance
                        || export.index != imports
                        || export.ty.is_some()
                    {
                        return Err("probe run export");
                    }
                    exported = true;
                }
            }
            _ => return Err("probe component contains an extra or unsupported section"),
        }
    }
    if modules != 2 || entries != 3 || !memory || !lower || !lift || !exported {
        return Err("probe graph incomplete");
    }
    Validator::new_with_features(WasmFeatures::WASM1.union(WasmFeatures::COMPONENT_MODEL))
        .validate_all(bytes)
        .map_err(|_| "probe fails complete binary and component type validation")?;
    Ok(())
}
