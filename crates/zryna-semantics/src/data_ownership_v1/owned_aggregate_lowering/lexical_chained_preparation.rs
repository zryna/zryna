use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::Ty;
use super::super::diagnostics::span;
use super::indexed_vec_preparation::IndexedObservation;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::Operation;

enum ChainBoundary {
    Static,
    Checked(usize),
}

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn chained_lexical_begin(
        &mut self,
        target: u32,
        expected: Ty,
        write: bool,
    ) -> Option<IndexedObservation> {
        let mut root = target;
        let mut path = Vec::new();
        while let RawExpressionKind::Index { base, index, .. } =
            self.decisions.function.body.expressions.get(root as usize)?.kind
        {
            path.push((base, index));
            root = base;
        }
        if path.len() < 2 {
            return Some(IndexedObservation::Unselected);
        }
        path.reverse();
        let root = self.resolve(root)?;
        let at = span(
            self.decisions.input.sources(),
            self.decisions.function.body.expressions.get(target as usize)?.span,
        );
        let first = self.lexical_chain_boundary(&path, root.ty, expected, at)?;
        let ChainBoundary::Checked(first) = first else {
            return Some(IndexedObservation::Unselected);
        };
        // A static prefix remains the conflict region, preserving proven sibling disjointness.
        let source = self.resolve(path[first].0)?;
        self.available_vector(source, write, at)?;
        let integer = self.decisions.primitive(TypeCategory::I32)?;
        let start = self.steps.len();
        self.push(
            Operation::IndexedEnter { end: usize::MAX, result: usize::MAX },
            integer,
            at,
            None,
        );
        let mut parent = None;
        let mut carrier = None;
        for (_, expression) in &path[first..] {
            let index = self.walk(*expression, integer)?;
            carrier = Some(index);
            parent = Some(if let Some(parent) = parent {
                let borrow = raw::BorrowId(self.state.facts.next_borrow);
                let cleanup = self.reverse(expected, at)?;
                self.indexed_effect(
                    raw::InstructionKind::ProjectIndexedBorrow { parent, borrow, index, cleanup },
                    expected,
                    at,
                )?;
                borrow
            } else {
                self.begin_access(source, index, write, expected, at)?
            });
        }
        let borrow = raw::BorrowId(self.state.facts.next_borrow);
        self.indexed_effect(
            raw::InstructionKind::BindIndexedBorrow { parent: parent?, borrow },
            expected,
            at,
        )?;
        let value = carrier?;
        let result = self.steps.iter().rposition(|step| step.value == Some(value))?;
        let end = self.steps.len();
        self.steps[start].operation = Operation::IndexedEnter { end, result };
        self.push(Operation::IndexedExit, integer, at, None);
        Some(IndexedObservation::Value(value))
    }

    fn lexical_chain_boundary(
        &mut self,
        path: &[(u32, u32)],
        mut ty: Ty,
        expected: Ty,
        at: Span,
    ) -> Option<ChainBoundary> {
        let mut first = None;
        for (ordinal, (_, index)) in path.iter().enumerate() {
            let record = self.decisions.layouts.type_by_id(ty.layout)?;
            let checked = match ty.category {
                TypeCategory::FixedArray => {
                    super::ordinary_indexed_array_preparation::checked_index(
                        &self.decisions.function.body.expressions.get(*index as usize)?.kind,
                        record.array_length()?,
                    )
                }
                TypeCategory::Vec => true,
                _ => {
                    self.lexical_chain_type_error(at);
                    return None;
                }
            };
            if checked && first.is_none() {
                first = Some(ordinal);
            }
            let element = record.referenced_type()?;
            ty = self
                .decisions
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.layout == element)
                .copied()?;
        }
        if ty != expected {
            self.lexical_chain_type_error(at);
            return None;
        }
        Some(first.map_or(ChainBoundary::Static, ChainBoundary::Checked))
    }

    fn lexical_chain_type_error(&mut self, at: Span) {
        self.decisions.errors.at(
            "ZRYNA-M3017",
            at,
            "indexed borrow annotation does not match its exact element type",
            "borrow one array or Vec element with its exact referent type",
        );
    }
}
