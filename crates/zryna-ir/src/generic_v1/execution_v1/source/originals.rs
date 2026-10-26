//! Authenticated original inventory and independently checked symbolic source bodies.
use super::{
    Closed, Expr, Failure, Frame, Kind, Originals, RawDataDeclarationKind, Resolver, SourceMap,
    Target, UntrustedSpan, VerifiedLayouts, VerifiedProjectSyntaxV5, WorldBuilder, charge,
    copy_key, diagnostic, exhausted, raw, reject, reserve, v5,
};
use zryna_source::FileId;
mod checks;
mod description;
mod facts;
mod key;
mod resolution;
mod sort;
mod stored;
pub(super) use description::{Description, describe};
pub(super) use facts::symbolic_fact;
pub(super) use resolution::{resolution_credit, resolve};
pub(super) use stored::stored_roots;

pub(super) struct Inventory {
    pub modules: Vec<raw::Module>,
    pub declarations: Vec<raw::Declaration>,
}

pub(super) fn inventory(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    spent: &mut usize,
) -> Result<Inventory, Failure> {
    if !syntax.is_bound_to(sources) {
        return Err(reject("full-profile syntax belongs to a foreign SourceMap"));
    }
    charge(spent, syntax.files().len())?;
    let mut modules = reserve(syntax.files().len())?;
    let mut declarations = Vec::new();
    for unit in syntax.files() {
        modules.push(raw::Module { id: unit.id, functions: small(unit.functions.len())? });
        charge(spent, unit.functions.len())?;
        declarations.try_reserve(unit.functions.len()).map_err(|_| Failure::AllocationFailure)?;
        for (index, function) in unit.functions.iter().enumerate() {
            declarations.push(raw::Declaration {
                module: unit.id,
                function: small(index)?,
                parameters: small(
                    function.type_parameters.as_ref().map_or(0, |p| p.parameters.len()),
                )?,
                span: function.span,
            });
        }
    }
    Ok(Inventory { modules, declarations })
}

pub(super) fn authenticate<'a>(
    inventory: &Inventory,
    syntax: &'a VerifiedProjectSyntaxV5,
    spent: &mut usize,
) -> Result<Originals<'a>, Failure> {
    // Prepay the new intrinsic comparisons; legacy binding and call-cycle checks stay exact.
    charge(spent, syntax.files().len())?;
    for unit in syntax.files() {
        charge(spent, unit.functions.len())?;
        for function in &unit.functions {
            charge(spent, add(function.name.text.len().min(6), 1)?)?;
            if function.name.text.eq_ignore_ascii_case("concat") {
                return Err(reject("original function collides with reserved concat intrinsic"));
            }
            charge(spent, function.body.expressions.len())?;
            for expression in &function.body.expressions {
                if let Expr::Call { callee, .. } = &expression.kind {
                    charge(spent, add(callee.text.len().min(6), 1)?)?;
                }
            }
        }
    }
    Originals::check_execution_parts(&inventory.modules, &inventory.declarations, syntax)
}

pub(super) fn check(
    originals: &Originals<'_>,
    sources: &SourceMap,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut used = *spent;
    checks::all(originals, sources, &mut used)?;
    *spent = used;
    Ok(())
}

