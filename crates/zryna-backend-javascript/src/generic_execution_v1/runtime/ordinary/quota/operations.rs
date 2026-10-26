use super::{
    Analysis, Argument, CloneNodeKind, Cost, Diagnostic, Function, Summary, cfg, charge, filled,
    internal, normalized, raw,
};
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, VerifiedOperation};

impl Analysis<'_, '_, '_> {
    pub(super) fn operation(
        &mut self,
        f: &Function<'_, '_>,
        op: VerifiedOperation<'_, '_>,
        v: &[Cost],
        frame: &super::frame::Frame,
        depth: usize,
    ) -> Result<(Cost, Cost), Diagnostic> {
        use raw::Operation as O;
        let p = &frame.places;
        let loans = &frame.loans;
        let zero = Cost::default();
        let one = Cost { handles: 1, allocations: 0, releases: 1 };
        let mut cost = zero;
        let result = match op.kind() {
            O::Unit
            | O::BoolLiteral(_)
            | O::I32Literal(_)
            | O::ScalarBinary { .. }
            | O::I32Neg { .. }
            | O::DropPlace { .. }
            | O::IndexCopy { .. }
            | O::BeginLoan { .. }
            | O::EndLoan { .. }
            | O::ReadLoan { .. } => zero,
            O::ReadPlace { place, .. } => {
                self.observed(f, p, loans, raw::PlaceOrLoan::Place(*place))?
            }
            O::InitializePlace { place, value } | O::ReplacePlace { place, value } => {
                let target = Self::target(f, p, loans, raw::PlaceOrLoan::Place(*place))?;
                let before = self.cells[target.cell];
                self.cells[target.cell] = if target.projected {
                    before.add(self.operand(v, &frame.temporary, *value))?
                } else {
                    before.maximum(self.operand(v, &frame.temporary, *value))
                };
                zero
            }
            O::ReplaceLoan { loan, value } => {
                let target = loans[*loan as usize];
                let before = self.cells[target.cell];
                self.cells[target.cell] = if target.projected {
                    before.add(self.operand(v, &frame.temporary, *value))?
                } else {
                    before.maximum(self.operand(v, &frame.temporary, *value))
                };
                zero
            }
            O::Clone { source, .. } => {
                let clone = op.clone_frontier().ok_or_else(internal)?;
                let potential = if clone.frontier().kind() == CloneNodeKind::Copy {
                    zero
                } else {
                    self.observed(f, p, loans, *source)?
                };
                cost = potential;
                potential
            }
            O::StructConstruct { fields, .. } | O::FixedArrayConstruct { elements: fields, .. } => {
                self.sum(fields, v, frame)?
            }
            O::ClosedEnumConstruct { payload, .. } => {
                payload.map_or(zero, |id| self.operand(v, &frame.temporary, id))
            }
            O::StringConcat { .. } => {
                cost = Cost { allocations: 1, ..one };
                cost
            }
            O::StringFromUtf8 { bytes, .. } => {
                cost = Cost { allocations: usize::from(!bytes.is_empty()), ..one };
                cost
            }
            O::VecConstruct { elements, .. } => {
                cost = Cost { allocations: usize::from(!elements.is_empty()), ..one };
                self.sum(elements, v, frame)?.add(cost)?
            }
            O::SharedConstruct { .. } => {
                cost = Cost { handles: 1, allocations: 1, releases: 2 };
                one
            }
            O::WeakDowngrade { .. } => {
                cost = one;
                one
            }
            O::VecPush { vector, value, .. } => {
                let target = Self::target(f, p, loans, *vector)?;
                let before = self.cells[target.cell];
                let with_buffer = if target.projected {
                    before.add(Cost { allocations: 1, ..zero })?
                } else {
                    Cost { allocations: before.allocations.max(1), ..before }
                };
                self.cells[target.cell] =
                    with_buffer.add(self.operand(v, &frame.temporary, *value))?;
                cost = Cost { handles: 1, allocations: 1, releases: 0 };
                zero
            }
            O::ClosedGenericCall { arguments, .. } | O::SourceCall { arguments, .. } => {
                let summary = self.call_effect(op, arguments, v, frame, depth)?;
                cost = summary.cost;
                summary.result
            } // Exhaustive operation matching makes future rows require an explicit proof update.
        };
        Ok((result, cost))
    }

    fn call_effect(
        &mut self,
        op: VerifiedOperation<'_, '_>,
        arguments: &[raw::CallArgument],
        values: &[Cost],
        frame: &super::frame::Frame,
        depth: usize,
    ) -> Result<Summary, Diagnostic> {
        let c = self.c;
        let loans = &frame.loans;
        let target = op.call_target().ok_or_else(internal)?.index();
        let mut args = filled(c, arguments.len(), Argument::default())?;
        charge(&c.work, arguments.len())?;
        for (slot, arg) in args.iter_mut().zip(arguments) {
            *slot = match arg {
                raw::CallArgument::Value(id) => {
                    Argument::Value(self.operand(values, &frame.temporary, *id))
                }
                raw::CallArgument::Loan(id) => Argument::Loan(loans[*id as usize]),
            };
        }
        self.function(target, &args, depth + 1)
    }

    pub(super) fn terminator(
        &mut self,
        function: &Function<'_, '_>,
        block: VerifiedBlock<'_, '_>,
        values: &mut [Cost],
        frame: &super::frame::Frame,
        summary: &mut Summary,
    ) -> Result<(), Diagnostic> {
        let context = self.c;
        let places = &frame.places;
        let loans = &frame.loans;
        let terminator = block.terminator();
        let payload = if let Some(upgrade) = block.weak_upgrade() {
            context.control(upgrade.referent_type())?;
            let alias = Cost { handles: 1, allocations: 0, releases: 1 };
            // Expiry uses one status too; successful acquisition additionally funds one release.
            summary.cost = summary.cost.add(alias)?;
            alias
        } else {
            match terminator {
                raw::Terminator::Return { value, .. } => {
                    let potential = value
                        .map_or(Cost::default(), |id| self.operand(values, &frame.temporary, id));
                    summary.result = summary.result.maximum(normalized(
                        context,
                        function.view.result_type(),
                        potential,
                    )?);
                    Cost::default()
                }
                raw::Terminator::ClosedEnumMatch { scrutinee, .. } => {
                    self.observed(function, places, loans, *scrutinee)?
                }
                raw::Terminator::Jump(_)
                | raw::Terminator::Branch { .. }
                | raw::Terminator::Trap { .. } => Cost::default(),
                raw::Terminator::WeakUpgradeBranch { .. } => return Err(internal()),
            }
        };
        let count = match terminator {
            raw::Terminator::ClosedEnumMatch { arms, .. } => arms.len(),
            raw::Terminator::Jump(_) => 1,
            raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
            _ => 0,
        };
        charge(&context.work, count)?;
        for (edge, binding) in cfg::edges(terminator) {
            let target = function.blocks[edge.target as usize];
            let skip = usize::from(binding.is_some());
            if let Some(binding) = binding {
                values[binding.id as usize] =
                    values[binding.id as usize].maximum(normalized(context, binding.ty, payload)?);
            }
            charge(&context.work, edge.arguments.len())?;
            for (arg, param) in edge.arguments.iter().zip(target.value_parameters().skip(skip)) {
                values[param.id() as usize] = values[param.id() as usize].maximum(normalized(
                    context,
                    param.ty(),
                    self.operand(values, &frame.temporary, *arg),
                )?);
            }
        }
        Ok(())
    }
}

impl Analysis<'_, '_, '_> {
    fn sum(
        &self,
        ids: &[u32],
        values: &[Cost],
        frame: &super::frame::Frame,
    ) -> Result<Cost, Diagnostic> {
        charge(&self.c.work, ids.len())?;
        ids.iter().try_fold(Cost::default(), |total, id| {
            total.add(self.operand(values, &frame.temporary, *id))
        })
    }
}
