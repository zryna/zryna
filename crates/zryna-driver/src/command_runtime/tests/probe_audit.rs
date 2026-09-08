//! Admission for fixed, independently authored test fixtures; never a production raw-byte route.

use wasmparser::{
    CanonicalFunction, CanonicalOption, ComponentAlias, ComponentAliasSectionReader,
    ComponentCanonicalSectionReader, ComponentExportSectionReader, ComponentExternalKind,
    ComponentImportSectionReader, ComponentInstance, ComponentInstanceSectionReader,
    ComponentTypeRef, Encoding, ExportSectionReader, ExternalKind, FunctionBody,
    ImportSectionReader, Instance, InstanceSectionReader, MemorySectionReader, Operator, Parser,
    Payload, TypeRef, Validator, WasmFeatures,
};

const MEMORY: u16 = 1;
const REALLOC: u16 = 2;
const RUN: u16 = 4;
const LOWER: u16 = 8;
const LIFT: u16 = 16;
const PUBLIC_INSTANCE: u16 = 32;
const EXPORTED: u16 = 64;

pub(super) fn audit(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.len() > 1024 * 1024 {
        return Err("probe byte envelope");
    }
    let mut audit = ProbeAudit::default();
    for payload in Parser::new(0).parse_all(bytes) {
        audit.payload(payload.map_err(|_| "malformed probe")?)?;
    }
    audit.finish(bytes)
}

#[derive(Default)]
struct ProbeAudit {
    modules: u32,
    inside: Option<u32>,
    progress: u16,
    entries: u32,
    imports: u32,
    aliases: u32,
    sections: u32,
}

