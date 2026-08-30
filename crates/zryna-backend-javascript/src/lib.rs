//! Direct JavaScript emission from verified Zryna IR.

#![forbid(unsafe_code)]

use std::fmt::Write;

use zryna_diagnostics::Diagnostic;
use zryna_ir::{ExprKind, Type, VerifiedFunction, VerifiedProgram};

const JAVASCRIPT_PRELUDE: &str = r#"function $zryna$checkArity($zryna$actual, $zryna$expected) {
  if ($zryna$actual !== $zryna$expected) {
    throw new TypeError("ZRYNA-B2102: scalar ABI arity mismatch");
  }
}

function $zryna$i32($zryna$value) {
  if (typeof $zryna$value !== "number") {
    throw new TypeError("ZRYNA-B2001: expected a primitive JavaScript Number");
  }
  if (!Number.isInteger($zryna$value) || $zryna$value < -2147483648 || $zryna$value > 2147483647 || Object.is($zryna$value, -0)) {
    throw new RangeError("ZRYNA-B2002: expected a canonical signed 32-bit integer");
  }
  return $zryna$value;
}

function $zryna$bool($zryna$value) {
  if (typeof $zryna$value !== "boolean") {
    throw new TypeError("ZRYNA-B2001: expected a primitive JavaScript Boolean");
  }
  return $zryna$value;
}

"#;

/// JavaScript artifacts produced by one compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaScriptArtifact {
    /// ECMAScript module source.
    pub source: String,
}

/// Emits modern ECMAScript from verified Zryna IR.
///
/// Raw Universal IR is not accepted by this boundary:
///
/// ```compile_fail
/// let raw = zryna_ir::Program::default();
/// let _ = zryna_backend_javascript::emit(&raw);
/// ```
///
/// # Errors
///
/// Returns a compiler diagnostic when a verified expression cannot be emitted.
pub fn emit(program: &VerifiedProgram) -> Result<JavaScriptArtifact, Diagnostic> {
    let mut output = String::new();
    if program.functions().len() == 0 {
        output.push_str("export {};\n");
        return Ok(JavaScriptArtifact { source: output });
    }

    output.push_str(JAVASCRIPT_PRELUDE);
    for (index, function) in program.functions().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        emit_function(function, &mut output)?;
    }
    Ok(JavaScriptArtifact { source: output })
}

fn emit_function(function: VerifiedFunction<'_>, output: &mut String) -> Result<(), Diagnostic> {
    write!(output, "export function {}(", function.abi_export().javascript_name().as_str())
        .map_err(|error| formatting_error(function, error))?;
    for index in 0..function.parameters().len() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "p{index}").map_err(|error| formatting_error(function, error))?;
    }
    output.push_str(") {\n");
    writeln!(output, "  $zryna$checkArity(arguments.length, {});", function.parameters().len())
        .map_err(|error| formatting_error(function, error))?;
    for (index, parameter) in function.parameters().iter().enumerate() {
        if *parameter != Type::I32 {
            return Err(profile_invariant_error(function));
        }
        writeln!(output, "  p{index} = $zryna$i32(p{index});")
            .map_err(|error| formatting_error(function, error))?;
    }
    for (index, expression) in function.expressions().iter().enumerate() {
        write!(output, "  const v{index} = ").map_err(|error| formatting_error(function, error))?;
        match &expression.kind {
            ExprKind::Parameter(parameter) => {
                write!(output, "p{parameter}")
                    .map_err(|error| formatting_error(function, error))?;
            }
            ExprKind::BoolLiteral(_) => {
                return Err(profile_invariant_error(function));
            }
            ExprKind::I32Add { lhs, rhs } => {
                write!(output, "(v{} + v{}) | 0", lhs.0, rhs.0)
                    .map_err(|error| formatting_error(function, error))?;
            }
            ExprKind::I32Literal(value) => {
                write!(output, "{value}").map_err(|error| formatting_error(function, error))?;
            }
        }
        output.push_str(";\n");
    }
    if function.return_type() != Type::I32 {
        return Err(profile_invariant_error(function));
    }
    write!(output, "  return $zryna$i32(v{});\n}}\n", function.body().0)
        .map_err(|error| formatting_error(function, error))?;
    Ok(())
}

fn profile_invariant_error(function: VerifiedFunction<'_>) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-J1001",
        None,
        format!(
            "verified function '{}' contains an operation outside the JavaScript proof profile",
            function.abi_export().javascript_name().as_str()
        ),
        "report this compiler invariant failure with the smallest reproducible Zryna source",
    )
}

fn formatting_error(function: VerifiedFunction<'_>, error: std::fmt::Error) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-J1002",
        None,
        format!(
            "could not emit JavaScript for '{}': {error}",
            function.abi_export().javascript_name().as_str()
        ),
        "report this compiler failure with the smallest reproducible Zryna source",
    )
}

/// Returns the JavaScript spelling associated with an IR type.
///
/// This representation helper does not admit a type into the current universal profile.
#[must_use]
pub const fn javascript_type(ty: Type) -> &'static str {
    match ty {
        Type::Unit => "undefined",
        Type::Bool => "boolean",
        Type::I32 => "number",
    }
}

