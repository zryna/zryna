use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawStatementKind;

use super::super::diagnostics::span;
use super::super::layout_graph::semantic_type;
use super::structured_graph::StructuredGraph;
use super::{PrivateOwnedAggregateLowerer, StatementOutcome, Ty};

#[cfg(test)]
#[path = "../tests/structured_cfg_resources.rs"]
pub(super) mod resources;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn lower_structured_cfg(
        &mut self,
        parameters: &[raw::ValueDefinition],
        result: Ty,
    ) -> Option<Vec<raw::Block>> {
        let mut errors = super::Errors::new(self.input.sources());
        let mut scratch = self.structured_scratch(&mut errors);
        let outcome = scratch.lower_structured_cfg_inner(parameters, result);
        if outcome.is_some() {
            super::structured_checkpoint::StructuredCheckpoint::capture(&scratch).restore(self);
        }
        self.errors.append(errors);
        outcome
    }

    fn lower_structured_cfg_inner(
        &mut self,
        parameters: &[raw::ValueDefinition],
        result: Ty,
    ) -> Option<Vec<raw::Block>> {
        let at = span(self.input.sources(), self.function.body.span);
        let [held_blocks, held_edges] = super::structured_graph::held_resources();
        let mut graph = StructuredGraph::new(self.function, held_blocks, held_edges, at, self)?;
        if self.structured_scope(self.function.body.root_block, result, &mut graph)? {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "structured owned function has a reachable fallthrough",
                "return one exact result on every terminating path",
            );
            return None;
        }
        graph.finish(self, parameters, at)
    }

    pub(super) fn structured_scope(
        &mut self,
        block: u32,
        result: Ty,
        graph: &mut StructuredGraph,
    ) -> Option<bool> {
        let scope = self.enter_lexical_scope(block);
        self.structured_scope_from(block, result, graph, scope)
    }

    #[allow(clippy::too_many_lines)]
    fn structured_scope_from(
        &mut self,
        block: u32,
        result: Ty,
        graph: &mut StructuredGraph,
        mut scope: super::lexical_indexed_scope::Scope,
    ) -> Option<bool> {
        let body = self.function.body.blocks.get(block as usize)?.clone();
        let mut fallthrough = true;
        for id in body.statements {
            let statement = self.function.body.statements.get(id as usize)?.clone();
            let statement_span = span(self.input.sources(), statement.span);
            if !fallthrough {
                self.errors.at(
                    "ZRYNA-M3015",
                    statement_span,
                    "statement follows a terminated ownership path",
                    "remove unreachable statements after a return",
                );
                return None;
            }
            match statement.kind {
                RawStatementKind::Block { block } => {
                    fallthrough = self.structured_scope(block, result, graph)?;
                }
                RawStatementKind::If { condition, then_block, else_clause, .. } => {
                    fallthrough = self.structured_if(
                        condition,
                        then_block,
                        else_clause.map(|clause| clause.block),
                        result,
                        statement_span,
                        graph,
                    )?;
                }
                RawStatementKind::While { condition, body_block, .. } => {
                    self.structured_loop(condition, body_block, result, statement_span, graph)?;
                }
                RawStatementKind::WeakUpgrade {
                    weak,
                    binding,
                    success_block,
                    failure_block,
                    ..
                } => {
                    fallthrough = self.structured_weak_upgrade(
                        weak,
                        &binding,
                        success_block,
                        failure_block,
                        result,
                        statement_span,
                        graph,
                    )?;
                }
                RawStatementKind::Return { value, .. } => {
                    for _ in 0..scope.drop_credits {
                        self.release_transition();
                    }
                    scope.drop_credits = 0;
                    self.structured_return(value, result, statement_span, graph)?;
                    fallthrough = false;
                }
                RawStatementKind::Assignment { target, value, .. }
                    if (self.is_vec_index(target) || self.is_checked_array_index(target))
                        && graph.contains_match(statement.span.start, statement.span.end) =>
                {
                    let ty = self.indexed_expression_type(target)?;
                    self.structured_indexed_operation(target, value, ty, true, graph)?;
                }
                _ => {
                    let mut shadow_outer = false;
                    if let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind
                        && !self.is_lexical_declaration(&statement)
                    {
                        let RawStatementKind::LocalDeclaration { ref name, .. } = statement.kind
                        else {
                            unreachable!("local declaration selected")
                        };
                        shadow_outer = scope.shadows_outer(
                            &name.text,
                            self.function.body.root_block,
                            &self.bindings,
                        );
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
                            if !self.reserve_transition(statement_span) {
                                return None;
                            }
                            scope.drop_credits += 1;
                        }
                        if self.structured_match_local(&statement, ty, graph)? {
                            continue;
                        }
                    }
                    if !matches!(
                        self.lower_statement_with_shadow(
                            id,
                            &statement,
                            result,
                            None,
                            0,
                            shadow_outer,
                        )?,
                        StatementOutcome::Continue
                    ) {
                        return None;
                    }
                }
            }
        }
        self.finish_structured_scope(&scope, fallthrough)
    }

    fn structured_return(
        &mut self,
        value: u32,
        result: Ty,
        at: Span,
        graph: &mut StructuredGraph,
    ) -> Option<()> {
        let value = self.structured_value(value, result, graph)?;
        let mut ended: Vec<_> = self.preparation_facts.active_borrows.keys().copied().collect();
        ended.reverse();
        for borrow in ended {
            self.release_transition();
            if !self.emit_effect(at, raw::InstructionKind::EndBorrow { borrow }) {
                return None;
            }
            self.preparation_facts.active_borrows.remove(&borrow);
        }
        self.preparation_facts
            .aliases
            .retain(|_, alias| self.preparation_facts.parameter_borrows.contains(&alias.borrow));
        let cleanup = self.push_cleanup(at, self.owners.owner(value))?;
        graph.terminate(self, at, raw::Terminator::Return { value, cleanup })?;
        Some(())
    }

    fn finish_structured_scope(
        &mut self,
        scope: &super::lexical_indexed_scope::Scope,
        fallthrough: bool,
    ) -> Option<bool> {
        if fallthrough {
            self.end_lexical_scope(scope)?;
        } else {
            for _ in 0..scope.drop_credits {
                self.release_transition();
            }
        }
        Some(fallthrough)
    }

    pub(super) fn structured_scope_with_owned_binding(
        &mut self,
        block: u32,
        result: Ty,
        name: String,
        binding: super::Binding,
        at: Span,
        graph: &mut StructuredGraph,
    ) -> Option<bool> {
        let mut scope = self.enter_lexical_scope(block);
        if !self.reserve_transition(at) {
            return None;
        }
        scope.add_owned_binding(&name, binding.place);
        self.bindings.insert(name, binding);
        self.structured_scope_from(block, result, graph, scope)
    }

    fn structured_condition(&mut self, condition: u32) -> Option<raw::ValueId> {
        let ty = self
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::Bool)
            .copied()?;
        self.value(condition, ty)
    }

    fn structured_if(
        &mut self,
        condition: u32,
        then_block: u32,
        else_block: Option<u32>,
        result: Ty,
        at: Span,
        graph: &mut StructuredGraph,
    ) -> Option<bool> {
        self.join_state(at)?;
        let condition = self.structured_condition(condition)?;
        let incoming = self.join_state(at)?;
        let origin = graph.terminate(
            self,
            at,
            raw::Terminator::Branch {
                condition,
                when_true: raw::Edge { target: raw::BlockId(0), arguments: Vec::new() },
                when_false: raw::Edge { target: raw::BlockId(0), arguments: Vec::new() },
            },
        )?;
        let when_true = graph.next(self, at)?;
        let then_falls = self.structured_scope(then_block, result, graph)?;
        let then_state = if then_falls { Some(self.join_state(at)?) } else { None };
        let then_jump =
            if then_falls { Some(graph.jump(self, at, raw::BlockId(0))?) } else { None };
        self.restore_join(&incoming);
        let when_false = graph.next(self, at)?;
        let else_falls = if let Some(block) = else_block {
            self.structured_scope(block, result, graph)?
        } else {
            true
        };
        if else_falls && let Some(state) = &then_state {
            self.reconcile_join(state, at)?;
        }
        let else_jump =
            if else_falls { Some(graph.jump(self, at, raw::BlockId(0))?) } else { None };
        if !else_falls && let Some(state) = &then_state {
            self.restore_join(state);
        }
        let raw::Terminator::Branch { when_true: yes, when_false: no, .. } =
            &mut graph.blocks[origin].terminator.as_mut()?.kind
        else {
            return None;
        };
        yes.target = when_true;
        no.target = when_false;
        if then_falls || else_falls {
            let join = graph.next(self, at)?;
            if let Some(block) = then_jump {
                graph.retarget_jump(block, join);
            }
            if let Some(block) = else_jump {
                graph.retarget_jump(block, join);
            }
            Some(true)
        } else {
            Some(false)
        }
    }

    fn structured_loop(
        &mut self,
        condition: u32,
        body: u32,
        result: Ty,
        at: Span,
        graph: &mut StructuredGraph,
    ) -> Option<()> {
        self.join_state(at)?;
        // A loop may change bytes while restoring exactly the same ownership state.
        // Forget compile-time byte lengths before planning its repeatedly executed header.
        self.preparation_facts.string_bytes.clear();
        let header_state = self.join_state(at)?;
        let preheader = graph.jump(self, at, raw::BlockId(0))?;
        let header = graph.next(self, at)?;
        graph.retarget_jump(preheader, header);
        let condition = self.structured_condition(condition)?;
        let exit_state = self.join_state(at)?;
        let branch = graph.terminate(
            self,
            at,
            raw::Terminator::Branch {
                condition,
                when_true: raw::Edge { target: raw::BlockId(0), arguments: Vec::new() },
                when_false: raw::Edge { target: raw::BlockId(0), arguments: Vec::new() },
            },
        )?;
        let body_id = graph.next(self, at)?;
        if self.structured_scope(body, result, graph)? {
            self.reconcile_join(&header_state, at)?;
            graph.jump(self, at, header)?;
        }
        self.restore_join(&exit_state);
        let exit = graph.next(self, at)?;
        let raw::Terminator::Branch { when_true, when_false, .. } =
            &mut graph.blocks[branch].terminator.as_mut()?.kind
        else {
            return None;
        };
        when_true.target = body_id;
        when_false.target = exit;
        Some(())
    }
}
