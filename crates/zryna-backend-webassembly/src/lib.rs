//! Deterministic core WebAssembly emission from verified Zryna IR.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction, Module,
    TypeSection, ValType,
};
use wasmparser::{Encoding, ExternalKind, Operator, Parser, Payload, Validator, WasmFeatures};
use zryna_diagnostics::Diagnostic;
use zryna_ir::{ExprKind, Type, VerifiedFunction, VerifiedProgram};

/// A complete core WebAssembly module that passed the pinned validator and the Zryna profile audit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedWebAssemblyArtifact {
    bytes: Vec<u8>,
}

impl ValidatedWebAssemblyArtifact {
    /// Returns the exact validated module bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Emits deterministic, import-free core WebAssembly from the current `I32V1` profile.
///
/// Raw Universal IR cannot enter this boundary:
///
/// ```compile_fail
/// let raw = zryna_ir::Program::default();
/// let _ = zryna_backend_webassembly::emit(&raw);
/// ```
///
/// # Errors
///
/// Returns a stable compiler diagnostic if a verified-program invariant cannot be encoded or if
/// the exact emitted bytes fail WebAssembly 1.0 validation or the narrower Zryna capability audit.
pub fn emit(program: &VerifiedProgram) -> Result<ValidatedWebAssemblyArtifact, Diagnostic> {
    let functions = program.functions().collect::<Vec<_>>();
    if functions.is_empty() {
        return seal(Module::new().finish());
    }

    let mut types = TypeSection::new();
    let mut type_indices = BTreeMap::<usize, u32>::new();
    let mut next_type_index = 0_u32;
    let mut function_type_indices = Vec::with_capacity(functions.len());
    for function in &functions {
        verify_signature(*function)?;
        let parameter_count = function.parameters().len();
        let type_index = if let Some(index) = type_indices.get(&parameter_count) {
            *index
        } else {
            let parameters = std::iter::repeat_n(ValType::I32, parameter_count);
            types.ty().function(parameters, [ValType::I32]);
            let index = next_type_index;
            next_type_index = next_type_index.checked_add(1).ok_or_else(index_error)?;
            type_indices.insert(parameter_count, index);
            index
        };
        function_type_indices.push(type_index);
    }

    let mut function_section = FunctionSection::new();
    let mut export_section = ExportSection::new();
    let mut code_section = CodeSection::new();
    for (function_index, (function, type_index)) in
        functions.iter().zip(function_type_indices).enumerate()
    {
        function_section.function(type_index);
        let function_index = u32::try_from(function_index).map_err(|_| index_error())?;
        export_section.export(
            function.abi_export().webassembly_name().as_str(),
            ExportKind::Func,
            function_index,
        );
        code_section.function(&encode_function(*function)?);
    }

    let mut module = Module::new();
    module.section(&types);
    module.section(&function_section);
    module.section(&export_section);
    module.section(&code_section);
    seal(module.finish())
}

fn verify_signature(function: VerifiedFunction<'_>) -> Result<(), Diagnostic> {
    let signature_is_i32 = function.parameters().iter().all(|ty| *ty == Type::I32)
        && function.return_type() == Type::I32;
    if signature_is_i32 { Ok(()) } else { Err(profile_invariant_error(function)) }
}

fn encode_function(function: VerifiedFunction<'_>) -> Result<Function, Diagnostic> {
    let mut body = Function::new(Vec::new());
    for expression in function.expressions() {
        match expression.kind {
            ExprKind::Parameter(index) => {
                body.instruction(&Instruction::LocalGet(index));
            }
            ExprKind::I32Literal(value) => {
                body.instruction(&Instruction::I32Const(value));
            }
            ExprKind::I32Add { .. } => {
                body.instruction(&Instruction::I32Add);
            }
            ExprKind::BoolLiteral(_) => return Err(profile_invariant_error(function)),
        }
    }
    body.instruction(&Instruction::End);
    Ok(body)
}

fn seal(bytes: Vec<u8>) -> Result<ValidatedWebAssemblyArtifact, Diagnostic> {
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(&bytes)
        .map_err(validation_error)?;
    audit_profile(&bytes)?;
    Ok(ValidatedWebAssemblyArtifact { bytes })
}

fn audit_profile(bytes: &[u8]) -> Result<(), Diagnostic> {
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

fn profile_invariant_error(function: VerifiedFunction<'_>) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W1001",
        None,
        format!(
            "verified function '{}' contains a type or operation outside the WebAssembly I32V1 proof profile",
            function.abi_export().webassembly_name().as_str()
        ),
        "report this compiler invariant failure with the smallest reproducible Zryna source",
    )
}

fn index_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W1002",
        None,
        "verified WebAssembly indexes exceeded the core binary index space",
        "report this compiler invariant failure with the smallest reproducible Zryna source",
    )
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

