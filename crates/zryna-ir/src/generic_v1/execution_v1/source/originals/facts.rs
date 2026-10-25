//! Finite symbolic source graph built with explicit metered work/value stacks.
use super::{
    Closed, Failure, Frame, Kind, Originals, WorldBuilder, add, charge, copy_key, description,
    equal, key, push, reject, reserve, small,
};
use crate::generic_v1::execution_v1::type_world::Fact;
struct Graph<'a, 's> {
    originals: &'a Originals<'s>,
    owner: Frame,
    builder: WorldBuilder,
    known: Vec<(Vec<u8>, u32)>,
}
enum Task {
    Key(Vec<u8>),
    Declare { key: Vec<u8>, kind: Kind, arguments: usize },
    Finish { id: u32, members: Vec<Option<u32>> },
}
pub(in crate::generic_v1::execution_v1::source) fn symbolic_fact(
    originals: &Originals<'_>,
    owner: Frame,
    ty: &Closed,
    spent: &mut usize,
) -> Result<Fact, Failure> {
    let Closed::Stored(key) = ty else {
        return Ok(Fact { copy: false, clone: false });
    };
    let mut used = *spent;
    let mut graph = Graph { originals, owner, builder: WorldBuilder::new(), known: Vec::new() };
    let root = graph.discover(key, &mut used)?;
    let fact = graph.builder.seal(&mut used)?.fact(root).ok_or(Failure::InternalFailure)?;
    *spent = used;
    Ok(fact)
}
impl Graph<'_, '_> {
    fn discover(&mut self, key: &[u8], spent: &mut usize) -> Result<u32, Failure> {
        let mut pending = Vec::new();
        let mut values = Vec::new();
        let key = copy_key(key, spent)?;
        push(&mut pending, Task::Key(key), spent)?;
        while let Some(task) = pending.pop() {
            charge(spent, 1)?;
            match task {
                Task::Key(key) => self.key(key, &mut pending, &mut values, spent)?,
                Task::Declare { key, kind, arguments } => {
                    let first =
                        values.len().checked_sub(arguments).ok_or(Failure::InternalFailure)?;
                    charge(spent, arguments)?;
                    let mut ids = reserve(arguments)?;
                    ids.extend(values.drain(first..));
                    // Argument-member discovery may already have published this header.
                    if let Ok(index) = self.find(&key, spent)? {
                        push(&mut values, self.known[index].1, spent)?;
                        continue;
                    }
                    let id = self.builder.declare(kind, &ids, spent)?;
                    self.remember(&key, id, spent)?;
                    if matches!(kind, Kind::Struct { .. } | Kind::Enum { .. }) {
                        self.members(key, id, &mut pending, spent)?;
                    } else {
                        push(&mut values, id, spent)?;
                    }
                }
                Task::Finish { id, mut members } => {
                    charge(spent, members.len())?;
                    let count = members.iter().filter(|member| member.is_some()).count();
                    let first = values.len().checked_sub(count).ok_or(Failure::InternalFailure)?;
                    let mut children = values.drain(first..);
                    for member in members.iter_mut().filter(|member| member.is_some()) {
                        *member = Some(children.next().ok_or(Failure::InternalFailure)?);
                    }
                    if children.next().is_some() {
                        return Err(Failure::InternalFailure);
                    }
                    drop(children);
                    self.builder.set_members(id, &members, spent)?;
                    push(&mut values, id, spent)?;
                }
            }
        }
        if values.len() != 1 {
            return Err(Failure::InternalFailure);
        }
        values.pop().ok_or(Failure::InternalFailure)
    }
    fn key(
        &mut self,
        key: Vec<u8>,
        pending: &mut Vec<Task>,
        values: &mut Vec<u32>,
        spent: &mut usize,
    ) -> Result<(), Failure> {
        key::validate(&key, spent)?;
        if let Ok(index) = self.find(&key, spent)? {
            return push(values, self.known[index].1, spent);
        }
        let shape = key::shape(&key, spent)?;
        let kind = self.kind(&shape, spent)?;
        charge(spent, shape.children.len())?;
        let mut children = reserve(shape.children.len())?;
        for child in &shape.children {
            children.push(copy_key(child, spent)?);
        }
        let arguments = children.len();
        drop(shape);
        push(pending, Task::Declare { key, kind, arguments }, spent)?;
        for child in children.into_iter().rev() {
            push(pending, Task::Key(child), spent)?;
        }
        Ok(())
    }
    fn members(
        &mut self,
        key: Vec<u8>,
        id: u32,
        pending: &mut Vec<Task>,
        spent: &mut usize,
    ) -> Result<(), Failure> {
        let shape = key::shape(&key, spent)?;
        charge(spent, shape.children.len())?;
        let mut args = reserve(shape.children.len())?;
        for child in &shape.children {
            args.push(copy_key(child, spent)?);
        }
        let module = shape.module;
        let index = shape.index;
        let structure = matches!(shape.tag, 0x10 | 0x12);
        drop(shape);
        let description = description::identity(self.originals, module, index, &args, spent)?;
        if description.structure != structure
            || !equal(&description.ty, &Closed::Stored(key), spent)?
        {
            return Err(reject("symbolic nominal key differs from authenticated declaration"));
        }
        charge(spent, description.members.len())?;
        let mut members = reserve(description.members.len())?;
        for (_, member) in &description.members {
            members.push(match member {
                Some(Closed::Stored(_)) => Some(u32::MAX),
                None => None,
                _ => return Err(reject("nonstored original member")),
            });
        }
        push(pending, Task::Finish { id, members }, spent)?;
        for (_, member) in description.members.into_iter().rev() {
            if let Some(Closed::Stored(key)) = member {
                push(pending, Task::Key(key), spent)?;
            }
        }
        Ok(())
    }
    fn find(&self, key: &[u8], spent: &mut usize) -> Result<Result<usize, usize>, Failure> {
        let (mut low, mut high) = (0, self.known.len());
        while low < high {
            let middle = low + (high - low) / 2;
            charge(spent, add(key.len().min(self.known[middle].0.len()), 1)?)?;
            match self.known[middle].0.as_slice().cmp(key) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return Ok(Ok(middle)),
            }
        }
        Ok(Err(low))
    }
    fn remember(&mut self, key: &[u8], id: u32, spent: &mut usize) -> Result<(), Failure> {
        let Err(index) = self.find(key, spent)? else {
            return Err(Failure::InternalFailure);
        };
        charge(spent, add(self.known.len() - index, 1)?)?;
        let key = copy_key(key, spent)?;
        self.known.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.known.insert(index, (key, id));
        Ok(())
    }
    fn kind(&self, shape: &key::Shape<'_>, spent: &mut usize) -> Result<Kind, Failure> {
        charge(spent, 1)?;
        let arity = small(shape.children.len())?;
        Ok(match shape.tag {
            0 => Kind::Bool,
            1 => Kind::I32,
            2 => Kind::String,
            0x30..=0x31 => {
                let unit = self
                    .originals
                    .units
                    .get(self.owner.module as usize)
                    .ok_or_else(|| reject("opaque owner module is absent"))?;
                let parameters = match self.owner.kind {
                    0 => unit
                        .functions
                        .get(self.owner.index as usize)
                        .and_then(|f| f.type_parameters.as_ref()),
                    1 => unit
                        .data_declarations
                        .get(self.owner.index as usize)
                        .and_then(|d| d.type_parameters.as_ref()),
                    _ => return Err(reject("opaque owner kind is invalid")),
                };
                let slot = u32::from(shape.tag - 0x30);
                if usize::try_from(slot).map_err(|_| Failure::InternalFailure)?
                    >= parameters.map_or(0, |p| p.parameters.len())
                {
                    return Err(reject("opaque slot is not declared in its owner"));
                }
                Kind::Opaque { owner: self.owner, slot }
            }
            0x10 | 0x12 => Kind::Struct { module: shape.module, declaration: shape.index, arity },
            0x11 | 0x13 => Kind::Enum { module: shape.module, declaration: shape.index, arity },
            0x14 => Kind::Option,
            0x15 => Kind::Result,
            0x20 => Kind::FixedArray { length: shape.length },
            0x21 => Kind::Vec,
            0x22 => Kind::Shared,
            0x23 => Kind::Weak,
            _ => return Err(reject("unknown symbolic capability shape")),
        })
    }
}

#[cfg(test)]
mod deep;
