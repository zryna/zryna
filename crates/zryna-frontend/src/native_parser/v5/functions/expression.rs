//! Scalar expressions and canonical postorder expression inventory.

use super::super::syntax;
use zryna_source::UntrustedSpan;

use crate::native_lexer::{Token, TokenKind};

use super::super::{FileParser, ParseError, raw, resource, unsupported};
use super::Body;

impl FileParser<'_> {
    pub(super) fn expression(
        &mut self,
        body: &mut Body,
        block_depth: u32,
    ) -> Result<u32, ParseError> {
        self.current().ok_or_else(|| unsupported(None, "missing expression"))?;
        let (id, depth) = self.binary_expression(body, 1)?;
        if depth + block_depth > syntax::MAX_NESTING_DEPTH {
            return Err(resource("expression exceeds the nesting limit"));
        }
        Ok(id)
    }

    pub(super) fn binary_expression(
        &mut self,
        body: &mut Body,
        nesting: u32,
    ) -> Result<(u32, u32), ParseError> {
        let mut values = vec![self.operand(body, nesting)?];
        let mut operators: Vec<(Token, u8)> = Vec::new();
        while let Some(token) = self.current() {
            let Some(precedence) = precedence(token.kind()) else { break };
            self.position += 1;
            if matches!(token.kind(), TokenKind::Plus | TokenKind::Minus)
                && self.current().is_some_and(|next| {
                    next.kind() == token.kind() && token.span().end() == next.span().start()
                })
            {
                return Err(unsupported(Some(token), "unsupported increment or decrement"));
            }
            while operators.last().is_some_and(|(_, current)| *current >= precedence) {
                let (operator, _) = operators.pop().expect("pending binary operator");
                self.reduce(body, &mut values, operator)?;
            }
            operators.push((token, precedence));
            values.push(self.operand(body, nesting)?);
        }
        while let Some((operator, _)) = operators.pop() {
            self.reduce(body, &mut values, operator)?;
        }
        Ok(values.pop().expect("expression root"))
    }

    fn reduce(
        &self,
        body: &mut Body,
        values: &mut Vec<(u32, u32)>,
        operator: Token,
    ) -> Result<(), ParseError> {
        let (rhs, rhs_depth) = values.pop().expect("binary right operand");
        let (lhs, lhs_depth) = values.pop().expect("binary left operand");
        let depth = lhs_depth.max(rhs_depth) + 1;
        if depth > syntax::MAX_NESTING_DEPTH {
            return Err(resource("expression exceeds the nesting limit"));
        }
        let span = UntrustedSpan {
            file: self.file,
            start: body.expressions[lhs as usize].span.start,
            end: body.expressions[rhs as usize].span.end,
        };
        let operator_span = raw(operator);
        let kind = match operator.kind() {
            TokenKind::Plus => syntax::RawExpressionKind::Addition { operator_span, lhs, rhs },
            TokenKind::Minus => syntax::RawExpressionKind::Subtraction { operator_span, lhs, rhs },
            TokenKind::Asterisk => {
                syntax::RawExpressionKind::Multiplication { operator_span, lhs, rhs }
            }
            TokenKind::StrictEqual => syntax::RawExpressionKind::Equal { operator_span, lhs, rhs },
            TokenKind::StrictNotEqual => {
                syntax::RawExpressionKind::NotEqual { operator_span, lhs, rhs }
            }
            TokenKind::LessThan => syntax::RawExpressionKind::LessThan { operator_span, lhs, rhs },
            TokenKind::LessEqual => {
                syntax::RawExpressionKind::LessEqual { operator_span, lhs, rhs }
            }
            TokenKind::GreaterThan => {
                syntax::RawExpressionKind::GreaterThan { operator_span, lhs, rhs }
            }
            TokenKind::GreaterEqual => {
                syntax::RawExpressionKind::GreaterEqual { operator_span, lhs, rhs }
            }
            _ => unreachable!("binary operator stack"),
        };
        let id = self.push_expression(body, syntax::RawExpressionSyntax { span, kind })?;
        values.push((id, depth));
        Ok(())
    }

    pub(super) fn call(&mut self, body: &mut Body, nesting: u32) -> Result<(u32, u32), ParseError> {
        let callee = self.name(super::super::names::Role::Runtime)?;
        let type_arguments = self.type_arguments(0)?;
        let open = self.take(TokenKind::OpenParen)?;
        let count = super::super::collections::bounds(&self.tokens, self.position - 1)
            .map_or(0, |(_, count)| count);
        let required = match callee.text.as_str() {
            "clone" | "shared" | "downgrade" | "borrow" | "borrowMut" => Some(1),
            "push" => Some(2),
            _ => None,
        };
        if required.is_some() && type_arguments.is_some() {
            return Err(unsupported(Some(open), "intrinsic type arguments are excluded"));
        }
        if required.is_some_and(|required| count != required) {
            return Err(unsupported(Some(open), "wrong intrinsic argument count"));
        }
        if count > syntax::MAX_PARAMETERS_PER_FUNCTION {
            return Err(resource("call exceeds the argument limit"));
        }
        let mut arguments = Vec::new();
        let mut commas = Vec::new();
        let mut depth = 0;
        while self.current().is_some_and(|token| token.kind() != TokenKind::CloseParen) {
            if arguments.len() >= syntax::MAX_PARAMETERS_PER_FUNCTION {
                return Err(resource("call exceeds the argument limit"));
            }
            let (argument, argument_depth) = self.binary_expression(body, nesting + 1)?;
            arguments.push(argument);
            depth = depth.max(argument_depth);
            let Some(comma) = self.maybe(TokenKind::Comma) else {
                break;
            };
            commas.push(comma);
        }
        let close = self.take(TokenKind::CloseParen)?;
        let span =
            UntrustedSpan { file: self.file, start: callee.span.start, end: close.span().end() };
        let kind = match (callee.text.as_str(), arguments.as_slice()) {
            ("clone", &[value]) => syntax::RawExpressionKind::Clone {
                keyword_span: callee.span,
                open_paren_span: raw(open),
                value,
                close_paren_span: raw(close),
            },
            ("shared", &[value]) => syntax::RawExpressionKind::Shared {
                keyword_span: callee.span,
                open_paren_span: raw(open),
                value,
                close_paren_span: raw(close),
            },
            ("downgrade", &[value]) => syntax::RawExpressionKind::Downgrade {
                keyword_span: callee.span,
                open_paren_span: raw(open),
                value,
                close_paren_span: raw(close),
            },
            ("borrow", &[value]) => syntax::RawExpressionKind::Borrow {
                keyword_span: callee.span,
                open_paren_span: raw(open),
                value,
                close_paren_span: raw(close),
            },
            ("borrowMut", &[value]) => syntax::RawExpressionKind::BorrowMut {
                keyword_span: callee.span,
                open_paren_span: raw(open),
                value,
                close_paren_span: raw(close),
            },
            ("push", &[vector, value]) => syntax::RawExpressionKind::VecPush {
                keyword_span: callee.span,
                open_paren_span: raw(open),
                vector,
                comma_span: raw(commas[0]),
                value,
                close_paren_span: raw(close),
            },
            ("clone" | "shared" | "downgrade" | "borrow" | "borrowMut" | "push", _) => {
                return Err(unsupported(Some(open), "wrong intrinsic argument count"));
            }
            _ => syntax::RawExpressionKind::Call {
                callee,
                type_arguments,
                open_paren_span: raw(open),
                arguments,
                close_paren_span: raw(close),
            },
        };
        let id = self.push_expression(body, syntax::RawExpressionSyntax { span, kind })?;
        Ok((id, depth + 1))
    }

    pub(super) fn push_expression(
        &self,
        body: &mut Body,
        expression: syntax::RawExpressionSyntax,
    ) -> Result<u32, ParseError> {
        if body.expressions.len() >= syntax::MAX_EXPRESSIONS_PER_FUNCTION
            || self.previous_expressions + body.expressions.len()
                >= syntax::MAX_EXPRESSIONS_PER_PROJECT
        {
            return Err(resource(
                if body.expressions.len() >= syntax::MAX_EXPRESSIONS_PER_FUNCTION {
                    "function exceeds the expression limit"
                } else {
                    "project exceeds the expression limit"
                },
            ));
        }
        let id = u32::try_from(body.expressions.len()).expect("bounded expressions");
        body.expressions.push(expression);
        Ok(id)
    }
}

fn precedence(kind: TokenKind) -> Option<u8> {
    match kind {
        TokenKind::StrictEqual | TokenKind::StrictNotEqual => Some(1),
        TokenKind::LessThan
        | TokenKind::LessEqual
        | TokenKind::GreaterThan
        | TokenKind::GreaterEqual => Some(2),
        TokenKind::Plus | TokenKind::Minus => Some(3),
        TokenKind::Asterisk => Some(4),
        _ => None,
    }
}
