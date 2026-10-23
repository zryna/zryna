//! Reject-only cardinality over bounded original tokens, without type normalization or allocation.
//!
//! A complete supported token grammar and outer close are required before reporting item three.
//! Unsupported/ambiguous syntax defers to the existing parser. This grants no raw syntax seal.

use crate::native_lexer::{Keyword, Token, TokenKind};

#[derive(Default)]
struct Items {
    count: usize,
    first: Option<Token>,
    last: Option<Token>,
    extra: Option<(Token, Token)>,
}
impl Items {
    fn begin(&mut self, token: Token) -> Option<()> {
        self.count = self.count.checked_add(1)?;
        if self.count == 3 {
            self.first = Some(token);
        }
        Some(())
    }
    fn token(&mut self, token: Token) {
        if self.count == 3 {
            self.last = Some(token);
        }
    }
    fn finish(&mut self) {
        if self.count == 3 {
            self.extra = self.first.zip(self.last);
        }
    }
}

#[derive(Clone, Copy)]
enum TypeState {
    Head { trailing: bool },
    Named { application: bool },
    Integer,
    Applied,
}

/// `start` follows `<`. Nested list arity and depth are deliberately not semantic checks here.
/// Canonical unsigned integer occurrences cover `FixedArray` lengths; the normal parser still owns
/// their placement/meaning when the list has one/two items. Counters are bounded by source tokens.
pub(super) fn arguments(
    tokens: &[Token],
    start: usize,
    name: impl Fn(Token) -> bool,
    integer: impl Fn(Token) -> bool,
) -> Option<(Token, Token)> {
    let mut items = Items::default();
    let mut depth = 0_usize;
    let mut state = TypeState::Head { trailing: false };
    for token in tokens.iter().copied().skip(start) {
        match (state, token.kind()) {
            (TypeState::Head { .. }, _) if name(token) || integer(token) => {
                if depth == 0 {
                    items.begin(token)?;
                }
                state = if name(token) {
                    // The existing parser permits the contextual bare name, but explicitly
                    // rejects `function<...>`; do not turn that syntax failure into arity.
                    TypeState::Named {
                        application: token.kind() != TokenKind::Keyword(Keyword::Function),
                    }
                } else {
                    TypeState::Integer
                };
            }
            (TypeState::Named { application: true }, TokenKind::LessThan) => {
                depth = depth.checked_add(1)?;
                state = TypeState::Head { trailing: false };
            }
            (
                TypeState::Named { .. } | TypeState::Integer | TypeState::Applied,
                TokenKind::Comma,
            ) => {
                if depth == 0 {
                    items.finish();
                    state = TypeState::Head { trailing: true };
                    continue;
                }
                state = TypeState::Head { trailing: true };
            }
            (
                TypeState::Named { .. }
                | TypeState::Integer
                | TypeState::Applied
                | TypeState::Head { trailing: true },
                TokenKind::GreaterThan | TokenKind::GreaterEqual,
            ) => {
                if depth == 0 {
                    items.finish();
                    return items.extra;
                }
                if token.kind() == TokenKind::GreaterEqual {
                    return None;
                }
                depth = depth.checked_sub(1)?;
                state = TypeState::Applied;
            }
            _ => return None,
        }
        items.token(token);
    }
    None
}

/// Exact source parameter grammar: name, `extends`, named bound, with optional trailing comma.
pub(super) fn parameters(
    tokens: &[Token],
    start: usize,
    name: impl Fn(Token) -> bool,
    bound: impl Fn(Token) -> bool,
) -> Option<(Token, Token)> {
    let mut items = Items::default();
    let mut stage = 0_u8;
    let mut trailing = false;
    for token in tokens.iter().copied().skip(start) {
        match (stage, token.kind()) {
            (0, TokenKind::GreaterThan) if trailing => return items.extra,
            (0, _) if name(token) => {
                items.begin(token)?;
                stage = 1;
            }
            (1, TokenKind::Keyword(Keyword::Extends)) => stage = 2,
            (2, _) if bound(token) => stage = 3,
            (3, TokenKind::Comma) => {
                items.finish();
                stage = 0;
                trailing = true;
                continue;
            }
            (3, TokenKind::GreaterThan) => {
                items.finish();
                return items.extra;
            }
            _ => return None,
        }
        items.token(token);
    }
    None
}
