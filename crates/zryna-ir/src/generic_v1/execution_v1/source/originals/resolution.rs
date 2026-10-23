//! Prepay the actual finite syntax walk plus bounded existing resolver preparation.
use super::{Closed, Failure, Resolver, add, charge, reject, v5};
use v5::RawTypeSyntaxKind as T;
enum Task {
    Node(u32),
    Finish(usize, usize),
}
fn push<T>(values: &mut Vec<T>, value: T, spent: &mut usize) -> Result<(), Failure> {
    charge(spent, 1)?;
    values.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
    values.push(value);
    Ok(())
}
pub(in crate::generic_v1::execution_v1::source) fn resolution_credit(
    resolver: &Resolver<'_, '_>,
    at: u32,
    spent: &mut usize,
) -> Result<(), Failure> {
    let unit = &resolver.originals.units[resolver.module as usize];
    let names = names_credit(unit, resolver.parameters, spent)?;
    // Existing resolver reserves exactly these two bounded task/value scratch arrays.
    charge(spent, 257 + 129)?;
    let mut pending = Vec::new();
    let mut sizes = Vec::new();
    push(&mut pending, Task::Node(at), spent)?;
    while let Some(task) = pending.pop() {
        charge(spent, 1)?;
        match task {
            Task::Node(id) => {
                let node = unit
                    .type_syntax
                    .get(id as usize)
                    .ok_or_else(|| reject("unknown source type occurrence"))?;
                let (base, children) = match &node.kind {
                    T::Named { name } => {
                        charge(spent, names.checked_mul(2).ok_or(Failure::InternalFailure)?)?;
                        let mut size = if name.text == "bool" || name.text == "i32" {
                            1
                        } else if name.text == "unit" {
                            0
                        } else {
                            9
                        };
                        if let Some(p) = resolver.parameters {
                            for (i, p) in p.parameters.iter().enumerate() {
                                charge(spent, p.name.text.len().min(name.text.len()) + 1)?;
                                if p.name.text == name.text {
                                    size = resolver
                                        .arguments
                                        .get(i)
                                        .ok_or_else(|| reject("missing original argument"))?
                                        .len();
                                    break;
                                }
                            }
                        }
                        charge(spent, size)?;
                        push(&mut sizes, size, spent)?;
                        continue;
                    }
                    T::String { .. } => {
                        charge(spent, 1)?;
                        push(&mut sizes, 1, spent)?;
                        continue;
                    }
                    T::Application { name, type_arguments } => {
                        charge(
                            spent,
                            add(
                                names.checked_mul(2).ok_or(Failure::InternalFailure)?,
                                type_arguments.arguments.len(),
                            )?,
                        )?;
                        let base =
                            if name.text == "Option" || name.text == "Result" { 5 } else { 13 };
                        (base, type_arguments.arguments.as_slice())
                    }
                    T::FixedArray { element, .. } => (5, std::slice::from_ref(element)),
                    T::Vec { argument, .. }
                    | T::Shared { argument, .. }
                    | T::Weak { argument, .. } => (1, std::slice::from_ref(argument)),
                    T::Borrow { argument, .. } | T::BorrowMut { argument, .. } => {
                        (0, std::slice::from_ref(argument))
                    }
                    T::Missing => return Err(reject("missing original type")),
                };
                charge(spent, children.len())?;
                push(&mut pending, Task::Finish(base, children.len()), spent)?;
                for child in children.iter().rev() {
                    push(&mut pending, Task::Node(*child), spent)?;
                }
            }
            Task::Finish(base, count) => {
                charge(spent, count)?;
                let first = sizes.len().checked_sub(count).ok_or(Failure::InternalFailure)?;
                let mut bytes = base;
                for n in &sizes[first..] {
                    bytes = bytes
                        .checked_add(*n)
                        .and_then(|n| n.checked_add(if base == 0 { 0 } else { 4 }))
                        .ok_or(Failure::InternalFailure)?;
                }
                sizes.truncate(first);
                charge(spent, add(add(bytes, count)?, 3)?)?;
                push(&mut sizes, bytes, spent)?;
            }
        }
    }
    if sizes.len() != 1 {
        return Err(Failure::InternalFailure);
    }
    Ok(())
}

fn names_credit(
    unit: &v5::RawSourceUnit,
    parameters: Option<&v5::RawTypeParameterList>,
    spent: &mut usize,
) -> Result<usize, Failure> {
    let mut count = add(unit.data_declarations.len(), unit.functions.len())?;
    for import in &unit.imports {
        count = add(count, import.bindings.len())?;
    }
    charge(spent, count)?;
    let mut names = count;
    for d in &unit.data_declarations {
        names = names
            .checked_add(crate::generic_v1::source::data_name(d).len())
            .ok_or(Failure::InternalFailure)?;
    }
    for f in &unit.functions {
        names = names.checked_add(f.name.text.len()).ok_or(Failure::InternalFailure)?;
    }
    for i in &unit.imports {
        for b in &i.bindings {
            names = names.checked_add(b.local.text.len()).ok_or(Failure::InternalFailure)?;
        }
    }
    if let Some(p) = parameters {
        charge(spent, p.parameters.len())?;
        for p in &p.parameters {
            names =
                names.checked_add(add(p.name.text.len(), 1)?).ok_or(Failure::InternalFailure)?;
        }
    }
    Ok(names)
}

pub(in crate::generic_v1::execution_v1::source) fn resolve(
    resolver: &Resolver<'_, '_>,
    at: u32,
    spent: &mut usize,
) -> Result<Closed, Failure> {
    resolution_credit(resolver, at, spent)?;
    resolver.resolve_symbolic(at)
}

pub(super) fn name_credit(
    resolver: &Resolver<'_, '_>,
    _name: &str,
    spent: &mut usize,
) -> Result<(), Failure> {
    let unit = resolver
        .originals
        .units
        .get(resolver.module as usize)
        .ok_or_else(|| reject("unknown original name owner"))?;
    charge(
        spent,
        add(add(unit.data_declarations.len(), unit.functions.len())?, unit.imports.len())?,
    )?;
    let mut total = 0;
    for data in &unit.data_declarations {
        total = add(total, add(crate::generic_v1::source::data_name(data).len(), 1)?)?;
    }
    for function in &unit.functions {
        total = add(total, add(function.name.text.len(), 1)?)?;
    }
    for import in &unit.imports {
        charge(spent, import.bindings.len())?;
        for binding in &import.bindings {
            total = add(total, add(binding.local.text.len(), 1)?)?;
        }
    }
    // Existing bounded binary search may repeat its final candidate; twice the
    // complete candidate-byte inventory bounds those folded comparisons/spelling.
    charge(spent, total.checked_mul(2).ok_or(Failure::InternalFailure)?)
}
