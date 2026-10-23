//! Source-spelled bounded lists; these contain no type resolution or trait authority.

use super::{FileParser, ParseError, raw, resource, syntax, unsupported};
use crate::native_lexer::{Keyword, TokenKind};
use zryna_diagnostics::Diagnostic;
use zryna_source::UntrustedSpan;

mod arity;

impl FileParser<'_> {
    pub(super) fn type_parameters(
        &mut self,
    ) -> Result<Option<syntax::RawTypeParameterList>, ParseError> {
        let Some(less) = self.maybe(TokenKind::LessThan) else { return Ok(None) };
        if let Some((first, _)) = arity::parameters(
            &self.tokens,
            self.position,
            |token| self.arity_name(token, super::names::Role::Parameter),
            |token| self.arity_name(token, super::names::Role::Type),
        ) {
            return Err(self.extra_type_item(
                "ZRYNA-D7001",
                raw(first),
                "at most two type parameters are admitted",
            ));
        }
        let mut parameters = Vec::new();
        let mut comma_spans = Vec::new();
        loop {
            if parameters.len() >= syntax::MAX_TYPE_PARAMETERS {
                return Err(unsupported(self.current(), "malformed bounded type parameters"));
            }
            let name = self.name(super::names::Role::Parameter)?;
            let extends = self.take(TokenKind::Keyword(Keyword::Extends))?;
            let bound = self.name(super::names::Role::Type)?;
            if super::names::type_diversion(&bound.text) {
                return Err(unsupported(self.current(), "bound must be a named type"));
            }
            parameters.push(syntax::RawTypeParameter {
                span: UntrustedSpan {
                    file: self.file,
                    start: name.span.start,
                    end: bound.span.end,
                },
                name,
                extends_span: raw(extends),
                bound,
            });
            let Some(comma) = self.maybe(TokenKind::Comma) else { break };
            comma_spans.push(raw(comma));
            if self.current().is_some_and(|token| token.kind() == TokenKind::GreaterThan) {
                break;
            }
        }
        let greater = self.take(TokenKind::GreaterThan)?;
        Ok(Some(syntax::RawTypeParameterList {
            span: UntrustedSpan {
                file: self.file,
                start: less.span().start(),
                end: greater.span().end(),
            },
            less_than_span: raw(less),
            parameters,
            comma_spans,
            greater_than_span: raw(greater),
        }))
    }

    pub(super) fn type_arguments(
        &mut self,
        depth: u32,
    ) -> Result<Option<syntax::RawTypeArgumentList>, ParseError> {
        let Some(less) = self.maybe(TokenKind::LessThan) else { return Ok(None) };
        self.check_argument_arity(self.position)?;
        if depth > syntax::MAX_NESTING_DEPTH {
            return Err(resource("type syntax exceeds the nesting limit"));
        }
        let mut arguments = Vec::new();
        let mut comma_spans = Vec::new();
        loop {
            if arguments.len() >= syntax::MAX_TYPE_ARGUMENTS {
                return Err(unsupported(self.current(), "malformed bounded type arguments"));
            }
            arguments.push(self.type_at_depth(depth + 1)?);
            let Some(comma) = self.maybe(TokenKind::Comma) else { break };
            comma_spans.push(raw(comma));
            if self.current().is_some_and(|token| {
                matches!(token.kind(), TokenKind::GreaterThan | TokenKind::GreaterEqual)
            }) {
                break;
            }
        }
        let greater = self.type_greater_than()?;
        Ok(Some(syntax::RawTypeArgumentList {
            span: UntrustedSpan { file: self.file, start: less.span().start(), end: greater.end },
            less_than_span: raw(less),
            arguments,
            comma_spans,
            greater_than_span: greater,
        }))
    }

    pub(super) fn check_argument_arity(&self, start: usize) -> Result<(), ParseError> {
        let extra = arity::arguments(
            &self.tokens,
            start,
            |token| self.arity_name(token, super::names::Role::Type),
            |token| {
                token.kind() == TokenKind::DecimalInteger
                    && (self.spelling(token) == "0" || !self.spelling(token).starts_with('0'))
            },
        );
        if let Some((first, last)) = extra {
            return Err(self.extra_type_item(
                "ZRYNA-M7001",
                UntrustedSpan { end: last.span().end(), ..raw(first) },
                "at most two type arguments are admitted",
            ));
        }
        Ok(())
    }

    fn arity_name(&self, token: crate::native_lexer::Token, role: super::names::Role) -> bool {
        matches!(token.kind(), TokenKind::Identifier | TokenKind::Keyword(_))
            && super::valid_name(self.spelling(token))
            && self.allows_name(self.spelling(token), role)
            && (!matches!(role, super::names::Role::Type)
                || !super::names::type_diversion(self.spelling(token)))
    }

    // Rejection precedes raw DTO issuance. The wire and source seal retain their two-item caps;
    // neither this producer error nor a provider advisory can authenticate an incomplete DTO.
    fn extra_type_item(
        &self,
        code: &'static str,
        span: UntrustedSpan,
        message: &'static str,
    ) -> ParseError {
        self.sources.verify_span(span).map_or_else(
            |_| unsupported(None, "invalid source span for a rejected generic item"),
            |span| ParseError {
                diagnostic: Diagnostic::error_at(
                    code,
                    span,
                    message,
                    "use one or two explicit bounded generic items",
                ),
            },
        )
    }

    /// Look ahead without allocating types or consuming parser inventory.
    pub(super) fn call_open(&self, name: usize) -> Option<usize> {
        let next = self.tokens.get(name + 1)?;
        if next.kind() == TokenKind::OpenParen {
            return Some(name + 1);
        }
        if next.kind() != TokenKind::LessThan {
            return None;
        }
        let end = super::super::collections::generic_end(&self.tokens, name + 1)?;
        (self.tokens.get(end)?.kind() == TokenKind::OpenParen).then_some(end)
    }
}
