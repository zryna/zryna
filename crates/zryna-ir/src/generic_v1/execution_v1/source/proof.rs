//! Only complete source replay issues original-operation permission and a source-built world.
use super::{Failure, TypeWorld, charge, raw, reject};
mod upgrades;
#[derive(Debug)]
struct CloneSite {
    function: usize,
    block: u32,
    instruction: u32,
    ty: u32,
}
#[derive(Debug)]
pub(in crate::generic_v1::execution_v1) struct SourceProof {
    world: TypeWorld,
    clones: Vec<CloneSite>,
    upgrades: upgrades::Upgrades,
}
impl SourceProof {
    pub(in crate::generic_v1::execution_v1) fn world(&self) -> &TypeWorld {
        &self.world
    }
    pub(in crate::generic_v1::execution_v1) fn clone_allowed(
        &self,
        function: usize,
        block: u32,
        instruction: u32,
        ty: u32,
        spent: &mut usize,
    ) -> Result<bool, Failure> {
        let mut lo = 0;
        let mut hi = self.clones.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            charge(spent, 1)?;
            let site = &self.clones[mid];
            match (site.function, site.block, site.instruction).cmp(&(function, block, instruction))
            {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    charge(spent, 1)?;
                    return Ok(site.ty == ty && self.world.fact(ty).is_some_and(|f| f.clone));
                }
            }
        }
        Ok(false)
    }
    pub(in crate::generic_v1::execution_v1) fn upgrade_site(
        &self,
        function: usize,
        block: usize,
        weak: raw::PlaceOrLoan,
        failure: u32,
        work: &mut usize,
    ) -> Result<zryna_source::UntrustedSpan, Failure> {
        self.upgrades.site(function, block, weak, failure, work)
    }
    pub(super) fn new(
        world: TypeWorld,
        program: &raw::Program,
        spent: &mut usize,
    ) -> Result<Self, Failure> {
        let mut clones = Vec::new();
        charge(spent, program.functions.len())?;
        for (fi, function) in program.functions.iter().enumerate() {
            charge(spent, function.blocks.len())?;
            for block in &function.blocks {
                charge(spent, block.instructions.len())?;
                for (pi, instruction) in block.instructions.iter().enumerate() {
                    if let raw::Operation::Clone { source, .. } = instruction.operation {
                        charge(spent, 1)?;
                        let ty = match source {
                            raw::PlaceOrLoan::Place(id) => {
                                function.places.get(id as usize).map(|p| p.ty)
                            }
                            raw::PlaceOrLoan::Loan(id) => {
                                function.loans.get(id as usize).map(|l| l.referent)
                            }
                        }
                        .ok_or(Failure::InternalFailure)?;
                        if !world.fact(ty).is_some_and(|f| f.clone) {
                            return Err(reject("source-built Clone permission is absent"));
                        }
                        charge(spent, 1)?;
                        clones.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
                        clones.push(CloneSite {
                            function: fi,
                            block: block.id,
                            instruction: u32::try_from(pi).map_err(|_| Failure::InternalFailure)?,
                            ty,
                        });
                    }
                }
            }
        }
        let upgrades = upgrades::Upgrades::new(program, spent)?;
        Ok(Self { world, clones, upgrades })
    }
}
pub(super) fn compare(
    expected: &raw::Program,
    claim: &raw::Program,
    spent: &mut usize,
) -> Result<(), Failure> {
    comparison_credit(expected, spent)?;
    comparison_credit(claim, spent)?;
    if expected != claim {
        return Err(reject("complete execution claim differs from authenticated source replay"));
    }
    Ok(())
}
fn rows(spent: &mut usize, count: usize) -> Result<(), Failure> {
    charge(spent, count.checked_mul(2).ok_or(Failure::InternalFailure)?)
}
fn comparison_credit(p: &raw::Program, spent: &mut usize) -> Result<(), Failure> {
    charge(spent, p.language_contract.len() + 1)?;
    rows(spent, p.modules.len() + p.declarations.len() + p.type_keys.len() + p.functions.len())?;
    for key in &p.type_keys {
        charge(spent, key.len())?;
    }
    for f in &p.functions {
        charge(spent, f.key.len() + f.executable_export.as_ref().map_or(0, String::len))?;
        rows(
            spent,
            f.parameters.len()
                + f.places.len()
                + f.loans.len()
                + f.blocks.len()
                + f.cleanup_plans.len(),
        )?;
        for plan in &f.cleanup_plans {
            rows(spent, plan.actions.len())?;
        }
        for b in &f.blocks {
            rows(spent, b.value_parameters.len() + b.instructions.len())?;
            for i in &b.instructions {
                match &i.operation {
                    raw::Operation::StringFromUtf8 { bytes, .. } => charge(spent, bytes.len())?,
                    raw::Operation::ClosedGenericCall { arguments, .. }
                    | raw::Operation::SourceCall { arguments, .. } => rows(spent, arguments.len())?,
                    raw::Operation::StructConstruct { fields, .. } => rows(spent, fields.len())?,
                    raw::Operation::FixedArrayConstruct { elements, .. }
                    | raw::Operation::VecConstruct { elements, .. } => rows(spent, elements.len())?,
                    _ => {}
                }
            }
            let edge = |e: &raw::Edge, spent: &mut usize| rows(spent, e.arguments.len());
            match &b.terminator {
                raw::Terminator::Jump(e) => edge(e, spent)?,
                raw::Terminator::Branch { yes, no, .. } => {
                    edge(yes, spent)?;
                    edge(no, spent)?;
                }
                raw::Terminator::ClosedEnumMatch { arms, .. } => {
                    rows(spent, arms.len())?;
                    for a in arms {
                        edge(&a.edge, spent)?;
                    }
                }
                raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                    edge(success, spent)?;
                    edge(expired, spent)?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}
