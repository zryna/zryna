//! Match expression construction and source-faithful qualified arm keys.

use std::collections::BTreeSet;

use super::super::syntax;
use zryna_source::UntrustedSpan;

use crate::native_lexer::TokenKind;

use super::super::{FileParser, ParseError, raw, resource, unsupported, valid_name};
use super::Body;

impl FileParser<'_> {
    pub(super) fn match_expression(
        &mut self,
        body: &mut Body,
        nesting: u32,
    ) -> Result<(u32, u32), ParseError> {
        let keyword = self.take(TokenKind::Identifier)?;
        let open_paren = self.take(TokenKind::OpenParen)?;
        if super::super::collections::bounds(&self.tokens, self.position - 1)
            .is_some_and(|(_, count)| count != 2)
        {
            return Err(unsupported(Some(keyword), "unsupported match expression"));
        }
        let (scrutinee, mut depth) = self.binary_expression(body, nesting + 1)?;
        self.take(TokenKind::Comma)?;
        let open_brace = self.take(TokenKind::OpenBrace)?;
        let count = self.collection_count(
            syntax::MAX_MATCH_ARMS_PER_EXPRESSION,
            "match exceeds the arm limit",
        )?;
        self.match_arms += count;
        if self.match_arms > syntax::MAX_MATCH_ARMS_PER_PROJECT {
            return Err(resource("project exceeds the match-arm limit"));
        }
        let mut arms = Vec::new();
        let mut seen = BTreeSet::new();
        while self.current().is_some_and(|token| token.kind() != TokenKind::CloseBrace) {
            let (arm, value_depth) = self.match_arm(body, nesting, &mut seen)?;
            depth = depth.max(value_depth);
            arms.push(arm);
            if self.maybe(TokenKind::Comma).is_none() {
                break;
            }
        }
        let close_brace = self.take(TokenKind::CloseBrace)?;
        let close_paren = self.take(TokenKind::CloseParen)?;
        let span = UntrustedSpan {
            file: self.file,
            start: keyword.span().start(),
            end: close_paren.span().end(),
        };
        let id = self.push_expression(
            body,
            syntax::RawExpressionSyntax {
                span,
                kind: syntax::RawExpressionKind::Match {
                    keyword_span: raw(keyword),
                    open_paren_span: raw(open_paren),
                    scrutinee,
                    close_paren_span: raw(close_paren),
                    open_brace_span: raw(open_brace),
                    arms,
                    close_brace_span: raw(close_brace),
                },
            },
        )?;
        Ok((id, depth + 1))
    }

    fn match_arm(
        &mut self,
        body: &mut Body,
        nesting: u32,
        seen: &mut BTreeSet<String>,
    ) -> Result<(syntax::RawMatchArm, u32), ParseError> {
        let key = self.take(TokenKind::StringLiteral)?;
        let spelling = self.spelling(key).to_owned();
        if !spelling.starts_with('"') || !spelling.ends_with('"') {
            return Err(unsupported(Some(key), "unsupported match arm key"));
        }
        let qualified = spelling[1..spelling.len() - 1].to_owned();
        let Some((type_text, variant_text)) = qualified.split_once('.') else {
            return Err(unsupported(Some(key), "unsupported match arm key"));
        };
        if !valid_name(type_text)
            || !valid_name(variant_text)
            || variant_text.contains('.')
            || !seen.insert(qualified.clone())
        {
            return Err(unsupported(Some(key), "unsupported match arm key"));
        }
        self.take(TokenKind::Colon)?;
        self.take(TokenKind::OpenParen)?;
        let binding = if self.current().is_some_and(|token| token.kind() == TokenKind::Identifier) {
            Some(self.name(super::super::names::Role::Value)?)
        } else {
            None
        };
        self.take(TokenKind::CloseParen)?;
        let arrow = self.take(TokenKind::FatArrow)?;
        let (value, depth) = self.binary_expression(body, nesting + 1)?;
        let start = key.span().start() + 1;
        let type_end = start + u32::try_from(type_text.len()).expect("bounded identifier");
        let variant_end = start + u32::try_from(qualified.len()).expect("bounded key");
        Ok((
            syntax::RawMatchArm {
                span: UntrustedSpan {
                    file: self.file,
                    start: key.span().start(),
                    end: body.expressions[value as usize].span.end,
                },
                type_name: syntax::RawIdentifierSyntax {
                    text: type_text.to_owned(),
                    span: UntrustedSpan { file: self.file, start, end: type_end },
                },
                dot_span: UntrustedSpan { file: self.file, start: type_end, end: type_end + 1 },
                variant: syntax::RawIdentifierSyntax {
                    text: variant_text.to_owned(),
                    span: UntrustedSpan { file: self.file, start: type_end + 1, end: variant_end },
                },
                binding,
                arrow_span: raw(arrow),
                value,
            },
            depth,
        ))
    }
}
