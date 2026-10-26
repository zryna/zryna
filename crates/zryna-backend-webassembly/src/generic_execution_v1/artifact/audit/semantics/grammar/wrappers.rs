//! Exact scalar-wrapper control census from the retained export and parameter roles.
use super::{Counts, Diagnostic, Model, raw};
pub(super) fn role(c: &mut Counts, index: usize, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    let f = m.functions[m.program.export_functions()[index - m.wrapper_base as usize]];
    c.ifs = 3;
    m.take(f.parameters().len())?;
    for param in f.parameters() {
        if let zryna_ir::generic_v1::execution_v1::VerifiedParameter::Value(v) = param
            && let raw::ValueType::Stored(ty) = v.ty()
        {
            c.ifs += u32::from(m.types[ty as usize].size() == 1);
        }
    }
    c.traps = c.ifs;
    Ok(())
}
