//! Exact owner/binder frames reconstructed from authenticated external roots and forest edges.
use super::{Failure, add, charge, reserve};
use zryna_syntax::{
    v4::RawStatementKind,
    v5::{
        RawDataDeclarationKind, RawExpressionKind, RawSourceUnit, RawTypeParameterList,
        RawTypeSyntaxKind,
    },
};

pub(super) struct Node {
    pub owner: usize,
    pub parent: Option<usize>,
}
pub(super) struct Forest<'a> {
    pub parameters: Vec<Option<&'a RawTypeParameterList>>,
    pub nodes: Vec<Node>,
}

pub(super) fn children(kind: &RawTypeSyntaxKind) -> &[u32] {
    match kind {
        RawTypeSyntaxKind::Vec { argument, .. }
        | RawTypeSyntaxKind::Shared { argument, .. }
        | RawTypeSyntaxKind::Weak { argument, .. }
        | RawTypeSyntaxKind::Borrow { argument, .. }
        | RawTypeSyntaxKind::BorrowMut { argument, .. } => std::slice::from_ref(argument),
        RawTypeSyntaxKind::FixedArray { element, .. } => std::slice::from_ref(element),
        RawTypeSyntaxKind::Application { type_arguments, .. } => &type_arguments.arguments,
        _ => &[],
    }
}

pub(super) fn forest<'u>(
    unit: &'u RawSourceUnit,
    spent: &mut usize,
) -> Result<Forest<'u>, Failure> {
    charge(
        spent,
        add(
            add(unit.data_declarations.len(), unit.functions.len())?,
            unit.type_syntax.len().checked_mul(2).ok_or(Failure::InternalFailure)?,
        )?,
    )?;
    let mut result = Forest {
        parameters: reserve(unit.data_declarations.len() + unit.functions.len())?,
        nodes: reserve(unit.type_syntax.len())?,
    };
    result.nodes.resize_with(unit.type_syntax.len(), || Node { owner: usize::MAX, parent: None });
    let mut pending = reserve(unit.type_syntax.len())?;
    for data in &unit.data_declarations {
        let owner = result.parameters.len();
        result.parameters.push(data.type_parameters.as_ref());
        match &data.kind {
            RawDataDeclarationKind::Struct { fields, .. } => {
                for field in fields {
                    assign(unit, &mut result, &mut pending, owner, field.type_syntax, spent)?;
                }
            }
            RawDataDeclarationKind::Enum { variants, .. } => {
                for variant in variants {
                    if let Some(id) = variant.payload_type {
                        assign(unit, &mut result, &mut pending, owner, id, spent)?;
                    }
                }
            }
        }
    }
    for function in &unit.functions {
        let owner = result.parameters.len();
        result.parameters.push(function.type_parameters.as_ref());
        for parameter in &function.parameters {
            assign(unit, &mut result, &mut pending, owner, parameter.type_syntax, spent)?;
        }
        assign(unit, &mut result, &mut pending, owner, function.result_type, spent)?;
        charge(
            spent,
            add(
                function.parameters.len(),
                add(function.body.statements.len(), function.body.expressions.len())?,
            )?,
        )?;
        for statement in &function.body.statements {
            if let RawStatementKind::LocalDeclaration { type_syntax, .. } = statement.kind {
                assign(unit, &mut result, &mut pending, owner, type_syntax, spent)?;
            }
        }
        for expression in &function.body.expressions {
            match &expression.kind {
                RawExpressionKind::VecConstruction { type_syntax, .. }
                | RawExpressionKind::FixedArrayConstruction { type_syntax, .. } => {
                    assign(unit, &mut result, &mut pending, owner, *type_syntax, spent)?;
                }
                RawExpressionKind::Call { type_arguments, .. }
                | RawExpressionKind::StructConstruction { type_arguments, .. }
                | RawExpressionKind::EnumConstruction { type_arguments, .. } => {
                    if let Some(list) = type_arguments {
                        for id in &list.arguments {
                            assign(unit, &mut result, &mut pending, owner, *id, spent)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    charge(spent, result.nodes.len())?;
    if result.nodes.iter().any(|node| node.owner == usize::MAX) {
        return Err(Failure::InternalFailure);
    }
    Ok(result)
}

fn assign(
    unit: &RawSourceUnit,
    forest: &mut Forest<'_>,
    pending: &mut Vec<(u32, Option<usize>)>,
    owner: usize,
    root: u32,
    spent: &mut usize,
) -> Result<(), Failure> {
    pending.push((root, None));
    while let Some((id, parent)) = pending.pop() {
        charge(spent, 1)?;
        let index = id as usize;
        let node = forest.nodes.get_mut(index).ok_or(Failure::InternalFailure)?;
        if node.owner != usize::MAX {
            return Err(Failure::InternalFailure);
        }
        *node = Node { owner, parent };
        charge(spent, children(&unit.type_syntax[index].kind).len())?;
        for child in children(&unit.type_syntax[index].kind) {
            if *child >= id {
                return Err(Failure::InternalFailure);
            }
            pending.push((*child, Some(index)));
        }
    }
    Ok(())
}
