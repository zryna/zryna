//! Scalar core sealing and its unchanged import-free I32V1 instruction audit.

use wasmparser::{Encoding, ExternalKind, Operator, Parser, Payload, Validator, WasmFeatures};
use zryna_diagnostics::Diagnostic;

use crate::ValidatedWebAssemblyArtifact;

pub(super) fn seal(bytes: Vec<u8>) -> Result<ValidatedWebAssemblyArtifact, Diagnostic> {
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(&bytes)
        .map_err(validation_error)?;
    audit_profile(&bytes)?;
    Ok(ValidatedWebAssemblyArtifact { bytes })
}

pub(super) fn audit_profile(bytes: &[u8]) -> Result<(), Diagnostic> {
    let mut saw_module_version = false;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(validation_error)?;
        match payload {
            Payload::Version { encoding: Encoding::Module, .. } if !saw_module_version => {
                saw_module_version = true;
            }
            Payload::TypeSection(types) => {
                for function_type in types.into_iter_err_on_gc_types() {
                    let function_type = function_type.map_err(validation_error)?;
                    if !function_type.params().iter().all(|ty| *ty == wasmparser::ValType::I32)
                        || function_type.results() != [wasmparser::ValType::I32]
                    {
                        return Err(profile_error("a function type outside I32V1"));
                    }
                }
            }
            Payload::FunctionSection(_) | Payload::CodeSectionStart { .. } | Payload::End(_) => {}
            Payload::ExportSection(exports) => {
                for export in exports {
                    if export.map_err(validation_error)?.kind != ExternalKind::Func {
                        return Err(profile_error("a non-function export"));
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                if body.get_locals_reader().map_err(validation_error)?.get_count() != 0 {
                    return Err(profile_error("function-local declarations"));
                }
                let mut operators = body.get_operators_reader().map_err(validation_error)?;
                while !operators.eof() {
                    match operators.read().map_err(validation_error)? {
                        Operator::LocalGet { .. }
                        | Operator::I32Const { .. }
                        | Operator::I32Add
                        | Operator::End => {}
                        _ => return Err(profile_error("an instruction outside I32V1")),
                    }
                }
            }
            _ => return Err(profile_error("a section outside the import-free I32V1 profile")),
        }
    }
    if !saw_module_version {
        return Err(profile_error("a missing core-module header"));
    }
    Ok(())
}

fn validation_error(error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W1003",
        None,
        format!("emitted core WebAssembly failed pinned WebAssembly 1.0 validation: {error}"),
        "report this compiler failure with the smallest reproducible Zryna source",
    )
}

fn profile_error(capability: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W1004",
        None,
        format!("core WebAssembly contains {capability}"),
        "emit only deterministic import-free I32V1 functions and exports",
    )
}
