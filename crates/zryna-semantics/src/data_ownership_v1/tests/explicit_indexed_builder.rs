use super::*;

#[path = "explicit_indexed_call_builder.rs"]
mod calls;

pub(super) struct Builder<'a> {
    pub(super) source: String,
    pub(super) types: &'a mut Vec<RawTypeSyntax>,
    pub(super) body: &'a mut RawFunctionBodySyntax,
    pub(super) slot: Option<u32>,
}

impl Builder<'_> {
    pub(super) fn callee(
        &mut self,
        element: u32,
        exclusive: bool,
        copy: bool,
    ) -> RawFunctionSyntax {
        calls::callee(self, element, exclusive, copy)
    }
    fn position(&self) -> u32 {
        u32::try_from(self.source.len()).expect("fixture position")
    }
    fn text(&mut self, text: &str) -> UntrustedSpan {
        let start = self.position();
        self.source.push_str(text);
        at(start, self.position())
    }
    fn name(&mut self, text: &str) -> RawIdentifierSyntax {
        RawIdentifierSyntax { text: text.into(), span: self.text(text) }
    }
    fn expression(&mut self, start: u32, kind: RawExpressionKind) -> u32 {
        let id = u32::try_from(self.body.expressions.len()).expect("expression");
        self.body.expressions.push(RawExpressionSyntax { span: at(start, self.position()), kind });
        id
    }
    fn reference(&mut self, text: &str) -> u32 {
        let start = self.position();
        let name = self.name(text);
        self.expression(start, RawExpressionKind::Reference { name })
    }
    fn cloned(&mut self, name: &str) -> u32 {
        let start = self.position();
        let keyword_span = self.text("clone");
        let open_paren_span = self.text("(");
        let value = self.reference(name);
        let close_paren_span = self.text(")");
        self.expression(
            start,
            RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span },
        )
    }
    fn ty(&mut self, id: u32) -> u32 {
        let old = self.types[id as usize].clone();
        let start = self.position();
        let spelling = self.source[old.span.start as usize..old.span.end as usize].to_owned();
        self.text(&spelling);
        self.copied_type(id, start - old.span.start)
    }
    fn copied_type(&mut self, id: u32, delta: u32) -> u32 {
        let old = self.types[id as usize].clone();
        let mut json = serde_json::to_value(&old).expect("type");
        shift(&mut json, 0, delta);
        let mut copied: RawTypeSyntax = serde_json::from_value(json).expect("shifted type");
        match &mut copied.kind {
            RawTypeSyntaxKind::Vec { argument, .. }
            | RawTypeSyntaxKind::Shared { argument, .. }
            | RawTypeSyntaxKind::Weak { argument, .. }
            | RawTypeSyntaxKind::FixedArray { element: argument, .. } => {
                *argument = self.copied_type(*argument, delta);
            }
            _ => {}
        }
        let result = u32::try_from(self.types.len()).expect("type");
        self.types.push(copied);
        result
    }
    fn statement(&mut self, start: u32, kind: RawStatementKind) -> u32 {
        let id = u32::try_from(self.body.statements.len()).expect("statement");
        self.body.statements.push(RawStatementSyntax { span: at(start, self.position()), kind });
        self.text(" ");
        id
    }
    fn alias(
        &mut self,
        element: u32,
        exclusive: bool,
        index: Option<i32>,
        alias_name: &str,
        whole: bool,
    ) -> u32 {
        let start = self.position();
        let keyword_span = self.text("const");
        self.text(" ");
        let name = self.name(alias_name);
        self.text(": ");
        let type_start = self.position();
        let type_keyword = self.text(if exclusive { "BorrowMut" } else { "Borrow" });
        let less_than_span = self.text("<");
        let argument = self.ty(element);
        let greater_than_span = self.text(">");
        let type_syntax = u32::try_from(self.types.len()).expect("borrow type");
        let kind = if exclusive {
            RawTypeSyntaxKind::BorrowMut {
                keyword_span: type_keyword,
                less_than_span,
                argument,
                greater_than_span,
            }
        } else {
            RawTypeSyntaxKind::Borrow {
                keyword_span: type_keyword,
                less_than_span,
                argument,
                greater_than_span,
            }
        };
        self.types.push(RawTypeSyntax { span: at(type_start, self.position()), kind });
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let borrow_start = self.position();
        let borrow_keyword = self.text(if exclusive { "borrowMut" } else { "borrow" });
        let open_paren_span = self.text("(");
        let index_start = self.position();
        let mut base = self.reference("items");
        if let Some(slot) = self.slot {
            let open_bracket_span = self.text("[");
            let start = self.position();
            let spelling = slot.to_string();
            self.text(&spelling);
            let index = self.expression(start, RawExpressionKind::I32Literal { spelling });
            let close_bracket_span = self.text("]");
            base = self.expression(
                index_start,
                RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
            );
        }
        let value = if whole {
            base
        } else {
            let open_bracket_span = self.text("[");
            let index = if let Some(index) = index {
                let start = self.position();
                let spelling = index.to_string();
                self.text(&spelling);
                self.expression(start, RawExpressionKind::I32Literal { spelling })
            } else {
                self.reference("index")
            };
            let close_bracket_span = self.text("]");
            self.expression(
                index_start,
                RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
            )
        };
        let close_paren_span = self.text(")");
        let kind = if exclusive {
            RawExpressionKind::BorrowMut {
                keyword_span: borrow_keyword,
                open_paren_span,
                value,
                close_paren_span,
            }
        } else {
            RawExpressionKind::Borrow {
                keyword_span: borrow_keyword,
                open_paren_span,
                value,
                close_paren_span,
            }
        };
        let initializer = self.expression(borrow_start, kind);
        let semicolon_span = self.text(";");
        self.statement(
            start,
            RawStatementKind::LocalDeclaration {
                keyword_span,
                mutable: false,
                name,
                type_syntax,
                equals_span,
                initializer,
                semicolon_span,
            },
        )
    }
    fn action(&mut self, element: u32, action: Action) -> u32 {
        let start = self.position();
        if matches!(action, Action::GrowthInside | Action::GrowthAfter) {
            let keyword_span = self.text("push");
            let open_paren_span = self.text("(");
            let vector = self.reference("items");
            let comma_span = self.text(",");
            self.text(" ");
            let value = self.reference("next");
            let close_paren_span = self.text(")");
            let expression = self.expression(
                start,
                RawExpressionKind::VecPush {
                    keyword_span,
                    open_paren_span,
                    vector,
                    comma_span,
                    value,
                    close_paren_span,
                },
            );
            let semicolon_span = self.text(";");
            return self.statement(
                start,
                RawStatementKind::ExpressionStatement { expression, semicolon_span },
            );
        }
        let kind =
            if matches!(action, Action::Replace | Action::ReplaceClone | Action::OwnerReplace) {
                let target = self.reference(if matches!(action, Action::OwnerReplace) {
                    "items"
                } else {
                    "loan"
                });
                self.text(" ");
                let equals_span = self.text("=");
                self.text(" ");
                let value = if matches!(action, Action::OwnerReplace) {
                    self.cloned("items")
                } else if matches!(action, Action::ReplaceClone) {
                    self.cloned("next")
                } else {
                    self.reference("next")
                };
                RawStatementKind::Assignment {
                    target,
                    equals_span,
                    value,
                    semicolon_span: self.text(";"),
                }
            } else {
                let keyword_span = self.text("const");
                self.text(" ");
                let name =
                    self.name(if matches!(action, Action::Escape) { "escaped" } else { "seen" });
                self.text(": ");
                let selected = if matches!(action, Action::OwnerMove) {
                    let RawStatementKind::LocalDeclaration { type_syntax, .. } =
                        self.body.statements[0].kind
                    else {
                        panic!("container local");
                    };
                    type_syntax
                } else {
                    element
                };
                let type_syntax = self.ty(selected);
                self.text(" ");
                let equals_span = self.text("=");
                self.text(" ");
                let initializer = if matches!(action, Action::OwnerMove) {
                    self.reference("items")
                } else if matches!(action, Action::Clone | Action::Escape) {
                    self.cloned("loan")
                } else if matches!(action, Action::Call) {
                    calls::call(self)
                } else {
                    self.reference("loan")
                };
                RawStatementKind::LocalDeclaration {
                    keyword_span,
                    mutable: false,
                    name,
                    type_syntax,
                    equals_span,
                    initializer,
                    semicolon_span: self.text(";"),
                }
            };
        self.statement(start, kind)
    }
    pub(super) fn lexical(
        &mut self,
        element: u32,
        exclusive: bool,
        action: Action,
        index: Option<i32>,
    ) {
        let open_brace_span = self.text("{");
        let block = u32::try_from(self.body.blocks.len()).expect("block");
        let statement = u32::try_from(self.body.statements.len()).expect("block statement");
        self.body.statements.push(RawStatementSyntax {
            span: open_brace_span,
            kind: RawStatementKind::Block { block },
        });
        self.text(" ");
        if matches!(action, Action::SiblingVec | Action::SameVec) {
            self.slot = Some(0);
        }
        let mut statements = vec![self.alias(element, exclusive, index, "loan", false)];
        statements.push(match action {
            Action::SiblingVec | Action::SameVec => {
                self.slot = Some(u32::from(matches!(action, Action::SiblingVec)));
                self.alias(element, true, Some(1), "other", false)
            }
            Action::Conflict => self.alias(element, true, Some(1), "other", false),
            Action::Collision => self.alias(element, false, index, "Loan", false),
            Action::RootShared => {
                let RawStatementKind::LocalDeclaration { type_syntax, .. } =
                    self.body.statements[0].kind
                else {
                    panic!("container local");
                };
                self.alias(type_syntax, false, None, "whole", true)
            }
            Action::Escape | Action::GrowthAfter => self.action(element, Action::Clone),
            _ => self.action(element, action),
        });
        let close_brace_span = self.text("}");
        self.body.blocks.push(RawBlockSyntax {
            span: at(open_brace_span.start, close_brace_span.end),
            open_brace_span,
            statements,
            close_brace_span,
        });
        self.body.statements[statement as usize].span =
            at(open_brace_span.start, close_brace_span.end);
        self.text(" ");
        self.body.blocks[0].statements = vec![0, 1, statement];
        if matches!(action, Action::Escape | Action::GrowthAfter) {
            let escaped = self.action(element, action);
            self.body.blocks[0].statements.push(escaped);
        }
    }
    pub(super) fn finish(&mut self) {
        let start = self.position();
        let keyword_span = self.text("return");
        self.text(" ");
        let value = self.reference("items");
        let semicolon_span = self.text(";");
        let statement =
            self.statement(start, RawStatementKind::Return { keyword_span, value, semicolon_span });
        self.body.blocks[0].statements.push(statement);
        let close = self.text("}");
        self.body.blocks[0].close_brace_span = close;
        self.body.blocks[0].span.end = close.end;
        self.body.span.end = close.end;
    }
}
