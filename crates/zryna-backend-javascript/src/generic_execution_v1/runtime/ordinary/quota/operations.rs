use super::{
    Analysis, CloneNodeKind, Cost, Diagnostic, Function, Summary, cfg, charge, filled, internal,
    normalized, raw, source,
};
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, VerifiedOperation};

impl Analysis<'_, '_, '_> {
    pub(super) fn operation(
        &mut self,
        f: &Function<'_, '_>,
        op: VerifiedOperation<'_, '_>,
        v: &[Cost],
        p: &mut [Cost],
        depth: usize,
    ) -> Result<(Cost, Cost), Diagnostic> {
        use raw::Operation as O;
        let c = self.c;
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
            | O::IndexCopy { .. } => zero,
            O::ReadPlace { place, .. } => source(c, f, p, *place)?,
            O::InitializePlace { place, value } | O::ReplacePlace { place, value } => {
                let root = f.place(*place)?.root().id() as usize;
                p[root] = if root == *place as usize {
                    p[root].maximum(v[*value as usize])
                } else {
                    p[root].add(v[*value as usize])?
                };
                zero
            }
            O::Clone { source: raw::PlaceOrLoan::Place(id), .. } => {
                let clone = op.clone_frontier().ok_or_else(internal)?;
                let potential = if clone.frontier().kind() == CloneNodeKind::Copy {
                    zero
                } else {
                    source(c, f, p, *id)?
                };
                cost = potential;
                potential
            }
            O::StructConstruct { fields, .. } | O::FixedArrayConstruct { elements: fields, .. } => {
                sum(c, fields, v)?
            }
            O::ClosedEnumConstruct { payload, .. } => payload.map_or(zero, |id| v[id as usize]),
            O::StringFromUtf8 { bytes, .. } => {
                cost = Cost { allocations: usize::from(!bytes.is_empty()), ..one };
                cost
            }
            O::VecConstruct { elements, .. } => {
                cost = Cost { allocations: usize::from(!elements.is_empty()), ..one };
                sum(c, elements, v)?.add(cost)?
            }
            O::SharedConstruct { .. } => {
                cost = Cost { handles: 1, allocations: 1, releases: 2 };
                one
            }
            O::WeakDowngrade { .. } => {
                cost = one;
                one
            }
            O::VecPush { .. } => {
                let push = op.vec_push().ok_or_else(internal)?;
                let target = push.target_place().ok_or_else(internal)?;
                let root = target.root().id() as usize;
                let element = v[push.value().id() as usize];
                p[root] = if root == target.id() as usize {
                    let before = p[root];
                    Cost { allocations: before.allocations.max(1), ..before }.add(element)?
                } else {
                    p[root].add(element)?.add(Cost { allocations: 1, ..zero })?
                };
                // Reserve is one canonical transition, with at most one physical growth attempt.
                cost = Cost { handles: 1, allocations: 1, releases: 0 };
                zero
            }
            O::ClosedGenericCall { arguments, .. } | O::SourceCall { arguments, .. } => {
                let target = op.call_target().ok_or_else(internal)?.index();
                let mut args = filled(c, arguments.len(), zero)?;
                for (slot, arg) in args.iter_mut().zip(arguments) {
                    let raw::CallArgument::Value(id) = arg else {
                        return Err(internal());
                    };
                    *slot = v[*id as usize];
                }
                let summary = self.function(target, &args, depth + 1)?;
                cost = summary.cost;
                summary.result
            }
            // Ordinary proof must not silently bless a newly enabled runtime row.
            _ => return Err(internal()),
        };
        Ok((result, cost))
    }

    pub(super) fn terminator(
        &mut self,
        function: &Function<'_, '_>,
        block: VerifiedBlock<'_, '_>,
        values: &mut [Cost],
        places: &[Cost],
        summary: &mut Summary,
    ) -> Result<(), Diagnostic> {
        let context = self.c;
        let terminator = block.terminator();
        let payload = if let Some(upgrade) = block.weak_upgrade() {
            upgrade.source_place().ok_or_else(internal)?;
            context.control(upgrade.referent_type())?;
            let alias = Cost { handles: 1, allocations: 0, releases: 1 };
            // Expiry uses one status too; successful acquisition additionally funds one release.
            summary.cost = summary.cost.add(alias)?;
            alias
        } else {
            match terminator {
                raw::Terminator::Return { value, .. } => {
                    let potential = value.map_or(Cost::default(), |id| values[id as usize]);
                    summary.result = summary.result.maximum(normalized(
                        context,
                        function.view.result_type(),
                        potential,
                    )?);
                    Cost::default()
                }
                raw::Terminator::ClosedEnumMatch {
                    scrutinee: raw::PlaceOrLoan::Place(id), ..
                } => source(context, function, places, *id)?,
                raw::Terminator::Jump(_)
                | raw::Terminator::Branch { .. }
                | raw::Terminator::Trap { .. } => Cost::default(),
                _ => return Err(internal()),
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
                    values[*arg as usize],
                )?);
            }
        }
        Ok(())
    }
}

fn sum(c: &super::Context<'_, '_>, ids: &[u32], values: &[Cost]) -> Result<Cost, Diagnostic> {
    charge(&c.work, ids.len())?;
    ids.iter().try_fold(Cost::default(), |sum, id| sum.add(values[*id as usize]))
}
