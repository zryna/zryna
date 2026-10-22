//! Block-frame opening, closing, and structured control transitions.

use super::super::{FileParser, ParseError, raw, resource, syntax};
use super::{Body, Frame, Owner, unfinished_block};
use crate::native_lexer::{Keyword, Token, TokenKind};
use zryna_source::UntrustedSpan;

impl FileParser<'_> {
    pub(super) fn control_statement(
        &mut self,
        body: &mut Body,
        frames: &mut Vec<Frame>,
        token: Token,
        statement: usize,
    ) -> Result<(), ParseError> {
        self.position += 1;
        let open_paren = self.take(TokenKind::OpenParen)?;
        let block_depth = u32::try_from(frames.len()).expect("bounded block depth");
        let condition = self.expression(body, block_depth)?;
        let close_paren = self.take(TokenKind::CloseParen)?;
        let block = u32::try_from(body.blocks.len()).expect("bounded blocks");
        let is_if = token.kind() == TokenKind::Keyword(Keyword::If);
        let kind = if is_if {
            syntax::RawStatementKind::If {
                keyword_span: raw(token),
                open_paren_span: raw(open_paren),
                condition,
                close_paren_span: raw(close_paren),
                then_block: block,
                else_clause: None,
            }
        } else {
            syntax::RawStatementKind::While {
                keyword_span: raw(token),
                open_paren_span: raw(open_paren),
                condition,
                close_paren_span: raw(close_paren),
                body_block: block,
            }
        };
        body.statements.push(syntax::RawStatementSyntax {
            span: UntrustedSpan {
                file: self.file,
                start: token.span().start(),
                end: close_paren.span().end(),
            },
            kind,
        });
        self.open_child(
            body,
            frames,
            if is_if { Owner::IfThen(statement) } else { Owner::While(statement) },
        )
    }

    pub(super) fn close_frame(
        &mut self,
        body: &mut Body,
        frames: &mut Vec<Frame>,
    ) -> Result<Option<Token>, ParseError> {
        let close = self.take(TokenKind::CloseBrace)?;
        let frame = frames.pop().expect("open block frame");
        let block_span = UntrustedSpan {
            file: self.file,
            start: frame.open.span().start(),
            end: close.span().end(),
        };
        let block = &mut body.blocks[frame.block];
        block.span = block_span;
        block.close_brace_span = raw(close);
        block.statements = frame.statements;
        if let Some(owner) = frame.owner {
            body.statements[owner.statement()].span.end = block_span.end;
        }
        if frames.is_empty() {
            return Ok(Some(close));
        }
        if let Some(Owner::WeakSuccess(index)) = frame.owner {
            self.take(TokenKind::Comma)?;
            self.take(TokenKind::OpenParen)?;
            self.take(TokenKind::CloseParen)?;
            let arrow = self.take(TokenKind::FatArrow)?;
            let failure = u32::try_from(body.blocks.len()).expect("bounded blocks");
            let syntax::RawStatementKind::WeakUpgrade { else_span, failure_block, .. } =
                &mut body.statements[index].kind
            else {
                unreachable!("weak owner")
            };
            *else_span = raw(arrow);
            *failure_block = failure;
            self.open_child(body, frames, Owner::WeakFailure(index))?;
        }
        if let Some(Owner::WeakFailure(index)) = frame.owner {
            self.take(TokenKind::CloseParen)?;
            let semicolon = self.take(TokenKind::Semicolon)?;
            body.statements[index].span.end = semicolon.span().end();
        }
        if let Some(Owner::IfThen(index)) = frame.owner
            && let Some(else_token) = self.maybe(TokenKind::Keyword(Keyword::Else))
        {
            let block = u32::try_from(body.blocks.len()).expect("bounded blocks");
            let syntax::RawStatementKind::If { else_clause, .. } = &mut body.statements[index].kind
            else {
                unreachable!("if owner")
            };
            *else_clause = Some(syntax::RawElseSyntax { keyword_span: raw(else_token), block });
            self.open_child(body, frames, Owner::IfElse(index))?;
        }
        Ok(None)
    }

    pub(super) fn open_child(
        &mut self,
        body: &mut Body,
        frames: &mut Vec<Frame>,
        owner: Owner,
    ) -> Result<(), ParseError> {
        if body.blocks.len() >= syntax::MAX_BLOCKS_PER_FUNCTION
            || self.previous_blocks + body.blocks.len() >= syntax::MAX_BLOCKS_PER_PROJECT
        {
            return Err(resource(if body.blocks.len() >= syntax::MAX_BLOCKS_PER_FUNCTION {
                "function exceeds the lexical-block limit"
            } else {
                "project exceeds the lexical-block limit"
            }));
        }
        if frames.len() >= syntax::MAX_NESTING_DEPTH as usize {
            return Err(resource("block nesting exceeds protocol-v5 limit"));
        }
        let open = self.take(TokenKind::OpenBrace)?;
        let block = body.blocks.len();
        body.blocks.push(unfinished_block(open));
        frames.push(Frame { block, open, owner: Some(owner), statements: Vec::new() });
        Ok(())
    }
}
