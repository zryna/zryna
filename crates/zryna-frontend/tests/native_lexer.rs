//! Independent evidence for the native protocol-v4 lexical foundation.

use zryna_frontend::native_lexer::{
    Keyword, Lexeme, MAX_LEXEMES_PER_PROJECT, MAX_LEXICAL_DIAGNOSTICS, MAX_TOKENS_PER_FILE,
    MAX_TRIVIA_PER_FILE, TokenKind, TriviaKind, lex,
};
use zryna_source::{SourceFileInput, SourceMap};

fn sources(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("bounded lexer fixture")
}

fn kinds(source: &str) -> Vec<TokenKind> {
    let map = sources(source);
    lex(&map).expect("valid lexical stream").files()[0].tokens().map(|token| token.kind()).collect()
}

#[test]
fn protocol_v4_surface_uses_maximal_munch_and_retains_trivia() {
    let text =
        "// 😀\r\nexport function identity(value: i32): i32 { /*keep*/ return value !== 0; }";
    let map = sources(text);
    let project = lex(&map).expect("valid protocol-v4 lexical surface");
    assert!(project.is_bound_to(&map));
    assert!(project.diagnostics().is_empty());
    let file = &project.files()[0];
    assert_eq!(file.path().as_str(), "src/main.zry");
    assert!(file.lexemes().iter().any(|lexeme| matches!(
        lexeme,
        Lexeme::Trivia(trivia) if trivia.kind() == TriviaKind::LineComment
    )));
    assert!(file.lexemes().iter().any(|lexeme| matches!(
        lexeme,
        Lexeme::Trivia(trivia) if trivia.kind() == TriviaKind::BlockComment
    )));
    assert!(file.tokens().any(|token| token.kind() == TokenKind::StrictNotEqual));
    let authoritative = map.source(file.id()).expect("lexed file belongs to source map");
    let reconstructed = file
        .lexemes()
        .iter()
        .map(|lexeme| {
            let span = lexeme.span();
            map.resolve(span).expect("every lexeme span is source authenticated");
            let start = usize::try_from(span.start()).expect("bounded source offset");
            let end = usize::try_from(span.end()).expect("bounded source offset");
            &authoritative.text()[start..end]
        })
        .collect::<String>();
    assert_eq!(reconstructed, text);
    for lexeme in file.lexemes() {
        map.resolve(lexeme.span()).expect("every lexeme span is source authenticated");
    }
}

#[test]
fn canonical_keywords_identifiers_literals_and_operators_are_distinct() {
    assert_eq!(
        kinds("if true { return value <= 12; } else { return 'é'; }").as_slice(),
        &[
            TokenKind::Keyword(Keyword::If),
            TokenKind::Keyword(Keyword::True),
            TokenKind::OpenBrace,
            TokenKind::Keyword(Keyword::Return),
            TokenKind::Identifier,
            TokenKind::LessEqual,
            TokenKind::DecimalInteger,
            TokenKind::Semicolon,
            TokenKind::CloseBrace,
            TokenKind::Keyword(Keyword::Else),
            TokenKind::OpenBrace,
            TokenKind::Keyword(Keyword::Return),
            TokenKind::StringLiteral,
            TokenKind::Semicolon,
            TokenKind::CloseBrace,
        ]
    );
}

#[test]
fn identifier_length_accepts_exact_boundary_and_rejects_complete_first_extra() {
    let exact = "a".repeat(128);
    let map = sources(&exact);
    let project = lex(&map).expect("exact identifier byte limit");
    assert!(project.diagnostics().is_empty());
    assert_eq!(
        project.files()[0].tokens().map(|token| token.kind()).collect::<Vec<_>>(),
        [TokenKind::Identifier]
    );

    assert_invalid_identifier_recovers(&"a".repeat(129));
}

#[test]
fn forbidden_identifiers_reject_only_exact_spellings_and_recover() {
    for spelling in ["constructor", "prototype", "__proto__"] {
        assert_invalid_identifier_recovers(spelling);
        let text = format!("{spelling}_ _{spelling} {spelling}1");
        let map = sources(&text);
        let project = lex(&map).expect("valid identifiers containing forbidden names");
        assert!(project.diagnostics().is_empty());
        assert_eq!(
            project.files()[0].tokens().map(|token| token.kind()).collect::<Vec<_>>(),
            [TokenKind::Identifier; 3]
        );
    }
}

fn assert_invalid_identifier_recovers(spelling: &str) {
    let text = format!("before/*é*/{spelling};after");
    let map = sources(&text);
    let project = lex(&map).expect("bounded invalid identifier recovery");
    assert_eq!(project, lex(&map).expect("deterministic invalid identifier replay"));
    let file = &project.files()[0];
    let tokens = file.tokens().collect::<Vec<_>>();
    assert_eq!(
        tokens.iter().map(|token| token.kind()).collect::<Vec<_>>(),
        [TokenKind::Identifier, TokenKind::Invalid, TokenKind::Semicolon, TokenKind::Identifier]
    );
    assert_eq!(project.diagnostics().len(), 1);
    let diagnostic = &project.diagnostics()[0];
    assert_eq!(diagnostic.code(), "ZRYNA-F1501");
    assert_eq!(diagnostic.primary_span(), Some(tokens[1].span()));
    assert_eq!(tokens[1].span().start(), u32::try_from("before/*é*/".len()).unwrap());
    assert_eq!(
        tokens[1].span().end() - tokens[1].span().start(),
        u32::try_from(spelling.len()).unwrap()
    );
    let reconstructed = file
        .lexemes()
        .iter()
        .map(|lexeme| {
            let span = lexeme.span();
            map.resolve(span).expect("source-authenticated recovery span");
            &text[usize::try_from(span.start()).unwrap()..usize::try_from(span.end()).unwrap()]
        })
        .collect::<String>();
    assert_eq!(reconstructed, text);
}