#[cfg(test)]
mod tests {
    use zryna_ir::{
        Expr, ExprId, ExprKind, Function, MAX_IR_EXPRESSION_DEPTH, Program, Type, verify,
    };
    use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};

    #[test]
    fn emits_wrapping_i32_addition() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/add.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/add.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must be valid");
        let program = Program {
            functions: vec![Function {
                name: "add".to_owned(),
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
            }],
        };
        let Ok(verified) = verify(program, &sources) else {
            panic!("test IR must be valid");
        };
        let Ok(artifact) = super::emit(&verified) else {
            panic!("JavaScript emission must succeed");
        };
        assert_eq!(
            artifact.source,
            concat!(
                "function $zryna$checkArity($zryna$actual, $zryna$expected) {\n",
                "  if ($zryna$actual !== $zryna$expected) {\n",
                "    throw new TypeError(\"ZRYNA-B2102: scalar ABI arity mismatch\");\n",
                "  }\n",
                "}\n",
                "\n",
                "function $zryna$i32($zryna$value) {\n",
                "  if (typeof $zryna$value !== \"number\") {\n",
                "    throw new TypeError(\"ZRYNA-B2001: expected a primitive JavaScript Number\");\n",
                "  }\n",
                "  if (!Number.isInteger($zryna$value) || $zryna$value < -2147483648 || $zryna$value > 2147483647 || Object.is($zryna$value, -0)) {\n",
                "    throw new RangeError(\"ZRYNA-B2002: expected a canonical signed 32-bit integer\");\n",
                "  }\n",
                "  return $zryna$value;\n",
                "}\n",
                "\n",
                "function $zryna$bool($zryna$value) {\n",
                "  if (typeof $zryna$value !== \"boolean\") {\n",
                "    throw new TypeError(\"ZRYNA-B2001: expected a primitive JavaScript Boolean\");\n",
                "  }\n",
                "  return $zryna$value;\n",
                "}\n",
                "\n",
                "export function add(p0, p1) {\n",
                "  $zryna$checkArity(arguments.length, 2);\n",
                "  p0 = $zryna$i32(p0);\n",
                "  p1 = $zryna$i32(p1);\n",
                "  const v0 = p0;\n",
                "  const v1 = p1;\n",
                "  const v2 = (v0 + v1) | 0;\n",
                "  return $zryna$i32(v2);\n",
                "}\n",
            )
        );
    }

    #[test]
    fn emits_empty_program_as_deterministic_esm() {
        let sources = SourceMap::build(Vec::new()).expect("empty source map must be valid");
        let verified = verify(Program::default(), &sources).expect("empty IR must verify");
        let first = super::emit(&verified).expect("empty JavaScript module must emit");
        let second = super::emit(&verified).expect("repeated empty module must emit");

        assert_eq!(first, second);
        assert_eq!(first.source, "export {};\n");
    }

    #[test]
    fn emission_is_byte_deterministic_and_uses_collision_proof_private_names() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/names.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/names.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must be valid");
        let function = |name: &str, value: i32| Function {
            name: name.to_owned(),
            parameters: Vec::new(),
            return_type: Type::I32,
            expressions: vec![Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(value) }],
            body: ExprId(0),
        };
        let verified = verify(
            Program { functions: vec![function("zryna_i32", i32::MIN), function("_x", i32::MAX)] },
            &sources,
        )
        .expect("name fixture must verify");

        let first = super::emit(&verified).expect("name fixture must emit");
        let second = super::emit(&verified).expect("repeated name fixture must emit");

        assert_eq!(first, second);
        assert!(!first.source.contains('\r'));
        assert!(first.source.ends_with('\n'));
        assert_eq!(first.source.matches("function $zryna$i32").count(), 1);
        assert_eq!(first.source.matches("function $zryna$bool").count(), 1);
        assert!(first.source.contains("export function zryna_i32()"));
        assert!(first.source.contains("export function _x()"));
        assert!(
            first.source.find("zryna_i32()").expect("first export")
                < first.source.find("_x()").expect("second export")
        );
        assert!(first.source.contains("const v0 = -2147483648;"));
        assert!(first.source.contains("const v0 = 2147483647;"));
    }

    #[test]
    fn bool_remains_outside_the_verified_backend_profile() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/bool.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/bool.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must be valid");
        let program = Program {
            functions: vec![Function {
                name: "flag".to_owned(),
                parameters: Vec::new(),
                return_type: Type::Bool,
                expressions: vec![Expr { ty: Type::Bool, span, kind: ExprKind::BoolLiteral(true) }],
                body: ExprId(0),
            }],
        };

        assert!(verify(program, &sources).is_err());
    }

    #[test]
    fn emits_the_max_depth_tree_iteratively_and_linearly() {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/deep.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map must be valid");
        let path = NormalizedSourcePath::new("src/deep.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");
        let span = sources.span(file, 0, 1).expect("fixture span must be valid");
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
        let expression_count = expressions.len();
        let program = Program {
            functions: vec![Function {
                name: "deepValue".to_owned(),
                parameters: Vec::new(),
                return_type: Type::I32,
                expressions,
                body: root,
            }],
        };
        let verified = verify(program, &sources).expect("maximum-depth IR must verify");
        let artifact = super::emit(&verified).expect("maximum-depth JavaScript must emit");
        assert_eq!(artifact.source.matches("  const v").count(), expression_count);
        assert!(artifact.source.len() < expression_count * 64);
    }
}
