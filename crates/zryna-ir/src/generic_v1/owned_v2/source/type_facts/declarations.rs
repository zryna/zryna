//! Every original enum is checked in its own opaque declaration frame, even when unused.
use super::super::Budget;
use super::{SourceTypeFacts, domain::add, nominal_key};
use crate::generic_v1::{Failure, budget, reject, source::Originals};
use zryna_syntax::v5::RawDataDeclarationKind;

pub(in super::super) fn check_declarations(originals: &Originals<'_>) -> Result<usize, Failure> {
    let mut credit = Budget::default();
    let mut members = 0;
    for unit in originals.units {
        for (index, original) in unit.data_declarations.iter().enumerate() {
            let RawDataDeclarationKind::Enum { variants, .. } = &original.kind else {
                return Err(reject("owned nominal source lane requires an original enum"));
            };
            members = add(members, variants.len())?;
            if members > zryna_layout::MAX_MEMBERS {
                return Err(budget("nominal original member count exceeds 65536"));
            }
            let arity = original.type_parameters.as_ref().map_or(0, |list| list.parameters.len());
            if arity > 2 {
                return Err(reject("nominal enum has wrong generic arity"));
            }
            let arguments: [&[u8]; 2] = [&[0x30], &[0x31]];
            let key = nominal_key(
                unit.id,
                u32::try_from(index).map_err(|_| Failure::InternalFailure)?,
                &arguments[..arity],
                &mut credit,
            )?;
            // Opaque tag identities never cross declaration frames through a shared cache.
            SourceTypeFacts::default().nominal(originals, &key, &mut credit)?;
        }
    }
    Ok(credit.nominal_used())
}
