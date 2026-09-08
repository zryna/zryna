//! Independently recognize the private bridge's signature and complete instruction stream.

use wasmparser::{
    Encoding, ExternalKind, FunctionBody, Operator, Parser, Payload, TypeRef, ValType, Validator,
    WasmFeatures,
};
use zryna_diagnostics::Diagnostic;

use super::bridge::CommandInvocation;

pub(super) fn audit(bytes: &[u8], invocation: &CommandInvocation) -> Result<(), Diagnostic> {
    if bytes.len() > 1024 {
        return Err(invalid("command bridge exceeds 1024 bytes"));
    }
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(bytes)
        .map_err(|_| invalid("command bridge is not valid core WebAssembly 1.0"))?;
    let mut sections = [false; 5];
    let mut body_seen = false;
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(malformed)? {
            Payload::Version { encoding: Encoding::Module, .. } | Payload::End(_) => {}
            Payload::TypeSection(types) if !sections[0] && types.count() == 2 => {
                sections[0] = true;
                for (index, ty) in types.into_iter_err_on_gc_types().enumerate() {
                    let ty = ty.map_err(malformed)?;
                    let count = if index == 0 { invocation.arguments().len() } else { 0 };
                    if ty.params().len() != count
                        || ty.params().iter().any(|ty| *ty != ValType::I32)
                        || ty.results() != [ValType::I32]
                    {
                        return Err(invalid("command bridge function type differs"));
                    }
                }
            }
            Payload::ImportSection(imports) if !sections[1] => {
                sections[1] = true;
                let mut count = 0;
                for import in imports.into_imports() {
                    let import = import.map_err(malformed)?;
                    count += 1;
                    if count != 1
                        || import.module != "scalar-core"
                        || import.name != invocation.export()
                        || import.ty != TypeRef::Func(0)
                    {
                        return Err(invalid("command bridge scalar import differs"));
                    }
                }
                if count != 1 {
                    return Err(invalid("command bridge scalar import is missing"));
                }
            }
            Payload::FunctionSection(functions) if !sections[2] && functions.count() == 1 => {
                sections[2] = true;
                for ty in functions {
                    if ty.map_err(malformed)? != 1 {
                        return Err(invalid("command bridge run type differs"));
                    }
                }
            }
            Payload::ExportSection(exports) if !sections[3] && exports.count() == 1 => {
                sections[3] = true;
                for export in exports {
                    let export = export.map_err(malformed)?;
                    if export.name != "run"
                        || export.kind != ExternalKind::Func
                        || export.index != 1
                    {
                        return Err(invalid("command bridge run export differs"));
                    }
                }
            }
            Payload::CodeSectionStart { count: 1, .. } if !sections[4] => sections[4] = true,
            Payload::CodeSectionEntry(body) if !body_seen => {
                body_seen = true;
                audit_body(&body, invocation)?;
            }
            _ => {
                return Err(invalid(
                    "command bridge has an extra section or unsupported construct",
                ));
            }
        }
    }
    if !body_seen || sections.iter().any(|seen| !seen) {
        return Err(invalid("command bridge is incomplete"));
    }
    Ok(())
}

fn audit_body(body: &FunctionBody<'_>, invocation: &CommandInvocation) -> Result<(), Diagnostic> {
    if body.get_locals_reader().map_err(malformed)?.get_count() != 0 {
        return Err(invalid("command bridge declares unexpected locals"));
    }
    let mut operators = body.get_operators_reader().map_err(malformed)?;
    for expected in invocation.arguments() {
        match operators.read().map_err(malformed)? {
            Operator::I32Const { value } if value == *expected => {}
            _ => return Err(invalid("command bridge argument instructions differ")),
        }
    }
    if !matches!(operators.read().map_err(malformed)?, Operator::Call { function_index: 0 }) {
        return Err(invalid("command bridge does not call its retained scalar export"));
    }
    match operators.read().map_err(malformed)? {
        Operator::I32Const { value } if value == invocation.expected() => {}
        _ => return Err(invalid("command bridge expected value differs")),
    }
    if !matches!(operators.read().map_err(malformed)?, Operator::I32Ne)
        || !matches!(operators.read().map_err(malformed)?, Operator::End)
        || !operators.eof()
    {
        return Err(invalid("command bridge comparison or termination differs"));
    }
    Ok(())
}

fn malformed(_: wasmparser::BinaryReaderError) -> Diagnostic {
    invalid("command bridge syntax is malformed")
}

fn invalid(message: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4014",
        None,
        message,
        "rebuild the private bridge from the matching verified scalar invocation",
    )
}
