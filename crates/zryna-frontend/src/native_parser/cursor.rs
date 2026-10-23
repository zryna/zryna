//! Token cursor operations and source-faithful v2 annotations.

use super::{
    AnnotationContext, Diagnostic, FileParser, Keyword, ParseError, Token, TokenKind,
    UntrustedSpan, error_at, failure, raw, recovery, syntax,
};

impl FileParser<'_> {
    pub(super) fn starts_function(&self) -> bool {
        self.current().is_some_and(|token| token.kind() == TokenKind::Keyword(Keyword::Function))
            || self
                .current()
                .is_some_and(|token| token.kind() == TokenKind::Keyword(Keyword::Export))
                && self
                    .tokens
                    .get(self.position + 1)
                    .is_some_and(|token| token.kind() == TokenKind::Keyword(Keyword::Function))
    }

    pub(super) fn error_between(
        &self,
        first: Token,
        last: Token,
        message: impl Into<String>,
    ) -> ParseError {
        let span =
            UntrustedSpan { file: self.file, start: first.span().start(), end: last.span().end() };
        self.sources.verify_span(span).map_or_else(
            |_| failure("ZRYNA-F1003", "native token range is not source-bound"),
            |span| ParseError {
                primary: Diagnostic::error_at(
                    "ZRYNA-F2002",
                    span,
                    message,
                    "use only the documented protocol-v2 bootstrap syntax",
                ),
                following: None,
            },
        )
    }

    pub(super) fn error_here(&self, code: &'static str, message: &'static str) -> ParseError {
        self.current()
            .or_else(|| self.tokens.last().copied())
            .map_or_else(|| failure(code, message), |token| error_at(token, code, message))
    }

    pub(super) fn current(&self) -> Option<Token> {
        self.tokens.get(self.position).copied()
    }

    pub(super) fn take(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        let token = self
            .current()
            .ok_or_else(|| self.error_here("ZRYNA-F2002", "unexpected end of source"))?;
        if token.kind() != kind {
            return Err(self.error_here("ZRYNA-F2002", "unexpected token in protocol-v2 source"));
        }
        self.position += 1;
        Ok(token)
    }

    pub(super) fn maybe(&mut self, kind: TokenKind) -> Option<Token> {
        if self.current().is_some_and(|token| token.kind() == kind) {
            let token = self.current();
            self.position += 1;
            token
        } else {
            None
        }
    }

    pub(super) fn identifier(&mut self) -> Result<syntax::RawIdentifierSyntax, ParseError> {
        let token = self.take(TokenKind::Identifier)?;
        if matches!(
            self.spelling(token),
            "this" | "null" | "super" | "new" | "typeof" | "void" | "delete" | "class"
        ) {
            return Err(error_at(token, "ZRYNA-F2002", "unsupported identifier spelling"));
        }
        Ok(syntax::RawIdentifierSyntax { text: self.spelling(token).to_owned(), span: raw(token) })
    }

    pub(super) fn spelling(&self, token: Token) -> &str {
        &self.text[token.span().start() as usize..token.span().end() as usize]
    }

    pub(super) fn annotation(
        &mut self,
        insertion: u32,
        context: AnnotationContext,
    ) -> Result<syntax::RawTypeSyntax, ParseError> {
        if self.maybe(TokenKind::Colon).is_none() {
            return Ok(syntax::RawTypeSyntax {
                span: UntrustedSpan { file: self.file, start: insertion, end: insertion },
                kind: syntax::RawTypeSyntaxKind::Missing,
            });
        }
        let token = self
            .current()
            .ok_or_else(|| self.error_here("ZRYNA-F2002", "missing type annotation"))?;
        match token.kind() {
            TokenKind::Identifier => {}
            _ => return Err(self.error_here("ZRYNA-F2002", "unsupported type annotation")),
        }
        if let Some(annotation) = self.recover_complex_annotation(&context)? {
            return Ok(annotation);
        }
        if let Some(kind) = recovery::primitive_kind(self.spelling(token)) {
            let context = match context {
                AnnotationContext::Parameter(index) => format!("parameter {index} annotation"),
                AnnotationContext::Result(index) => format!("function {index} result annotation"),
            };
            let error = ParseError {
                primary: Diagnostic::error_at(
                    "ZRYNA-F2002",
                    token.span(),
                    format!("{context} uses unsupported syntax '{kind}'"),
                    "use only the documented protocol-v2 bootstrap syntax",
                ),
                following: None,
            };
            if !self.recovering {
                return Err(error);
            }
            self.retain_error(&error);
        }
        if matches!(self.spelling(token), "infer" | "keyof" | "readonly" | "unique") {
            return Err(error_at(token, "ZRYNA-F2002", "unsupported type annotation"));
        }
        self.position += 1;
        Ok(syntax::RawTypeSyntax {
            span: raw(token),
            kind: syntax::RawTypeSyntaxKind::Named { name: self.spelling(token).to_owned() },
        })
    }
}
