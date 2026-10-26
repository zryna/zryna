//! Actual-byte capability audit for the legacy `ControlFlowV1` core profile.

use super::ControlFlowLayout;
use crate::emission_diagnostics::{control_flow_audit_error, control_flow_validation_error};
use wasmparser::{Encoding, ExternalKind, Operator, Parser, Payload};
use zryna_diagnostics::Diagnostic;

#[allow(clippy::too_many_lines)]
pub(super) fn audit_control_flow_profile(
    bytes: &[u8],
    expected: &ControlFlowLayout<'_>,
) -> Result<(), Diagnostic> {
    let mut saw_version = false;
    let mut saw_types = false;
    let mut saw_functions = false;
    let mut saw_exports = false;
    let mut saw_code_start = false;
    let mut code_index = 0_usize;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(control_flow_validation_error)?;
        match payload {
            Payload::Version { encoding: Encoding::Module, .. } if !saw_version => {
                saw_version = true;
            }
            Payload::TypeSection(types) if !saw_types => {
                saw_types = true;
                let mut actual = Vec::new();
                for function_type in types.into_iter_err_on_gc_types() {
                    let function_type = function_type.map_err(control_flow_validation_error)?;
                    if !function_type.params().iter().all(|ty| *ty == wasmparser::ValType::I32)
                        || function_type.results() != [wasmparser::ValType::I32]
                    {
                        return Err(control_flow_audit_error("a non-scalar function type"));
                    }
                    actual.push(function_type.params().len());
                }
                if actual != expected.type_parameter_counts {
                    return Err(control_flow_audit_error("a noncanonical type index inventory"));
                }
            }
            Payload::FunctionSection(functions) if !saw_functions => {
                saw_functions = true;
                let actual = functions
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(control_flow_validation_error)?;
                if actual != expected.function_type_indices {
                    return Err(control_flow_audit_error(
                        "a noncanonical function index inventory",
                    ));
                }
            }
            Payload::ExportSection(exports) if !saw_exports => {
                saw_exports = true;
                let mut actual = Vec::new();
                for export in exports {
                    let export = export.map_err(control_flow_validation_error)?;
                    if export.kind != ExternalKind::Func {
                        return Err(control_flow_audit_error("a non-function export"));
                    }
                    actual.push((export.name.to_owned(), export.index));
                }
                if actual != expected.exports {
                    return Err(control_flow_audit_error(
                        "an export outside sealed public authority",
                    ));
                }
            }
            Payload::CodeSectionStart { count, .. } if !saw_code_start => {
                saw_code_start = true;
                if usize::try_from(count).ok() != Some(expected.functions.len()) {
                    return Err(control_flow_audit_error("a mismatched code body inventory"));
                }
            }
            Payload::CodeSectionEntry(body) => {
                let expected_locals = expected
                    .local_counts
                    .get(code_index)
                    .copied()
                    .ok_or_else(|| control_flow_audit_error("an extra code body"))?;
                code_index += 1;
                let mut actual_locals = 0_u32;
                for local in body.get_locals_reader().map_err(control_flow_validation_error)? {
                    let (count, ty) = local.map_err(control_flow_validation_error)?;
                    if ty != wasmparser::ValType::I32 {
                        return Err(control_flow_audit_error("a non-i32 local declaration"));
                    }
                    actual_locals = actual_locals.checked_add(count).ok_or_else(|| {
                        control_flow_audit_error("an overflowing local inventory")
                    })?;
                }
                if actual_locals != expected_locals {
                    return Err(control_flow_audit_error("a noncanonical local inventory"));
                }
                let mut operators =
                    body.get_operators_reader().map_err(control_flow_validation_error)?;
                while !operators.eof() {
                    match operators.read().map_err(control_flow_validation_error)? {
                        Operator::Unreachable
                        | Operator::Block { .. }
                        | Operator::Loop { .. }
                        | Operator::If { .. }
                        | Operator::Else
                        | Operator::End
                        | Operator::Br { .. }
                        | Operator::BrIf { .. }
                        | Operator::Return
                        | Operator::Call { .. }
                        | Operator::LocalGet { .. }
                        | Operator::LocalSet { .. }
                        | Operator::I32Const { .. }
                        | Operator::I32Eq
                        | Operator::I32Ne
                        | Operator::I32LtS
                        | Operator::I32GtS
                        | Operator::I32LeS
                        | Operator::I32GeS
                        | Operator::I32Add
                        | Operator::I32Sub
                        | Operator::I32Mul
                        | Operator::I32Or => {}
                        _ => {
                            return Err(control_flow_audit_error(
                                "an instruction outside the sealed M2 profile",
                            ));
                        }
                    }
                }
            }
            Payload::End(_) => {}
            _ => {
                return Err(control_flow_audit_error(
                    "a section outside the capability-minimal M2 profile",
                ));
            }
        }
    }
    if !saw_version {
        return Err(control_flow_audit_error("a missing core-module header"));
    }
    if expected.functions.is_empty() {
        if saw_types || saw_functions || saw_exports || saw_code_start || code_index != 0 {
            return Err(control_flow_audit_error("sections in an empty module"));
        }
    } else if !saw_types
        || !saw_functions
        || !saw_exports
        || !saw_code_start
        || code_index != expected.functions.len()
    {
        return Err(control_flow_audit_error("a missing canonical M2 section or code body"));
    }
    Ok(())
}
