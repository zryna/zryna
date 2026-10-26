//! Source-built arm ranges authorize their own join, never a caller-installed CFG permission.
use super::{Failure, Matches, charge, raw, reject, reserve, resource_failure};
use zryna_source::UntrustedSpan;
#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(in crate::generic_v1::execution_v1) struct Scope {
    pub(in crate::generic_v1::execution_v1) entry: usize,
    pub(in crate::generic_v1::execution_v1) parent: u32,
    pub(in crate::generic_v1::execution_v1) blocks: std::ops::Range<usize>,
    pub(in crate::generic_v1::execution_v1) join: usize,
    pub(in crate::generic_v1::execution_v1) region: UntrustedSpan,
}

impl Matches {
    pub(in crate::generic_v1::execution_v1::source) fn scopes(
        &self,
        function: usize,
        f: &raw::Function,
        work: &mut usize,
    ) -> Result<Vec<Scope>, Failure> {
        charge(work, 1)?;
        let sites = self.0.get(function).ok_or(Failure::InternalFailure)?;
        charge(
            work,
            sites
                .len()
                .checked_mul(2)
                .ok_or_else(|| resource_failure("source match sparse scan census overflow"))?,
        )?;
        let count = entries(sites).try_fold(0usize, |n, site| {
            n.checked_add(site.arms.len())
                .ok_or_else(|| resource_failure("source match scope census overflow"))
        })?;
        charge(
            work,
            count
                .checked_mul(size_of::<Scope>() + 8)
                .ok_or_else(|| resource_failure("source match scope storage overflow"))?,
        )?;
        let mut scopes = reserve(count)?;
        for site in entries(sites) {
            let block =
                f.blocks.get(site.block).ok_or_else(|| reject("match scope entry is foreign"))?;
            self.site(function, site.block, f, &block.terminator, work)?;
            let raw::Terminator::ClosedEnumMatch { arms, .. } = &block.terminator else {
                return Err(Failure::InternalFailure);
            };
            if site.join >= f.blocks.len() || site.join <= site.block {
                return Err(reject("match scope join is outside its original CFG"));
            }
            for (arm, original) in arms.iter().zip(&site.arms) {
                if arm.edge.target as usize != original.blocks.start
                    || original.blocks.start >= original.blocks.end
                    || original.blocks.end > site.join
                    || original.blocks.start <= site.block
                {
                    return Err(reject("match scope differs from source-created arm blocks"));
                }
                scopes.push(Scope {
                    entry: site.block,
                    parent: site.parent,
                    blocks: original.blocks.clone(),
                    join: site.join,
                    region: original.region,
                });
            }
        }
        Ok(scopes)
    }
}

fn entries(sites: &[Option<super::Use>]) -> impl Iterator<Item = &super::Use> {
    sites
        .iter()
        .inspect(|_| {
            #[cfg(test)]
            SCANNED.with(|count| count.set(count.get() + 1));
        })
        .flatten()
}

#[cfg(test)]
std::thread_local! {
    static SCANNED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