impl ProbeAudit {
    fn payload(&mut self, payload: Payload<'_>) -> Result<(), &'static str> {
        if let Some(module) = self.inside {
            return self.core_payload(module, payload);
        }
        self.sections += 1;
        if self.sections > 128 {
            return Err("probe outer payload envelope");
        }
        match payload {
            Payload::Version { encoding: Encoding::Component, .. } | Payload::End(_) => Ok(()),
            Payload::ModuleSection { .. } if self.modules < 2 => {
                self.inside = Some(self.modules);
                self.modules += 1;
                Ok(())
            }
            Payload::ComponentTypeSection(types)
                if types.count() <= 64 && types.range().end - types.range().start <= 8192 =>
            {
                Ok(())
            }
            Payload::ComponentImportSection(items) => self.component_imports(items),
            Payload::InstanceSection(items) => self.core_instances(items),
            Payload::ComponentAliasSection(items) => self.aliases(items),
            Payload::ComponentCanonicalSection(items) if items.count() == 1 => {
                self.canonical(items)
            }
            Payload::ComponentInstanceSection(items)
                if !self.has(PUBLIC_INSTANCE) && items.count() == 1 =>
            {
                self.public_instance(items)
            }
            Payload::ComponentExportSection(items) if !self.has(EXPORTED) && items.count() == 1 => {
                self.component_exports(items)
            }
            _ => Err("probe component contains an extra or unsupported section"),
        }
    }

    fn core_payload(&mut self, module: u32, payload: Payload<'_>) -> Result<(), &'static str> {
        match payload {
            Payload::End(_) => self.inside = None,
            Payload::Version { encoding: Encoding::Module, .. }
            | Payload::TypeSection(_)
            | Payload::FunctionSection(_)
            | Payload::CodeSectionStart { count: 1, .. } => {}
            Payload::ImportSection(items) if module == 1 && items.count() == 1 => {
                Self::core_imports(items)?;
            }
            Payload::MemorySection(memories)
                if module == 0 && !self.has(MEMORY) && memories.count() == 1 =>
            {
                self.memory(memories)?;
            }
            Payload::ExportSection(exports) => Self::core_exports(module, exports)?,
            Payload::CodeSectionEntry(body) => Self::body(&body)?,
            _ => return Err("probe core has extra memory/table/start/section"),
        }
        Ok(())
    }

    fn core_imports(items: ImportSectionReader<'_>) -> Result<(), &'static str> {
        for import in items.into_imports() {
            let import = import.map_err(|_| "probe core import")?;
            if import.module != "host" || import.name != "deny" || import.ty != TypeRef::Func(0) {
                return Err("probe core import binding");
            }
        }
        Ok(())
    }

    fn memory(&mut self, memories: MemorySectionReader<'_>) -> Result<(), &'static str> {
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
        self.mark(MEMORY);
        Ok(())
    }

    fn core_exports(module: u32, exports: ExportSectionReader<'_>) -> Result<(), &'static str> {
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
        Ok(())
    }

    fn body(body: &FunctionBody<'_>) -> Result<(), &'static str> {
        if body.get_locals_reader().map_err(|_| "probe locals")?.get_count() != 0 {
            return Err("probe locals");
        }
        let mut operators = body.get_operators_reader().map_err(|_| "probe operators")?;
        while !operators.eof() {
            match operators.read().map_err(|_| "probe operator")? {
                Operator::I32Const { .. }
                | Operator::Call { function_index: 0 }
                | Operator::Unreachable
                | Operator::Loop { .. }
                | Operator::Br { relative_depth: 0 }
                | Operator::End => {}
                _ => return Err("probe instruction outside declared call/loop controls"),
            }
        }
        Ok(())
    }

    fn component_imports(
        &mut self,
        items: ComponentImportSectionReader<'_>,
    ) -> Result<(), &'static str> {
        if items.count() > 2 - self.imports {
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
            self.imports += 1;
        }
        Ok(())
    }

    fn core_instances(&mut self, items: InstanceSectionReader<'_>) -> Result<(), &'static str> {
        if items.count() > 3 - self.entries {
            return Err("probe core index entries");
        }
        for instance in items {
            match (self.entries, instance.map_err(|_| "probe instance")?) {
                (0, Instance::Instantiate { module_index: 0, args }) if args.is_empty() => {}
                (1, Instance::FromExports(exports))
                    if self.has(LOWER)
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
            self.entries += 1;
        }
        Ok(())
    }

    fn aliases(&mut self, items: ComponentAliasSectionReader<'_>) -> Result<(), &'static str> {
        if items.count() > 8 - self.aliases {
            return Err("probe alias envelope");
        }
        self.aliases += items.count();
        for alias in items {
            match alias.map_err(|_| "probe alias")? {
                ComponentAlias::CoreInstanceExport {
                    kind: ExternalKind::Memory,
                    instance_index: 0,
                    name: "memory",
                } if self.has(MEMORY) => {}
                ComponentAlias::CoreInstanceExport {
                    kind: ExternalKind::Func,
                    instance_index: 0,
                    name: "realloc",
                } if !self.has(REALLOC) => self.mark(REALLOC),
                ComponentAlias::CoreInstanceExport {
                    kind: ExternalKind::Func,
                    instance_index: 2,
                    name: "run",
                } if !self.has(RUN) => self.mark(RUN),
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
        Ok(())
    }

    fn canonical(
        &mut self,
        items: ComponentCanonicalSectionReader<'_>,
    ) -> Result<(), &'static str> {
        for function in items {
            match function.map_err(|_| "probe canonical")? {
                CanonicalFunction::Lower { func_index: 0, options }
                    if !self.has(LOWER)
                        && self.has(MEMORY)
                        && self.has(REALLOC)
                        && options.as_ref()
                            == [
                                CanonicalOption::UTF8,
                                CanonicalOption::Memory(0),
                                CanonicalOption::Realloc(0),
                            ] =>
                {
                    self.mark(LOWER);
                }
                CanonicalFunction::Lift { core_func_index: 2, options, .. }
                    if !self.has(LIFT) && self.has(RUN) && options.is_empty() =>
                {
                    self.mark(LIFT);
                }
                _ => return Err("probe canonical options or binding"),
            }
        }
        Ok(())
    }

    fn public_instance(
        &mut self,
        items: ComponentInstanceSectionReader<'_>,
    ) -> Result<(), &'static str> {
        for instance in items {
            match instance.map_err(|_| "probe public instance")? {
                ComponentInstance::FromExports(exports)
                    if self.has(LIFT)
                        && exports.len() == 1
                        && exports[0].name.name == "run"
                        && exports[0].kind == ComponentExternalKind::Func
                        && exports[0].index == 1 =>
                {
                    self.mark(PUBLIC_INSTANCE);
                }
                _ => return Err("probe public function binding"),
            }
        }
        Ok(())
    }

    fn component_exports(
        &mut self,
        items: ComponentExportSectionReader<'_>,
    ) -> Result<(), &'static str> {
        for export in items {
            let export = export.map_err(|_| "probe export")?;
            if !self.has(PUBLIC_INSTANCE)
                || export.name.name != "wasi:cli/run@0.2.12"
                || export.kind != ComponentExternalKind::Instance
                || export.index != self.imports
                || export.ty.is_some()
            {
                return Err("probe run export");
            }
            self.mark(EXPORTED);
        }
        Ok(())
    }

    fn finish(self, bytes: &[u8]) -> Result<(), &'static str> {
        if self.modules != 2
            || self.entries != 3
            || !self.has(MEMORY)
            || !self.has(LOWER)
            || !self.has(LIFT)
            || !self.has(EXPORTED)
        {
            return Err("probe graph incomplete");
        }
        Validator::new_with_features(WasmFeatures::WASM1.union(WasmFeatures::COMPONENT_MODEL))
            .validate_all(bytes)
            .map_err(|_| "probe fails complete binary and component type validation")?;
        Ok(())
    }

    const fn has(&self, phase: u16) -> bool {
        self.progress & phase != 0
    }

    fn mark(&mut self, phase: u16) {
        self.progress |= phase;
    }
}
