//! Maximal closed original subtrees, including unused-template closed descendants.
use super::{
    Closed, Failure, Originals, RawDataDeclarationKind, Resolver, add, charge, copy_key, exhausted,
    key, push, reject, reserve, resolution, resolve, small,
};
mod owners;
mod shapes;
pub(in crate::generic_v1::execution_v1::source) fn stored_roots(
    originals: &Originals<'_>,
    spent: &mut usize,
) -> Result<Vec<Vec<u8>>, Failure> {
    let mut roots = Vec::new();
    for primitive in 0..3 {
        let key = copy_key(&[primitive], spent)?;
        push(&mut roots, key, spent)?;
    }
    for unit in originals.units {
        charge(spent, unit.data_declarations.len())?;
        for (index, data) in unit.data_declarations.iter().enumerate() {
            if data.type_parameters.as_ref().is_some_and(|list| !list.parameters.is_empty()) {
                continue;
            }
            let tag = if matches!(data.kind, RawDataDeclarationKind::Struct { .. }) {
                0x10
            } else {
                0x11
            };
            let Closed::Stored(key) = key::encode(tag, &[unit.id, small(index)?], &[], spent)?
            else {
                return Err(Failure::InternalFailure);
            };
            push(&mut roots, key, spent)?;
        }
        let forest = owners::forest(unit, spent)?;
        let shapes = shapes::classify(originals, unit, &forest, spent)?;
        charge(spent, forest.nodes.len())?;
        for (index, node) in forest.nodes.iter().enumerate() {
            let Some(shape) = shapes[index] else {
                continue;
            };
            if node.parent.is_some_and(|parent| shapes[parent].is_some()) {
                continue;
            }
            shape.check()?;
            let resolver = Resolver {
                originals,
                module: unit.id,
                parameters: forest.parameters[node.owner],
                arguments: Vec::new(),
            };
            let Closed::Stored(key) = resolve(&resolver, small(index)?, spent)? else {
                return Err(Failure::InternalFailure);
            };
            if key.len() != shape.bytes {
                return Err(Failure::InternalFailure);
            }
            push(&mut roots, key, spent)?;
        }
    }
    Ok(roots)
}