pub(super) fn program(
    inventory: Inventory,
    sources: &SourceMap,
    entry: FileId,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    generic_keys: &[&[u8]],
    spent: &mut usize,
) -> Result<raw::Program, Failure> {
    use zryna_layout::StorageTarget;
    if sources.verify_file_id(entry.index()).ok() != Some(entry)
        || linear.source_map_identity() != sources.identity()
        || linux.source_map_identity() != sources.identity()
        || linear.target() != StorageTarget::Linear32V1
        || linux.target() != StorageTarget::LinuxX8664V1
        || linear.universe_identity() != linux.universe_identity()
        || linear.original_modules() != linux.original_modules()
        || linear.original_declarations() != linux.original_declarations()
        || linear.types().len() != linux.types().len()
    {
        return Err(reject("source producer dual-layout/entry authorities differ"));
    }
    charge(spent, linear.types().len())?;
    let mut type_keys = reserve(linear.types().len())?;
    for (a, b) in linear.types().zip(linux.types()) {
        charge(spent, add(a.key().len().min(b.key().len()), 1)?)?;
        if a.key() != b.key() {
            return Err(reject("dual target type identities differ"));
        }
        type_keys.push(copy_key(a.key(), spent)?);
    }
    if generic_keys.len() > 4096 {
        return Err(exhausted("source function instance ceiling"));
    }
    let capacity = add(generic_keys.len(), inventory.declarations.len())?;
    charge(spent, capacity)?;
    let mut keys = reserve(capacity)?;
    for key in generic_keys {
        charge(spent, add(key.len().checked_mul(4).ok_or(Failure::InternalFailure)?, 67)?)?;
        crate::generic_v1::keys::decode(key, crate::generic_v1::keys::Domain::FunctionInstance)?;
        keys.push(copy_key(key, spent)?);
    }
    for declaration in &inventory.declarations {
        if declaration.parameters == 0 {
            charge(spent, 9)?;
            let mut key = reserve(9)?;
            key.push(0x41);
            key.extend(declaration.module.to_le_bytes());
            key.extend(declaration.function.to_le_bytes());
            keys.push(key);
        }
    }
    sort::keys(&mut keys, spent)?;
    charge(spent, keys.len())?;
    let mut functions = reserve(keys.len())?;
    for key in keys {
        let original_span = header(&key, &inventory.declarations, spent)?;
        functions.push(raw::Function {
            key,
            // Untrusted replay skeletons retain authentic source headers, never fake IDs.
            original_span,
            executable_export: None,
            parameters: Vec::new(),
            result_type: raw::ValueType::Unit,
            places: Vec::new(),
            loans: Vec::new(),
            blocks: Vec::new(),
            cleanup_plans: Vec::new(),
        });
    }
    charge(spent, raw::CONTRACT.len())?;
    Ok(raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: inventory.modules,
        declarations: inventory.declarations,
        type_keys,
        universe: *linear.universe_identity(),
        linear32: *linear.fingerprint(),
        linux_x86_64: *linux.fingerprint(),
        entry_module: entry.index(),
        functions,
    })
}

fn header(
    key: &[u8],
    declarations: &[raw::Declaration],
    spent: &mut usize,
) -> Result<UntrustedSpan, Failure> {
    use crate::generic_v1::keys::{Domain, decode};
    charge(spent, add(key.len().checked_mul(4).ok_or(Failure::InternalFailure)?, 67)?)?;
    let decoded = decode(
        key,
        if key.first() == Some(&0x40) { Domain::FunctionInstance } else { Domain::SourceRoot },
    )?;
    let target = decoded.declaration().ok_or(Failure::InternalFailure)?;
    let (mut low, mut high) = (0, declarations.len());
    while low < high {
        let middle = low + (high - low) / 2;
        charge(spent, 3)?;
        let declaration = &declarations[middle];
        match (declaration.module, declaration.function).cmp(&target) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => {
                if usize::try_from(declaration.parameters).map_err(|_| Failure::InternalFailure)?
                    != decoded.arguments().len()
                {
                    return Err(reject(
                        "source function key differs from its exact original arity",
                    ));
                }
                return Ok(declaration.span);
            }
        }
    }
    Err(reject("source function key has no authenticated original header"))
}

fn add(left: usize, right: usize) -> Result<usize, Failure> {
    left.checked_add(right).ok_or_else(|| exhausted("original source work overflow"))
}
fn small(value: usize) -> Result<u32, Failure> {
    u32::try_from(value).map_err(|_| Failure::InternalFailure)
}
fn push<T>(values: &mut Vec<T>, value: T, spent: &mut usize) -> Result<(), Failure> {
    charge(spent, 1)?;
    values.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
    values.push(value);
    Ok(())
}
fn clone_type(ty: &Closed, spent: &mut usize) -> Result<Closed, Failure> {
    Ok(match ty {
        Closed::Unit => Closed::Unit,
        Closed::Stored(key) => Closed::Stored(copy_key(key, spent)?),
        Closed::Borrow(key, exclusive) => Closed::Borrow(copy_key(key, spent)?, *exclusive),
    })
}
fn equal(left: &Closed, right: &Closed, spent: &mut usize) -> Result<bool, Failure> {
    let bytes = |ty: &Closed| match ty {
        Closed::Stored(key) | Closed::Borrow(key, _) => key.len(),
        Closed::Unit => 0,
    };
    charge(spent, add(bytes(left).min(bytes(right)), 1)?)?;
    Ok(left == right)
}

#[cfg(test)]
mod tests;
