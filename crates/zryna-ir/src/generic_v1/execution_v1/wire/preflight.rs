//! Raw structural checks are symmetric with decode; they establish no source/type authority.
use super::{
    Failure, invalid,
    limits::{Totals, bound},
    raw,
};
use crate::data_ownership_v1 as m3;
use crate::generic_v1::keys::{self, Domain};
mod function;
mod operation;
pub(super) fn check(program: &raw::Program) -> Result<(), Failure> {
    if program.language_contract != raw::CONTRACT {
        return Err(invalid());
    }
    bound(program.modules.len(), m3::MAX_MODULES)?;
    bound(program.declarations.len(), m3::MAX_FUNCTIONS_PER_PROGRAM)?;
    bound(program.type_keys.len(), m3::MAX_INSTANTIATED_TYPES)?;
    bound(program.functions.len(), m3::MAX_FUNCTIONS_PER_PROGRAM)?;
    let mut declaration_index = 0usize;
    for (index, module) in program.modules.iter().enumerate() {
        if usize::try_from(module.id).map_err(|_| invalid())? != index {
            return Err(invalid());
        }
        bound(module.functions as usize, m3::MAX_FUNCTIONS_PER_MODULE)?;
        for original in 0..module.functions {
            let declaration = program.declarations.get(declaration_index).ok_or_else(invalid)?;
            if declaration.module != module.id
                || declaration.function != original
                || declaration.parameters > 2
            {
                return Err(invalid());
            }
            declaration_index = declaration_index.checked_add(1).ok_or_else(invalid)?;
        }
    }
    if declaration_index != program.declarations.len() {
        return Err(invalid());
    }
    sorted_keys(&program.type_keys, false)?;
    let mut previous: Option<&[u8]> = None;
    let mut generic_functions = 0usize;
    let mut totals = Totals::default();
    for function in &program.functions {
        key(&function.key, true)?;
        if previous.is_some_and(|prior| prior >= function.key.as_slice()) {
            return Err(invalid());
        }
        previous = Some(&function.key);
        if function.key.first() == Some(&0x40) {
            super::limits::add(&mut generic_functions, 1, 4096)?;
        }
        function::check(function, &mut totals)?;
    }
    Ok(())
}
fn key(bytes: &[u8], function: bool) -> Result<(), Failure> {
    let domain = if function {
        match bytes.first() {
            Some(0x40) => Domain::FunctionInstance,
            Some(0x41) => Domain::SourceRoot,
            _ => return Err(invalid()),
        }
    } else {
        Domain::Type
    };
    keys::decode(bytes, domain)?;
    Ok(())
}
fn sorted_keys(keys: &[Vec<u8>], function: bool) -> Result<(), Failure> {
    for (index, bytes) in keys.iter().enumerate() {
        key(bytes, function)?;
        if index > 0 && keys[index - 1] >= *bytes {
            return Err(invalid());
        }
    }
    Ok(())
}
