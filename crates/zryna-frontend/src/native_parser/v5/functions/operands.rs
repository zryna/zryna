//! Scalar operand recognition, unary signs, and postfix chains.

use super::super::syntax;
use zryna_source::UntrustedSpan;

use crate::native_lexer::{Keyword, Token, TokenKind};

use super::super::{FileParser, ParseError, raw, resource, unsupported};
use super::Body;

impl FileParser<'_> {
    pub(super) fn operand(
        &mut self,
        body: &mut Body,
        nesting: u32,
    ) -> Result<(u32, u32), ParseError> {
        if nesting > syntax::MAX_NESTING_DEPTH {
            return Err(resource("expression nesting exceeds protocol-v5 limit"));
        }
        let mut minuses = Vec::new();
        while let Some(token) = self.current().filter(|token| token.kind() == TokenKind::Minus) {
            if minuses
                .last()
                .is_some_and(|previous: &Token| previous.span().end() == token.span().start())
            {
                return Err(unsupported(Some(token), "unsupported decrement"));
            }
            minuses.push(token);
            if minuses.len() >= syntax::MAX_NESTING_DEPTH as usize {
                return Err(resource("expression nesting exceeds protocol-v5 limit"));
            }
            self.position += 1;
        }
        let first = self.current().ok_or_else(|| unsupported(None, "missing expression"))?;
        let (mut id, mut depth) = if matches!(
            first.kind(),
            TokenKind::Identifier
                | TokenKind::Keyword(
                    Keyword::As | Keyword::From | Keyword::Let | Keyword::Interface
                )
        ) && self.spelling(first) == "match"
            && self.next().is_some_and(|next| next.kind() == TokenKind::OpenParen)
        {
            self.match_expression(body, nesting)?
        } else if let Some(type_end) = self.typed_array_end() {
            self.typed_array(body, nesting, type_end)?
        } else if matches!(
            first.kind(),
            TokenKind::Identifier
                | TokenKind::Keyword(
                    Keyword::As | Keyword::From | Keyword::Let | Keyword::Interface
                )
        ) && self.next().is_some_and(|next| next.kind() == TokenKind::Dot)
            && self.call_open(self.position + 2).is_some()
        {
            self.enum_construction(body, nesting)?
        } else if matches!(
            first.kind(),
            TokenKind::Identifier
                | TokenKind::Keyword(
                    Keyword::As | Keyword::From | Keyword::Let | Keyword::Interface
                )
        ) && self.call_open(self.position).is_some_and(|open| {
            self.tokens.get(open + 1).is_some_and(|next| next.kind() == TokenKind::OpenBrace)
        }) && !matches!(
            self.spelling(first),
            "clone"
                | "borrow"
                | "borrowMut"
                | "shared"
                | "downgrade"
                | "push"
                | "match"
                | "upgradeWeak"
        ) {
            self.struct_construction(body, nesting)?
        } else if matches!(
            first.kind(),
            TokenKind::Identifier
                | TokenKind::Keyword(
                    Keyword::As | Keyword::From | Keyword::Let | Keyword::Interface
                )
        ) && self.call_open(self.position).is_some()
        {
            self.call(body, nesting)?
        } else if first.kind() == TokenKind::DecimalInteger
            && !minuses.is_empty()
            && minuses.last().is_some_and(|minus| minus.span().end() == first.span().start())
            && self.spelling(first) != "0"
        {
            let minus = minuses.pop().expect("numeric negation");
            self.numeric_negation(body, minus, first)?
        } else {
            self.simple_operand(body, first)?
        };
        (id, depth) = self.postfix(body, id, depth, nesting)?;
        for minus in minuses.into_iter().rev() {
            depth += 1;
            let span = UntrustedSpan {
                file: self.file,
                start: minus.span().start(),
                end: body.expressions[id as usize].span.end,
            };
            id = self.push_expression(
                body,
                syntax::RawExpressionSyntax {
                    span,
                    kind: syntax::RawExpressionKind::Negation {
                        operator_span: raw(minus),
                        operand: id,
                    },
                },
            )?;
        }
        Ok((id, depth))
    }

    fn numeric_negation(
        &mut self,
        body: &mut Body,
        minus: Token,
        digits: Token,
    ) -> Result<(u32, u32), ParseError> {
        let spelling =
            self.text[minus.span().start() as usize..digits.span().end() as usize].to_owned();
        if spelling.len() > 64 || spelling[1..].starts_with('0') {
            return Err(unsupported(Some(digits), "noncanonical integer literal"));
        }
        self.position += 1;
        let span = UntrustedSpan {
            file: self.file,
            start: minus.span().start(),
            end: digits.span().end(),
        };
        let id = self.push_expression(
            body,
            syntax::RawExpressionSyntax {
                span,
                kind: syntax::RawExpressionKind::I32Literal { spelling },
            },
        )?;
        Ok((id, 1))
    }

    fn simple_operand(&mut self, body: &mut Body, first: Token) -> Result<(u32, u32), ParseError> {
        let kind = match first.kind() {
            TokenKind::Identifier
            | TokenKind::Keyword(Keyword::As | Keyword::From | Keyword::Let | Keyword::Interface) => {
                syntax::RawExpressionKind::Reference {
                    name: self.name(super::super::names::Role::Runtime)?,
                }
            }
            TokenKind::Keyword(Keyword::True | Keyword::False) => {
                self.position += 1;
                syntax::RawExpressionKind::BoolLiteral {
                    value: first.kind() == TokenKind::Keyword(Keyword::True),
                }
            }
            TokenKind::DecimalInteger => {
                let spelling = self.spelling(first);
                if spelling.len() > 64 || (spelling != "0" && spelling.starts_with('0')) {
                    return Err(unsupported(Some(first), "noncanonical integer literal"));
                }
                let spelling = spelling.to_owned();
                self.position += 1;
                syntax::RawExpressionKind::I32Literal { spelling }
            }
            TokenKind::StringLiteral => {
                let spelling = self.spelling(first).to_owned();
                self.position += 1;
                syntax::RawExpressionKind::StringLiteral { spelling }
            }
            _ => return Err(unsupported(Some(first), "unsupported protocol-v5 expression")),
        };
        Ok((self.push_expression(body, syntax::RawExpressionSyntax { span: raw(first), kind })?, 1))
    }

    fn postfix(
        &mut self,
        body: &mut Body,
        mut id: u32,
        mut depth: u32,
        nesting: u32,
    ) -> Result<(u32, u32), ParseError> {
        loop {
            if let Some(dot) = self.maybe(TokenKind::Dot) {
                let field = self.identifier()?;
                let span = UntrustedSpan {
                    file: self.file,
                    start: body.expressions[id as usize].span.start,
                    end: field.span.end,
                };
                id = self.push_expression(
                    body,
                    syntax::RawExpressionSyntax {
                        span,
                        kind: syntax::RawExpressionKind::FieldAccess {
                            base: id,
                            dot_span: raw(dot),
                            field,
                        },
                    },
                )?;
                depth += 1;
            } else if let Some(open) = self.maybe(TokenKind::OpenBracket) {
                let (index, index_depth) = self.binary_expression(body, nesting + 1)?;
                let close = self.take(TokenKind::CloseBracket)?;
                let span = UntrustedSpan {
                    file: self.file,
                    start: body.expressions[id as usize].span.start,
                    end: close.span().end(),
                };
                id = self.push_expression(
                    body,
                    syntax::RawExpressionSyntax {
                        span,
                        kind: syntax::RawExpressionKind::Index {
                            base: id,
                            open_bracket_span: raw(open),
                            index,
                            close_bracket_span: raw(close),
                        },
                    },
                )?;
                depth = depth.max(index_depth) + 1;
            } else {
                break;
            }
        }
        Ok((id, depth))
    }
}
