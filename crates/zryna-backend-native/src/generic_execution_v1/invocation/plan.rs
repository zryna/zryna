//! Checked source-only invocation additions, paid before any code-generation reserve.
use super::status::Cases;
use crate::generic_execution_v1::{codegen, invariant};
use zryna_abi::{Invocation, ScalarType, VerifiedInvocation};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, VerifiedCloneNode, VerifiedOperation};
use zryna_native_mir::generic_execution_v1::{MAX_CODEGEN_UNITS, VerifiedProgram};
pub(in crate::generic_execution_v1) struct Plan<'p> {
    pub request: VerifiedInvocation<'p>,
    pub target: usize,
    rows: Vec<Vec<Vec<Cases>>>,
    pub work: u64,
}
fn budget() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N7001",
        None,
        "full execution invocation exceeds its checked budget",
        "reduce physical type, parameter or code amplification",
    )
}
fn take(work: &mut u64, units: u64) -> Result<(), Diagnostic> {
    *work = work.checked_add(units).filter(|n| *n <= MAX_CODEGEN_UNITS).ok_or_else(budget)?;
    Ok(())
}
fn reserve<T>(count: usize) -> Result<Vec<T>, Diagnostic> {
    let mut v = Vec::new();
    v.try_reserve_exact(count).map_err(codegen)?;
    Ok(v)
}
impl<'p> Plan<'p> {
    pub fn derive(p: &VerifiedProgram<'p, '_>, request: Invocation) -> Result<Self, Diagnostic> {
        Self::from_work(p, request, p.codegen_work_units())
    }
    pub fn from_work(
        p: &VerifiedProgram<'p, '_>,
        request: Invocation,
        mut work: u64,
    ) -> Result<Self, Diagnostic> {
        // Signature validation itself allocates no code-generation data.
        let request = p.program().scalar_abi().prepare_invocation(request).map_err(|error| {
            Diagnostic::error(
                error.code(),
                None,
                "invocation does not match its exact scalar export",
                "use the selected export and its exact typed argument list",
            )
        })?;
        let n = u64::try_from(request.arguments().len()).map_err(codegen)?;
        let instruction_count = 46u64
            .checked_add(n.checked_mul(5).ok_or_else(budget)?)
            .and_then(|x| {
                x.checked_add(u64::from(request.export().result() == ScalarType::Bool) * 4)
            })
            .and_then(|x| x.checked_add(u64::from(p.requires_recursive_arena()) * 30))
            .ok_or_else(budget)?;
        let wrapper = instruction_count
            .checked_mul(128)
            .and_then(|x| x.checked_add(n.checked_add(2)?.checked_mul(16)?))
            .ok_or_else(budget)?;
        take(&mut work, wrapper)?;
        // Complete source census before outer/inner tables or Clone traversal scratch.
        take(&mut work, (p.functions().len() as u64).checked_mul(32).ok_or_else(budget)?)?;
        for f in p.functions() {
            take(
                &mut work,
                (f.source().blocks().len() as u64).checked_mul(32).ok_or_else(budget)?,
            )?;
            for b in f.source().blocks() {
                take(&mut work, (b.operations().len() as u64).checked_mul(16).ok_or_else(budget)?)?;
            }
        }
        let mut rows = reserve(p.functions().len())?;
        for f in p.functions() {
            let mut blocks = reserve(f.source().blocks().len())?;
            for b in f.source().blocks() {
                let mut ops = reserve(b.operations().len())?;
                for op in b.operations() {
                    let clone = if let Some(clone) = op.clone_frontier() {
                        clone_cases(p, clone.frontier(), &mut work)?
                    } else {
                        Cases::default()
                    };
                    let cases = super::status::operation(op, clone)?;
                    if op.failure().is_some() {
                        pay(&mut work, cases)?;
                    }
                    if op.vec_push().is_some() {
                        pay(&mut work, Cases::capacity())?;
                    }
                    ops.push(cases);
                }
                if b.weak_upgrade().is_some() {
                    pay(&mut work, Cases::count())?;
                }
                blocks.push(ops);
            }
            rows.push(blocks);
        }
        let target =
            *p.program().export_functions().get(request.export().index()).ok_or_else(invariant)?;
        Ok(Self { request, target, rows, work })
    }
    pub(super) fn operation(&self, op: VerifiedOperation<'_, '_>) -> Result<Cases, Diagnostic> {
        let site = op.site();
        self.rows
            .get(site.function)
            .and_then(|f| f.get(site.block as usize))
            .and_then(|b| b.get(site.instruction as usize))
            .copied()
            .ok_or_else(invariant)
    }
}
fn pay(work: &mut u64, cases: Cases) -> Result<(), Diagnostic> {
    if cases.len() != 0 {
        take(
            work,
            cases
                .len()
                .checked_mul(4)
                .and_then(|n| n.checked_add(1))
                .and_then(|n| n.checked_mul(128))
                .ok_or_else(budget)?,
        )?;
    }
    Ok(())
}
fn clone_cases(
    p: &VerifiedProgram<'_, '_>,
    root: VerifiedCloneNode<'_, '_>,
    work: &mut u64,
) -> Result<Cases, Diagnostic> {
    let n = p.program().type_count();
    let mut members = 0u64;
    take(work, (n as u64).checked_mul(4).ok_or_else(budget)?)?;
    for id in 0..n {
        let ty = p.stored_type(u32::try_from(id).map_err(codegen)?).ok_or_else(invariant)?;
        members = members
            .checked_add(ty.fields().len() as u64)
            .and_then(|x| x.checked_add(ty.variants().len() as u64))
            .ok_or_else(budget)?;
    }
    let unit = u64::try_from(size_of::<VerifiedCloneNode<'_, '_>>()).map_err(codegen)?;
    take(
        work,
        (n as u64)
            .checked_mul(unit.checked_add(8).ok_or_else(budget)?)
            .and_then(|x| x.checked_add(members.checked_mul(3)?))
            .ok_or_else(budget)?,
    )?;
    let mut seen = reserve(n)?;
    seen.resize(n, false);
    let mut queue = reserve(n)?;
    queue.push(root);
    seen[root.ty() as usize] = true;
    let mut result = Cases::default();
    let mut cursor = 0;
    while cursor < queue.len() {
        let node = queue[cursor];
        cursor += 1;
        match node.kind() {
            CloneNodeKind::String | CloneNodeKind::VecEach => result.union(Cases::allocation()),
            CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => result.union(Cases::count()),
            _ => {}
        }
        let children = node
            .fields()
            .map(|(_, x)| x)
            .chain(node.variants().filter_map(|(_, x)| x))
            .chain(node.element());
        for child in children {
            let slot = seen.get_mut(child.ty() as usize).ok_or_else(invariant)?;
            if !*slot {
                *slot = true;
                queue.push(child);
            }
        }
    }
    Ok(result)
}
