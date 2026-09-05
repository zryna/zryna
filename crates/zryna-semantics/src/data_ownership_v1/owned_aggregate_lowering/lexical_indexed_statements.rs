use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_syntax::v4::{
    RawExpressionKind, RawFunctionSyntax, RawStatementKind, RawStatementSyntax, RawTypeSyntaxKind,
};

use super::super::diagnostics::span;
use super::super::layout_graph::semantic_type;
use super::PrivateOwnedAggregateLowerer;
use super::constructor_preparation::PreparedValue;
use super::preparation_plan::LexicalAlias;

pub(in crate::data_ownership_v1) fn has_indexed_borrow(function: &RawFunctionSyntax) -> bool {
    let has_vec_construction = function
        .body
        .expressions
        .iter()
        .any(|expression| matches!(expression.kind, RawExpressionKind::VecConstruction { .. }));
    function.body.expressions.iter().any(|expression| {
        let value = match expression.kind {
            RawExpressionKind::Borrow { value, .. }
            | RawExpressionKind::BorrowMut { value, .. } => value,
            _ => return false,
        };
        function.body.expressions.get(value as usize).is_some_and(|expression| {
            let RawExpressionKind::Index { index, .. } = expression.kind else { return false };
            !function.parameters.is_empty()
                || has_vec_construction
                || function.body.expressions.get(index as usize).is_some_and(|expression| {
                    !matches!(expression.kind, RawExpressionKind::I32Literal { .. })
                })
        })
    })
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn is_lexical_declaration(&self, statement: &RawStatementSyntax) -> bool {
        let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind else {
            return false;
        };
        self.file.type_syntax().get(type_syntax as usize).is_some_and(|ty| {
            matches!(
                ty.kind,
                RawTypeSyntaxKind::Borrow { .. } | RawTypeSyntaxKind::BorrowMut { .. }
            )
        })
    }

    pub(super) fn lower_lexical_declaration(
        &mut self,
        statement: &RawStatementSyntax,
    ) -> Option<()> {
        let RawStatementKind::LocalDeclaration { mutable, name, type_syntax, initializer, .. } =
            &statement.kind
        else {
            return None;
        };
        let at = span(self.input.sources(), statement.span);
        let (argument, access) = match self.file.type_syntax().get(*type_syntax as usize)?.kind {
            RawTypeSyntaxKind::Borrow { argument, .. } => (argument, raw::BorrowAccess::Shared),
            RawTypeSyntaxKind::BorrowMut { argument, .. } => {
                (argument, raw::BorrowAccess::Exclusive)
            }
            _ => return None,
        };
        let ty = semantic_type(
            self.file,
            argument,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        let operand = match self.expression(*initializer)?.kind {
            RawExpressionKind::Borrow { value, .. } if access == raw::BorrowAccess::Shared => {
                Some(value)
            }
            RawExpressionKind::BorrowMut { value, .. }
                if access == raw::BorrowAccess::Exclusive =>
            {
                Some(value)
            }
            _ => None,
        };
        if *mutable || operand.is_none() || !super::mixed_shape::supported(ty, self.layouts) {
            self.errors.at("ZRYNA-M3017", at, "indexed alias requires a const binding, matching access and exact supported referent",
                "initialize const Borrow with borrow or const BorrowMut with borrowMut");
            return None;
        }
        if self
            .bindings
            .keys()
            .chain(self.preparation_facts.aliases.keys())
            .any(|key| key.eq_ignore_ascii_case(&name.text))
        {
            self.errors.at(
                "ZRYNA-M3002",
                at,
                "borrow alias collides with an existing binding",
                "use distinct names throughout the active lexical scope",
            );
            return None;
        }
        let integer = self
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::I32)
            .copied()?;
        let operand = operand?;
        if !self.reserve_transition(at) {
            return None;
        }
        let Some(prepared) = PreparedValue::prepare_lexical_begin(
            self,
            operand,
            ty,
            access == raw::BorrowAccess::Exclusive,
            integer,
        ) else {
            self.release_transition();
            return None;
        };
        prepared.consume();
        let borrow = raw::BorrowId(self.preparation_facts.next_borrow.checked_sub(1)?);
        self.preparation_facts
            .aliases
            .insert(name.text.clone(), LexicalAlias { borrow, ty, access });
        Some(())
    }

    pub(super) fn lexical_assignment(&mut self, target: u32, value: u32) -> Option<bool> {
        let RawExpressionKind::Reference { name } = &self.expression(target)?.kind else {
            return Some(false);
        };
        let Some(alias) = self.preparation_facts.aliases.get(&name.text).copied() else {
            return Some(false);
        };
        PreparedValue::prepare_lexical_replacement(self, value, alias)?.consume();
        Some(true)
    }
}