#[cfg(test)]
mod tests {
    use wasmparser::{ExternalKind, Operator, Parser, Payload, Validator, WasmFeatures};
    use zryna_ir::{
        Expr, ExprId, ExprKind, Function, MAX_IR_EXPRESSION_DEPTH, Program, Type, verify,
    };
    use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap, Span};

    fn fixture() -> (SourceMap, Span) {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/module.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/module.zry").expect("fixture path");
        let file = sources.file_id(&path).expect("fixture file");
        let span = sources.span(file, 0, 1).expect("fixture span");
        (sources, span)
    }

    fn add_function(span: Span, name: &str) -> Function {
        Function {
            name: name.to_owned(),
            parameters: vec![Type::I32, Type::I32],
            return_type: Type::I32,
            expressions: vec![
                Expr { ty: Type::I32, span, kind: ExprKind::Parameter(0) },
                Expr { ty: Type::I32, span, kind: ExprKind::Parameter(1) },
                Expr {
                    ty: Type::I32,
                    span,
                    kind: ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) },
                },
            ],
            body: ExprId(2),
        }
    }

    #[test]
    fn emits_exact_minimal_add_module() {
        let (sources, span) = fixture();
        let verified = verify(Program { functions: vec![add_function(span, "add")] }, &sources)
            .expect("add fixture must verify");
        let artifact = super::emit(&verified).expect("add fixture must emit");
        let expected = [
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f,
            0x7f, 0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64,
            0x00, 0x00, 0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b,
        ];
        assert_eq!(artifact.bytes(), expected);
    }

    #[test]
    fn emission_is_byte_deterministic_and_capability_free() {
        let (sources, span) = fixture();
        let verified = verify(
            Program { functions: vec![add_function(span, "add"), add_function(span, "sum")] },
            &sources,
        )
        .expect("fixture must verify");
        let first = super::emit(&verified).expect("fixture must emit");
        let second = super::emit(&verified).expect("repeated fixture must emit");
        assert_eq!(first, second);

        let mut type_count = 0;
        let mut exports = Vec::new();
        for payload in Parser::new(0).parse_all(first.bytes()) {
            match payload.expect("emitted bytes must parse") {
                Payload::TypeSection(reader) => type_count += reader.count(),
                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export.expect("export must parse");
                        assert_eq!(export.kind, ExternalKind::Func);
                        exports.push(export.name.to_owned());
                    }
                }
                Payload::ImportSection(_)
                | Payload::MemorySection(_)
                | Payload::TableSection(_)
                | Payload::GlobalSection(_)
                | Payload::StartSection { .. }
                | Payload::ElementSection(_)
                | Payload::DataSection(_) => panic!("emitter introduced a capability section"),
                _ => {}
            }
        }
        assert_eq!(type_count, 1, "equal signatures must use one stable first-use type");
        assert_eq!(exports, ["add", "sum"]);
    }

    #[test]
    fn signed_leb_i32_boundaries_are_valid_and_exact() {
        let (sources, span) = fixture();
        let values = [0, -1, i32::MIN, i32::MAX];
        let functions = values
            .iter()
            .enumerate()
            .map(|(index, value)| Function {
                name: format!("value{index}"),
                parameters: Vec::new(),
                return_type: Type::I32,
                expressions: vec![Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(*value) }],
                body: ExprId(0),
            })
            .collect();
        let verified =
            verify(Program { functions }, &sources).expect("literal fixture must verify");
        let artifact = super::emit(&verified).expect("literal fixture must emit");
        Validator::new_with_features(WasmFeatures::WASM1)
            .validate_all(artifact.bytes())
            .expect("boundary module must validate");
        let mut encoded_values = Vec::new();
        for payload in Parser::new(0).parse_all(artifact.bytes()) {
            if let Payload::CodeSectionEntry(body) = payload.expect("payload must parse") {
                let mut operators = body.get_operators_reader().expect("operators");
                if let Operator::I32Const { value } = operators.read().expect("literal") {
                    encoded_values.push(value);
                }
            }
        }
        assert_eq!(encoded_values, values);
        assert!(artifact.bytes().windows(5).any(|bytes| bytes == [0x80, 0x80, 0x80, 0x80, 0x78]));
        assert!(artifact.bytes().windows(5).any(|bytes| bytes == [0xff, 0xff, 0xff, 0xff, 0x07]));
    }

    #[test]
    fn maximum_depth_emits_iteratively() {
        let (sources, span) = fixture();
        let mut expressions = vec![Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(1) }];
        let mut root = ExprId(0);
        for _ in 1..MAX_IR_EXPRESSION_DEPTH {
            let leaf = ExprId(u32::try_from(expressions.len()).expect("bounded fixture"));
            expressions.push(Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(1) });
            let parent = ExprId(u32::try_from(expressions.len()).expect("bounded fixture"));
            expressions.push(Expr {
                ty: Type::I32,
                span,
                kind: ExprKind::I32Add { lhs: root, rhs: leaf },
            });
            root = parent;
        }
        let verified = verify(
            Program {
                functions: vec![Function {
                    name: "deepValue".to_owned(),
                    parameters: Vec::new(),
                    return_type: Type::I32,
                    expressions,
                    body: root,
                }],
            },
            &sources,
        )
        .expect("maximum-depth fixture must verify");
        let artifact = super::emit(&verified).expect("maximum-depth fixture must emit");
        let maximum_depth =
            usize::try_from(MAX_IR_EXPRESSION_DEPTH).expect("IR depth limit must fit usize");
        assert!(artifact.bytes().len() < maximum_depth * 8);
    }

    #[test]
    fn validator_and_profile_audit_fail_closed() {
        let malformed = b"not wasm";
        assert_eq!(
            super::seal(malformed.to_vec()).expect_err("bad magic must fail").code(),
            "ZRYNA-W1003"
        );

        let mut module = wasm_encoder::Module::new();
        let mut memories = wasm_encoder::MemorySection::new();
        memories.memory(wasm_encoder::MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memories);
        assert_eq!(
            super::seal(module.finish()).expect_err("memory capability must fail").code(),
            "ZRYNA-W1004"
        );

        let mut module = wasm_encoder::Module::new();
        let mut types = wasm_encoder::TypeSection::new();
        types.ty().function([wasm_encoder::ValType::I64], [wasm_encoder::ValType::I32]);
        module.section(&types);
        assert_eq!(
            super::seal(module.finish()).expect_err("i64 type must fail").code(),
            "ZRYNA-W1004"
        );

        let mut module = wasm_encoder::Module::new();
        let mut types = wasm_encoder::TypeSection::new();
        types.ty().function([wasm_encoder::ValType::V128], [wasm_encoder::ValType::I32]);
        module.section(&types);
        assert_eq!(
            super::seal(module.finish())
                .expect_err("post-MVP SIMD type must fail pinned WebAssembly 1.0 validation")
                .code(),
            "ZRYNA-W1003"
        );
    }

    #[test]
    fn empty_program_is_only_the_core_module_header() {
        let sources = SourceMap::build(Vec::new()).expect("empty source map");
        let verified = verify(Program::default(), &sources).expect("empty program");
        let artifact = super::emit(&verified).expect("empty module");
        assert_eq!(artifact.bytes(), b"\0asm\x01\0\0\0");
    }
}
