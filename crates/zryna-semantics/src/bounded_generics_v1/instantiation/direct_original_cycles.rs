//! Layout-owned sufficient self-cycle proof from actual original generic member roots only.
use super::{
    BodyTypeContext, DeclarationIdentity, DeclarationKind, InstantiationFailure, TypeShape,
    diagnostics::Errors,
};
use zryna_layout::generic_v1::{
    original_cycles::{
        Argument, DirectMember, NominalIdentity, ParameterIdentity, rejects_direct_self_member,
    },
    raw::NominalKind,
};
use zryna_syntax::v5::RawDataDeclarationKind;

pub(super) fn collect(
    bodies: &BodyTypeContext<'_, '_>,
    errors: &mut Errors,
) -> Result<(), InstantiationFailure> {
    let declarations = bodies.declarations();
    for module in declarations.modules() {
        let unit = &declarations.syntax().files()[module.identity().index() as usize];
        for declaration in module.data_declarations() {
            let arity = declaration.type_parameters().count();
            if !(1..=2).contains(&arity) {
                continue;
            }
            let owner = declaration.identity();
            let original = &unit.data_declarations[owner.source_index() as usize];
            // This bounded borrowed pass visits F member roots, not descendant nominal heads.
            // At most two immediate arguments are observed per candidate: O(D+F), constant
            // witness space, no substituted keys, closed probes, reachability or unfolding.
            match &original.kind {
                RawDataDeclarationKind::Struct { fields, .. } => {
                    for (ordinal, field) in fields.iter().enumerate() {
                        member(bodies, owner, arity, ordinal, field.type_syntax, errors)?;
                    }
                }
                RawDataDeclarationKind::Enum { variants, .. } => {
                    for (ordinal, variant) in variants.iter().enumerate() {
                        if let Some(occurrence) = variant.payload_type {
                            member(bodies, owner, arity, ordinal, occurrence, errors)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn member(
    bodies: &BodyTypeContext<'_, '_>,
    owner: DeclarationIdentity,
    arity: usize,
    ordinal: usize,
    occurrence: u32,
    errors: &mut Errors,
) -> Result<(), InstantiationFailure> {
    let view =
        bodies.source_type(owner, occurrence).ok_or(InstantiationFailure::InternalFailure)?;
    let TypeShape::Nominal(target) = view.shape() else { return Ok(()) };
    let mut arguments = [Argument::Other, Argument::Other];
    let mut count = 0;
    for (index, child) in view.children().enumerate() {
        let slot = arguments.get_mut(index).ok_or(InstantiationFailure::InternalFailure)?;
        if let TypeShape::Parameter(parameter) = child.shape()
            && parameter.declaration().kind() != DeclarationKind::Function
        {
            *slot = Argument::Parameter(ParameterIdentity {
                owner: nominal(parameter.declaration())?,
                slot: parameter.index(),
            });
        }
        count += 1;
    }
    let witness = DirectMember {
        owner: nominal(owner)?,
        parameters: u32::try_from(arity).map_err(|_| InstantiationFailure::InternalFailure)?,
        target: nominal(target)?,
        arguments: &arguments[..count],
    };
    if rejects_direct_self_member(&witness) {
        let node = &bodies.declarations().syntax().files()[owner.module().index() as usize]
            .type_syntax[occurrence as usize];
        let numeric = [
            owner.module().index() as usize,
            owner.source_index() as usize,
            ordinal,
            occurrence as usize,
        ];
        errors.at(bodies, "ZRYNA-L3002", Some(node.span), &[], &numeric,
            "original generic stored member repeats its own complete nominal type under every substitution, forming a by-value self cycle".into())?;
    }
    Ok(())
}

fn nominal(owner: DeclarationIdentity) -> Result<NominalIdentity, InstantiationFailure> {
    let kind = match owner.kind() {
        DeclarationKind::Struct => NominalKind::Struct,
        DeclarationKind::Enum => NominalKind::Enum,
        DeclarationKind::Function => return Err(InstantiationFailure::InternalFailure),
    };
    Ok(NominalIdentity { module: owner.module().index(), declaration: owner.source_index(), kind })
}
