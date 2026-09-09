use zryna_frontend::syntax_v2::{
    RawExpressionKind, RawExpressionSyntax, RawFunctionBodySyntax, RawFunctionSyntax,
    RawIdentifierSyntax, RawParameterSyntax, RawProjectSyntaxSnapshot, RawSourceUnit,
    RawStatementKind, RawStatementSyntax, RawTypeSyntax, RawTypeSyntaxKind, verify_snapshot,
};
use zryna_source::{SourceMap, UntrustedSpan};

use super::{DiagnosticRevision, DiagnosticSession, DiagnosticSessionError};

/// Admits the fixed single-function scalar fixture used by transport lifecycle tests.
///
/// # Errors
///
/// Returns an error when the source is not the exact fixture shape or semantic admission fails.
pub fn admit_single_function_fixture(
    session: &mut DiagnosticSession,
    sources: SourceMap,
) -> Result<DiagnosticRevision, DiagnosticSessionError> {
    let file = sources.verify_file_id(0).map_err(|_| DiagnosticSessionError::SourceInvariant)?;
    let source = sources.source(file).ok_or(DiagnosticSessionError::SourceInvariant)?;
    if source.path().as_str() != "src/main.zry" {
        return Err(DiagnosticSessionError::SemanticAuthority);
    }
    let text = source.text();
    let name = text.get(16..17).ok_or(DiagnosticSessionError::SemanticAuthority)?;
    let parameter = text.get(18..19).ok_or(DiagnosticSessionError::SemanticAuthority)?;
    if !name.bytes().all(|byte| byte.is_ascii_lowercase())
        || !parameter.bytes().all(|byte| byte.is_ascii_lowercase())
        || text
            != format!("export function {name}({parameter}: i32): i32 {{ return {parameter}; }}\n")
    {
        return Err(DiagnosticSessionError::SemanticAuthority);
    }
    let span = |start, end| UntrustedSpan { file: 0, start, end };
    let raw = RawProjectSyntaxSnapshot {
        schema_version: 2,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            functions: vec![RawFunctionSyntax {
                span: span(0, 44),
                export_span: span(0, 6),
                function_span: span(7, 15),
                name: RawIdentifierSyntax { text: name.to_owned(), span: span(16, 17) },
                parameters: vec![RawParameterSyntax {
                    span: span(18, 24),
                    name: RawIdentifierSyntax { text: parameter.to_owned(), span: span(18, 19) },
                    type_syntax: RawTypeSyntax {
                        span: span(21, 24),
                        kind: RawTypeSyntaxKind::Named { name: "i32".to_owned() },
                    },
                }],
                result_type: RawTypeSyntax {
                    span: span(27, 30),
                    kind: RawTypeSyntaxKind::Named { name: "i32".to_owned() },
                },
                body: RawFunctionBodySyntax {
                    span: span(31, 44),
                    statements: vec![RawStatementSyntax {
                        span: span(33, 42),
                        kind: RawStatementKind::Return { keyword_span: span(33, 39), value: 0 },
                    }],
                    expressions: vec![RawExpressionSyntax {
                        span: span(40, 41),
                        kind: RawExpressionKind::Reference {
                            name: RawIdentifierSyntax {
                                text: parameter.to_owned(),
                                span: span(40, 41),
                            },
                        },
                    }],
                },
            }],
        }],
        diagnostics: Vec::new(),
    };
    let syntax =
        verify_snapshot(raw, &sources).map_err(|_| DiagnosticSessionError::SemanticAuthority)?;
    session.admit_semantics(sources, &syntax)
}
