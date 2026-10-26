//! Every original enum is checked in its own opaque declaration frame, even when unused.
use super::super::Budget;
use super::{Admission, SourceTypeFacts, domain::add, nominal_key_for};
use crate::generic_v1::{Failure, budget, reject, source::Originals};
use zryna_syntax::v5::RawDataDeclarationKind;

pub(in super::super) fn check_declarations_for(
    originals: &Originals<'_>,
    admission: Admission,
) -> Result<usize, Failure> {
    let mut credit = Budget::default();
    let mut members = 0;
    for unit in originals.units {
        for (index, original) in unit.data_declarations.iter().enumerate() {
            let (structure, count) = match &original.kind {
                RawDataDeclarationKind::Enum { variants, .. } => (false, variants.len()),
                RawDataDeclarationKind::Struct { fields, .. }
                    if admission == Admission::StructV4 =>
                {
                    (true, fields.len())
                }
                RawDataDeclarationKind::Struct { .. } => {
                    return Err(reject("owned nominal source lane requires an original enum"));
                }
            };
            members = add(members, count)?;
            if members > zryna_layout::MAX_MEMBERS {
                return Err(budget("nominal original member count exceeds 65536"));
            }
            let arity = original.type_parameters.as_ref().map_or(0, |list| list.parameters.len());
            if arity > 2 {
                return Err(reject("nominal enum has wrong generic arity"));
            }
            let arguments: [&[u8]; 2] = [&[0x30], &[0x31]];
            let key = nominal_key_for(
                unit.id,
                u32::try_from(index).map_err(|_| Failure::InternalFailure)?,
                &arguments[..arity],
                &mut credit,
                structure,
            )?;
            // Opaque tag identities never cross declaration frames through a shared cache.
            SourceTypeFacts::new(admission).nominal(originals, &key, &mut credit)?;
        }
    }
    Ok(credit.nominal_used())
}
