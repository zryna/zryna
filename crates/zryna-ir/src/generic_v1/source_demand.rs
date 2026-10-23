//! Complete original stored roots, independent of raw claims or executable function demand.
use super::{
    Failure, raw, reject, reserve,
    source::Originals,
    source_types::{Closed, Resolver},
};
use zryna_syntax::v5::RawDataDeclarationKind;
mod owners;
mod shapes;

/// Only authenticated original replay constructs this private, exact-program-bound token.
pub(super) struct StoredRoots<'p> {
    program: &'p raw::Program,
    indices: Vec<usize>,
}
impl StoredRoots<'_> {
    pub(super) fn indices_for(&self, program: &raw::Program) -> Result<&[usize], Failure> {
        if !std::ptr::eq(self.program, program) {
            return Err(reject("original stored roots belong to a different program inventory"));
        }
        Ok(&self.indices)
    }
}

pub(super) fn derive<'p>(
    program: &'p raw::Program,
    originals: &Originals<'_>,
) -> Result<StoredRoots<'p>, Failure> {
    let mut marks = reserve(program.type_keys.len())?;
    marks.resize(program.type_keys.len(), false);
    for unit in originals.units {
        for (index, data) in unit.data_declarations.iter().enumerate() {
            if data.type_parameters.as_ref().is_some_and(|list| !list.parameters.is_empty()) {
                continue;
            }
            // Required nongeneric originals are roots even when no function mentions them.
            let mut key = [0; 9];
            key[0] = match data.kind {
                RawDataDeclarationKind::Struct { .. } => 0x10,
                RawDataDeclarationKind::Enum { .. } => 0x11,
            };
            key[1..5].copy_from_slice(&unit.id.to_le_bytes());
            key[5..9].copy_from_slice(
                &u32::try_from(index).map_err(|_| Failure::InternalFailure)?.to_le_bytes(),
            );
            mark(program, &key, &mut marks)?;
        }
        let forest = owners::forest(unit)?;
        let shapes = shapes::classify(originals, unit, &forest)?;
        for (index, node) in forest.nodes.iter().enumerate() {
            let Some(shape) = shapes[index] else { continue };
            if node.parent.is_some_and(|parent| shapes[parent].is_some()) {
                continue;
            }
            // The authenticated forest owns every node once. Maximal closed subtrees are
            // disjoint: no node is resolved separately beneath a resolved closed parent.
            // Exact byte/depth descriptors preflight every intermediate copied key. The
            // Resolver visits each selected subtree once; ancestor copies cost at most the
            // inherited depth64 per node. No per-node key cache or N-times arena scan exists.
            shape.check()?;
            let resolver = Resolver {
                originals,
                module: unit.id,
                parameters: forest.parameters[node.owner],
                arguments: Vec::new(),
            };
            let Closed::Stored(key) =
                resolver.resolve(u32::try_from(index).map_err(|_| Failure::InternalFailure)?)?
            else {
                return Err(Failure::InternalFailure);
            };
            if key.len() != shape.bytes {
                return Err(Failure::InternalFailure);
            }
            mark(program, &key, &mut marks)?;
        }
    }
    let mut indices = reserve(marks.iter().filter(|mark| **mark).count())?;
    indices.extend(marks.iter().enumerate().filter_map(|(index, marked)| marked.then_some(index)));
    Ok(StoredRoots { program, indices })
}

fn mark(program: &raw::Program, key: &[u8], marks: &mut [bool]) -> Result<(), Failure> {
    let index = program
        .type_keys
        .binary_search_by(|candidate| candidate.as_slice().cmp(key))
        .map_err(|_| reject("original stored source root is absent from the complete universe"))?;
    *marks.get_mut(index).ok_or(Failure::InternalFailure)? = true;
    Ok(())
}

#[cfg(test)]
mod tests;
