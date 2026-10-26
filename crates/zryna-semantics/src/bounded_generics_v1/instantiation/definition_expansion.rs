//! Rejection-only direct definition proof; no symbolic term becomes a closed instance.
use super::diagnostics::Errors;
use super::model::Builder;
use super::{DeclarationIdentity, DeclarationKind, InstantiationFailure, TypeShape, reserve};
use zryna_syntax::v5::RawTypeSyntaxKind;

mod unification;

#[derive(Clone, Copy)]
struct Term {
    owner: DeclarationIdentity,
    parameters: u8,
    variable: Option<u8>,
}

struct Work {
    remaining: usize,
}
impl Work {
    fn new(nodes: usize) -> Result<Self, InstantiationFailure> {
        // Per original arena: N reserved summaries + N source visits + at most 2N child
        // edges. Each of at most N candidates has at most two equations; an equation
        // costs one visit, at most three alias checks, and at most two occurs visits.
        // This derived 16N invariant is not a new language/resource ceiling.
        let remaining = nodes
            .checked_mul(1 + 1 + 2 + 2 * (1 + 3 + 2))
            .ok_or(InstantiationFailure::InternalFailure)?;
        Ok(Self { remaining })
    }
    fn take(&mut self, amount: usize) -> Result<(), InstantiationFailure> {
        self.remaining =
            self.remaining.checked_sub(amount).ok_or(InstantiationFailure::InternalFailure)?;
        Ok(())
    }
}

pub(super) fn collect(
    builder: &Builder<'_, '_, '_>,
    errors: &mut Errors,
) -> Result<(), InstantiationFailure> {
    let bodies = builder.bodies;
    let declarations = bodies.declarations();
    for module in declarations.modules() {
        let unit = &declarations.syntax().files()[module.identity().index() as usize];
        let mut work = Work::new(unit.type_syntax.len())?;
        work.take(unit.type_syntax.len())?;
        let mut terms = reserve(unit.type_syntax.len())?;
        terms.resize(unit.type_syntax.len(), None);
        // Source authentication makes child occurrences strictly earlier. A shared child
        // reuses one summary: neither nesting nor DAG fan-in causes repeated subtree walks.
        for (owner, occurrence) in bodies.source_owners(module.identity().index()) {
            work.take(1)?;
            if owner.kind() == DeclarationKind::Function {
                continue;
            }
            let declaration =
                declarations.declaration(owner).ok_or(InstantiationFailure::InternalFailure)?;
            let arity = declaration.type_parameters().count();
            if arity > 2 {
                return Err(InstantiationFailure::InternalFailure);
            }
            let node = &unit.type_syntax[occurrence];
            let children = children(&node.kind)?;
            let mut parameters = 0;
            let mut arguments = [None; 2];
            for (index, child) in children.into_iter().enumerate() {
                let Some(child) = child else { continue };
                work.take(1)?;
                if child >= occurrence {
                    return Err(InstantiationFailure::InternalFailure);
                }
                let term: Term = terms[child].ok_or(InstantiationFailure::InternalFailure)?;
                if term.owner != owner {
                    return Err(InstantiationFailure::InternalFailure);
                }
                parameters |= term.parameters;
                arguments[index] = Some(term);
            }
            let view = bodies
                .source_type(
                    owner,
                    u32::try_from(occurrence).map_err(|_| InstantiationFailure::InternalFailure)?,
                )
                .ok_or(InstantiationFailure::InternalFailure)?;
            let variable = if let TypeShape::Parameter(parameter) = view.shape() {
                if parameter.declaration() != owner || parameter.index() as usize >= arity {
                    return Err(InstantiationFailure::InternalFailure);
                }
                let index = u8::try_from(parameter.index())
                    .map_err(|_| InstantiationFailure::InternalFailure)?;
                parameters = 1u8 << index;
                Some(index)
            } else {
                None
            };
            terms[occurrence] = Some(Term { owner, parameters, variable });
            if arity == 0 || view.shape() != TypeShape::Nominal(owner) {
                continue;
            }
            if unification::expands(arguments, arity, &mut work)? {
                let witness =
                    [owner.module().index() as usize, owner.source_index() as usize, occurrence];
                errors.at(bodies, "ZRYNA-M7003", Some(node.span), &[], &witness,
                    "declaration-generated direct application cannot unify with its original parameter tuple; finite type arguments would contain themselves".into())?;
            }
        }
    }
    Ok(())
}

fn children(kind: &RawTypeSyntaxKind) -> Result<[Option<usize>; 2], InstantiationFailure> {
    let mut result = [None; 2];
    match kind {
        RawTypeSyntaxKind::Application { type_arguments, .. } => {
            if type_arguments.arguments.len() > 2 {
                return Err(InstantiationFailure::InternalFailure);
            }
            for (index, child) in type_arguments.arguments.iter().enumerate() {
                result[index] = Some(*child as usize);
            }
        }
        RawTypeSyntaxKind::Vec { argument, .. }
        | RawTypeSyntaxKind::Shared { argument, .. }
        | RawTypeSyntaxKind::Weak { argument, .. }
        | RawTypeSyntaxKind::Borrow { argument, .. }
        | RawTypeSyntaxKind::BorrowMut { argument, .. } => result[0] = Some(*argument as usize),
        RawTypeSyntaxKind::FixedArray { element, .. } => result[0] = Some(*element as usize),
        RawTypeSyntaxKind::Missing
        | RawTypeSyntaxKind::Named { .. }
        | RawTypeSyntaxKind::String { .. } => {}
    }
    Ok(result)
}
