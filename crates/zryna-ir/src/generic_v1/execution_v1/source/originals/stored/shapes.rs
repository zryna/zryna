//! Allocation-free closedness and exact canonical byte/depth preflight of original subtrees.
use super::owners::{Forest, children};
use super::{Failure, Originals, Resolver, add, charge, exhausted, reject, reserve, resolution};
use crate::generic_v1::source::Target;
use zryna_syntax::v5::{RawSourceUnit, RawTypeSyntaxKind};

#[derive(Clone, Copy)]
pub(super) struct Shape {
    pub bytes: usize,
    depth: usize,
}
impl Shape {
    pub(super) fn check(self) -> Result<Self, Failure> {
        if self.bytes > 4096 || self.depth > 64 {
            return Err(exhausted("original stored root exceeds inherited byte/depth ceiling"));
        }
        Ok(self)
    }
}

pub(super) fn classify(
    originals: &Originals<'_>,
    unit: &RawSourceUnit,
    forest: &Forest<'_>,
    spent: &mut usize,
) -> Result<Vec<Option<Shape>>, Failure> {
    charge(spent, unit.type_syntax.len())?;
    let mut shapes = reserve(unit.type_syntax.len())?;
    for (index, node) in unit.type_syntax.iter().enumerate() {
        let parameters = forest.parameters[forest.nodes[index].owner];
        let parameter_credit = parameters.map_or(0, |p| p.parameters.len());
        charge(spent, parameter_credit)?;
        if let Some(parameters) = parameters {
            for parameter in &parameters.parameters {
                charge(spent, add(parameter.name.text.len(), 1)?)?;
            }
        }
        let opaque = |name: &str| {
            parameters.is_some_and(|list| {
                list.parameters.iter().any(|parameter| parameter.name.text == name)
            })
        };
        charge(spent, add(1, children(&node.kind).len())?)?;
        let header = match &node.kind {
            RawTypeSyntaxKind::Missing
            | RawTypeSyntaxKind::Borrow { .. }
            | RawTypeSyntaxKind::BorrowMut { .. } => None,
            RawTypeSyntaxKind::Named { name } if opaque(&name.text) || name.text == "unit" => None,
            RawTypeSyntaxKind::Named { name } => match name.text.as_str() {
                "bool" | "i32" => Some(1),
                "Option" | "Result" => {
                    return Err(reject("original family root requires explicit arguments"));
                }
                _ => nominal(originals, unit.id, &name.text, 0, spent)?,
            },
            RawTypeSyntaxKind::Application { name, type_arguments } => {
                if opaque(&name.text) {
                    return Err(reject(
                        "opaque parameter cannot head an original type application",
                    ));
                }
                match name.text.as_str() {
                    "Option" if type_arguments.arguments.len() == 1 => Some(5),
                    "Result" if type_arguments.arguments.len() == 2 => Some(5),
                    "Option" | "Result" => {
                        return Err(reject("original family root has inexact arity"));
                    }
                    _ => nominal(
                        originals,
                        unit.id,
                        &name.text,
                        type_arguments.arguments.len(),
                        spent,
                    )?,
                }
            }
            RawTypeSyntaxKind::FixedArray { .. } => Some(5),
            RawTypeSyntaxKind::String { .. }
            | RawTypeSyntaxKind::Vec { .. }
            | RawTypeSyntaxKind::Shared { .. }
            | RawTypeSyntaxKind::Weak { .. } => Some(1),
        };
        let shape = header
            .map(|bytes| combine(bytes, children(&node.kind), &shapes))
            .transpose()?
            .flatten();
        shapes.push(shape);
    }
    Ok(shapes)
}

fn nominal(
    originals: &Originals<'_>,
    module: u32,
    name: &str,
    supplied: usize,
    spent: &mut usize,
) -> Result<Option<usize>, Failure> {
    let resolver = Resolver { originals, module, parameters: None, arguments: Vec::new() };
    resolution::name_credit(&resolver, name, spent)?;
    let Target::Data(module, index) = originals.resolve(module, name)? else {
        // Semantic Function type heads have no stored identity or descendants.
        return Ok(None);
    };
    let data = &originals.units[module as usize].data_declarations[index as usize];
    let arity = data.type_parameters.as_ref().map_or(0, |list| list.parameters.len());
    if supplied != arity {
        return Err(reject("original nominal root requires exact explicit arity"));
    }
    Ok(Some(if supplied == 0 { 9 } else { 13 }))
}

fn combine(
    bytes: usize,
    child_ids: &[u32],
    shapes: &[Option<Shape>],
) -> Result<Option<Shape>, Failure> {
    let mut result = Shape { bytes, depth: 0 };
    let mut closed = true;
    for id in child_ids {
        let Some(child) = shapes.get(*id as usize).ok_or(Failure::InternalFailure)? else {
            closed = false;
            continue;
        };
        result.bytes = result
            .bytes
            .checked_add(4)
            .and_then(|n| n.checked_add(child.bytes))
            .ok_or(Failure::InternalFailure)?;
        result.depth =
            result.depth.max(child.depth.checked_add(1).ok_or(Failure::InternalFailure)?);
    }
    if closed { result.check().map(Some) } else { Ok(None) }
}