#[test]
fn malformed_unicode_strings_and_comments_are_deterministic_and_recover() {
    let map = sources("é; 'bad\\escape'; \"open\u{2028}return 1; /* open");
    let first = lex(&map).expect("bounded malformed stream");
    let second = lex(&map).expect("deterministic replay");
    assert_eq!(first, second);
    assert_eq!(first.diagnostics().len(), 4);
    assert!(first.diagnostics().iter().all(|diagnostic| {
        diagnostic.code() == "ZRYNA-F1501"
            && diagnostic.primary_span().is_some_and(|span| map.resolve(span).is_ok())
    }));
    assert!(first.files()[0].tokens().any(|token| token.kind() == TokenKind::Semicolon));
    assert!(first.files()[0].tokens().any(|token| token.kind() == TokenKind::DecimalInteger));
}

#[test]
fn token_and_diagnostic_limits_accept_exact_reject_first_extra_and_recover() {
    let exact_tokens = sources(&";".repeat(MAX_TOKENS_PER_FILE));
    assert_eq!(
        lex(&exact_tokens).expect("exact token boundary").files()[0].tokens().count(),
        MAX_TOKENS_PER_FILE
    );
    let extra_token = sources(&";".repeat(MAX_TOKENS_PER_FILE + 1));
    assert_eq!(
        lex(&extra_token).expect_err("first extra token").diagnostic().code(),
        "ZRYNA-F1502"
    );

    let exact_trivia = sources(&" /**/".repeat(MAX_TRIVIA_PER_FILE / 2));
    assert_eq!(
        lex(&exact_trivia).expect("exact trivia boundary").files()[0]
            .lexemes()
            .iter()
            .filter(|lexeme| matches!(lexeme, Lexeme::Trivia(_)))
            .count(),
        MAX_TRIVIA_PER_FILE
    );
    let extra_trivia = sources(&(" /**/".repeat(MAX_TRIVIA_PER_FILE / 2) + " "));
    assert_eq!(
        lex(&extra_trivia).expect_err("first extra trivia").diagnostic().code(),
        "ZRYNA-F1502"
    );

    let exact_diagnostics = sources(&"?".repeat(MAX_LEXICAL_DIAGNOSTICS));
    assert_eq!(
        lex(&exact_diagnostics).expect("exact diagnostic boundary").diagnostics().len(),
        MAX_LEXICAL_DIAGNOSTICS
    );
    let extra_diagnostic = sources(&"?".repeat(MAX_LEXICAL_DIAGNOSTICS + 1));
    assert_eq!(
        lex(&extra_diagnostic).expect_err("first extra diagnostic").diagnostic().code(),
        "ZRYNA-F1502"
    );

    let recovered = sources("export function ok(): i32 { return 1; }");
    assert!(lex(&recovered).expect("pristine recovery").diagnostics().is_empty());
}

#[test]
fn project_lexeme_limit_accepts_exact_and_rejects_first_extra() {
    let quarter = "; ".repeat(MAX_LEXEMES_PER_PROJECT / 8);
    let exact = SourceMap::build(
        (0..4)
            .map(|index| SourceFileInput {
                path: format!("src/{index}.zry"),
                text: quarter.clone(),
            })
            .collect(),
    )
    .expect("exact project fixture");
    assert_eq!(
        lex(&exact)
            .expect("exact project lexeme boundary")
            .files()
            .iter()
            .map(|file| file.lexemes().len())
            .sum::<usize>(),
        MAX_LEXEMES_PER_PROJECT
    );

    let mut extra_inputs = (0..4)
        .map(|index| SourceFileInput { path: format!("src/{index}.zry"), text: quarter.clone() })
        .collect::<Vec<_>>();
    extra_inputs.push(SourceFileInput { path: "src/extra.zry".to_owned(), text: ";".to_owned() });
    let extra = SourceMap::build(extra_inputs).expect("first-extra project fixture");
    assert_eq!(
        lex(&extra).expect_err("first extra project lexeme").diagnostic().code(),
        "ZRYNA-F1502"
    );
}

#[test]
fn canonical_file_order_and_source_map_identity_are_preserved() {
    let map = SourceMap::build(vec![
        SourceFileInput { path: "src/z.zry".to_owned(), text: "return z;".to_owned() },
        SourceFileInput { path: "src/a.zry".to_owned(), text: "return a;".to_owned() },
    ])
    .expect("canonical multi-file map");
    let project = lex(&map).expect("multi-file lexical stream");
    assert_eq!(
        project.files().iter().map(|file| file.path().as_str()).collect::<Vec<_>>(),
        ["src/a.zry", "src/z.zry"]
    );
    let other = sources("return a;");
    assert!(!project.is_bound_to(&other));
}
