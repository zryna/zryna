use super::super::diagnostics::span;
use super::super::layout_graph::semantic_type;
use super::{PrivateOwnedAggregateLowerer, StatementOutcome, Ty};
use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1::raw;
use zryna_syntax::v4::RawStatementKind;

pub(super) struct Scope {
    block: u32,
    next: usize,
    bindings: BTreeMap<String, super::Binding>,
    declared: BTreeSet<String>,
    aliases: BTreeSet<String>,
    owners: BTreeSet<raw::PlaceId>,
    pub(super) drop_credits: usize,
}

impl Scope {
    pub(super) fn add_owned_binding(&mut self, name: &str, owner: raw::PlaceId) {
        self.bindings.remove(name);
        self.owners.remove(&owner);
        self.drop_credits += 1;
    }

    pub(super) fn shadows_outer(
        &mut self,
        name: &str,
        root: u32,
        bindings: &BTreeMap<String, super::Binding>,
    ) -> bool {
        let duplicate = self.declared.iter().any(|declared| declared.eq_ignore_ascii_case(name));
        self.declared.insert(name.to_owned());
        !duplicate && self.block != root && bindings.contains_key(name)
    }
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn enter_lexical_scope(&self, block: u32) -> Scope {
        Scope {
            block,
            next: 0,
            bindings: self.bindings.clone(),
            declared: BTreeSet::new(),
            aliases: self.preparation_facts.aliases.keys().cloned().collect(),
            owners: self.owners.pending().iter().copied().collect(),
            drop_credits: 0,
        }
    }

    pub(super) fn lower_lexical_scope(&mut self, block: u32, result: Ty) -> Option<()> {
        let reserved = self.reserved_transitions;
        let outcome = self.lower_credited_scope(block, result);
        if outcome.is_none() {
            self.reserved_transitions = reserved;
        } else {
            assert_eq!(self.reserved_transitions, reserved, "lexical tail credits discharged");
        }
        outcome
    }

    fn lower_credited_scope(&mut self, block: u32, result: Ty) -> Option<()> {
        let mut scopes = vec![self.enter_lexical_scope(block)];
        while let Some(scope) = scopes.last_mut() {
            let block = self.function.body.blocks.get(scope.block as usize)?;
            let Some(&id) = block.statements.get(scope.next) else {
                let scope = scopes.pop()?;
                self.end_lexical_scope(&scope)?;
                continue;
            };
            scope.next += 1;
            let statement = self.function.body.statements.get(id as usize)?.clone();
            if let RawStatementKind::Block { block } = statement.kind {
                scopes.push(self.enter_lexical_scope(block));
                continue;
            }
            if matches!(statement.kind, RawStatementKind::Return { .. }) {
                self.errors.at(
                    "ZRYNA-M3017",
                    span(self.input.sources(), statement.span),
                    "indexed lexical access cannot cross a return or continuation",
                    "end lexical access before the final function return",
                );
                return None;
            }
            if let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind
                && !self.is_lexical_declaration(&statement)
            {
                let ty = semantic_type(
                    self.file,
                    type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )?;
                if !ty.is_copy() {
                    let at = span(self.input.sources(), statement.span);
                    if !self.reserve_transition(at) {
                        return None;
                    }
                    scope.drop_credits += 1;
                }
            }
            if !matches!(
                self.lower_statement(id, &statement, result, None, 0)?,
                StatementOutcome::Continue
            ) {
                return None;
            }
        }
        Some(())
    }

    pub(super) fn end_lexical_scope(&mut self, scope: &Scope) -> Option<()> {
        let at =
            span(self.input.sources(), self.function.body.blocks.get(scope.block as usize)?.span);
        let mut ended: Vec<_> = self
            .preparation_facts
            .aliases
            .iter()
            .filter(|(name, _)| !scope.aliases.contains(*name))
            .map(|(_, alias)| alias.borrow)
            .collect();
        ended.sort_unstable_by_key(|id| std::cmp::Reverse(id.0));
        let dropped: Vec<_> = self
            .owners
            .pending()
            .iter()
            .rev()
            .copied()
            .filter(|owner| !scope.owners.contains(owner))
            .collect();
        assert!(dropped.len() <= scope.drop_credits, "each scoped owner reserves its drop");
        for _ in 0..ended.len().checked_add(scope.drop_credits)? {
            self.release_transition();
        }
        if !self.preflight_transition(ended.len().checked_add(dropped.len())?, at) {
            return None;
        }
        for borrow in ended {
            self.emit_prepared_effect(at, raw::InstructionKind::EndBorrow { borrow });
            self.preparation_facts.active_borrows.remove(&borrow)?;
        }
        self.preparation_facts.aliases.retain(|name, _| scope.aliases.contains(name));
        for place in dropped {
            self.emit_prepared_effect(at, raw::InstructionKind::DropPlace { place });
            let delta = self.owners.consume_owner(place)?;
            self.preparation_facts.apply(delta);
        }
        self.bindings.clone_from(&scope.bindings);
        Some(())
    }
}
