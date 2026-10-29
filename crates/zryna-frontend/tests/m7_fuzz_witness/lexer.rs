//! Lexer-only retained outcome projection.
use serde_json::Value;

pub fn lexer_output(
    result: &Result<
        zryna_frontend::native_lexer::LexedProject,
        zryna_frontend::native_lexer::LexError,
    >,
) -> Value {
    match result {
        Ok(project) => serde_json::json!({"result":{
            "files":project.files().iter().map(|file| serde_json::json!({
                "id":file.id().index(), "path":file.path().as_str(),
                "lexemes":file.lexemes().iter().map(|lexeme| {
                    let span = lexeme.span();
                    let kind = match lexeme {
                        zryna_frontend::native_lexer::Lexeme::Token(token) => format!("token:{:?}", token.kind()),
                        zryna_frontend::native_lexer::Lexeme::Trivia(trivia) => format!("trivia:{:?}", trivia.kind()),
                    };
                    serde_json::json!({"kind":kind,"start":span.start(),"end":span.end()})
                }).collect::<Vec<_>>()
            })).collect::<Vec<_>>(),
            "diagnostics":project.diagnostics().iter().map(|diagnostic| serde_json::json!({
                "code":diagnostic.code(),"message":diagnostic.message(),
                "severity":diagnostic.severity(),"guidance":diagnostic.guidance(),
                "span":diagnostic.primary_span().map(|span| serde_json::json!({
                    "file":span.file().index(),"start":span.start(),"end":span.end()}))
            })).collect::<Vec<_>>()
        }}),
        Err(error) => serde_json::json!({"error":format!("{error:?}")}),
    }
}
