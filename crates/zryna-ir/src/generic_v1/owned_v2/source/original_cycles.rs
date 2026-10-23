//! Independent rejection-only proof of an exact original generic member-root self edge.
use crate::generic_v1::{
    Failure,
    source::{Originals, Target},
};
use zryna_layout::generic_v1::{
    original_cycles::{
        Argument, DirectMember, NominalIdentity, ParameterIdentity, rejects_direct_self_member,
    },
    raw::NominalKind,
};
use zryna_source::{SourceMap, UntrustedSpan};
use zryna_syntax::v5::{RawDataDeclaration, RawDataDeclarationKind, RawTypeSyntaxKind};

pub(super) fn check(originals: &Originals<'_>, sources: &SourceMap) -> Result<(), Failure> {
    // Verified v5 syntax already bounds the complete declaration/member inventory.
    // Each borrowed member root has at most two direct argument observations; there
    // is no recursive argument/body traversal, retained table or copied key.
    let mut first: Option<UntrustedSpan> = None;
    for unit in originals.units {
        for (index, declaration) in unit.data_declarations.iter().enumerate() {
            let Some(parameters) = declaration.type_parameters.as_ref() else { continue };
            if !(1..=2).contains(&parameters.parameters.len()) {
                continue;
            }
            let owner = identity(unit.id, index, declaration)?;
            let mut consider = |occurrence| -> Result<(), Failure> {
                if let Some(at) = member(originals, declaration, &owner, occurrence)?
                    && first.is_none_or(|earlier| {
                        (at.file, at.start, at.end) < (earlier.file, earlier.start, earlier.end)
                    })
                {
                    first = Some(at);
                }
                Ok(())
            };
            match &declaration.kind {
                RawDataDeclarationKind::Struct { fields, .. } => {
                    for field in fields {
                        consider(field.type_syntax)?;
                    }
                }
                RawDataDeclarationKind::Enum { variants, .. } => {
                    for variant in variants {
                        if let Some(occurrence) = variant.payload_type {
                            consider(occurrence)?;
                        }
                    }
                }
            }
        }
    }
    let Some(at) = first else { return Ok(()) };
    let at = sources.verify_span(at).map_err(|_| Failure::InternalFailure)?;
    let diagnostic = zryna_diagnostics::Diagnostic::error_at(
        "ZRYNA-L3002",
        at,
        "original generic stored member has the same nominal identity and ordered parameter tuple as its owner",
        "break the by-value storage cycle with an admitted indirection",
    );
    Err(Failure::Diagnostics(vec![diagnostic]))
}

fn identity(
    module: u32,
    index: usize,
    declaration: &RawDataDeclaration,
) -> Result<NominalIdentity, Failure> {
    Ok(NominalIdentity {
        module,
        declaration: u32::try_from(index).map_err(|_| Failure::InternalFailure)?,
        kind: match declaration.kind {
            RawDataDeclarationKind::Struct { .. } => NominalKind::Struct,
            RawDataDeclarationKind::Enum { .. } => NominalKind::Enum,
        },
    })
}

fn member(
    originals: &Originals<'_>,
    declaration: &RawDataDeclaration,
    owner: &NominalIdentity,
    occurrence: u32,
) -> Result<Option<UntrustedSpan>, Failure> {
    let Some(parameters) = declaration.type_parameters.as_ref() else { return Ok(None) };
    let arity = parameters.parameters.len();
    if !(1..=2).contains(&arity)
        || (arity == 2 && parameters.parameters[0].name.text == parameters.parameters[1].name.text)
    {
        return Ok(None);
    }
    let types = &originals.units[owner.module as usize].type_syntax;
    let node = types.get(occurrence as usize).ok_or(Failure::InternalFailure)?;
    let RawTypeSyntaxKind::Application { name, type_arguments } = &node.kind else {
        return Ok(None);
    };
    if type_arguments.arguments.len() != arity
        || parameters.parameters.iter().any(|parameter| parameter.name.text == name.text)
    {
        return Ok(None);
    }
    let Ok(Target::Data(module, index)) = originals.resolve(owner.module, &name.text) else {
        return Ok(None);
    };
    let target_declaration = &originals.units[module as usize].data_declarations[index as usize];
    let target = identity(module, index as usize, target_declaration)?;
    let mut arguments = [Argument::Other, Argument::Other];
    for (ordinal, occurrence) in type_arguments.arguments.iter().enumerate() {
        let argument = types.get(*occurrence as usize).ok_or(Failure::InternalFailure)?;
        let RawTypeSyntaxKind::Named { name } = &argument.kind else { continue };
        if let Some(slot) =
            parameters.parameters.iter().position(|parameter| parameter.name.text == name.text)
        {
            arguments[ordinal] = Argument::Parameter(ParameterIdentity {
                owner: owner.clone(),
                slot: u32::try_from(slot).map_err(|_| Failure::InternalFailure)?,
            });
        }
    }
    let proof = DirectMember {
        owner: owner.clone(),
        parameters: u32::try_from(arity).map_err(|_| Failure::InternalFailure)?,
        target,
        arguments: &arguments[..arity],
    };
    Ok(rejects_direct_self_member(&proof).then_some(node.span))
}
