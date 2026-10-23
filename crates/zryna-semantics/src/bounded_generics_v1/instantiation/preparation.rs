//! Private original-validation prefix retaining actual root inventories and all checked counters.
use super::{
    BodyTypeContext, InstanceContext, InstantiationFailure, definition_expansion, diagnostics,
    direct_original_cycles, discovery, model::Builder, source_calls,
};

/// Temporary state only: no closed instance, ownership, layout or executable authority.
pub(super) struct PreparedOriginals<'b, 'c, 's> {
    builder: Builder<'b, 'c, 's>,
}

pub(super) fn prepare_originals<'b, 'c, 's>(
    bodies: &'b BodyTypeContext<'c, 's>,
) -> Result<PreparedOriginals<'b, 'c, 's>, InstantiationFailure> {
    let mut builder = Builder::new(bodies)?;
    let mut original_errors = diagnostics::Errors::default();
    source_calls::collect(&builder, &mut original_errors)?;
    if original_errors.is_empty() {
        // Only actual authenticated supplied roots are interned in this private temporary.
        // Existing source-call failures keep their precedence over supplied arguments.
        discovery::roots(&mut builder)?;
        builder.check_errors()?;
    }
    definition_expansion::collect(&builder, &mut original_errors)?;
    direct_original_cycles::collect(bodies, &mut original_errors)?;
    if !original_errors.is_empty() {
        return Err(InstantiationFailure::Diagnostics(original_errors.finish()?));
    }
    Ok(PreparedOriginals { builder })
}

impl<'b, 'c, 's> PreparedOriginals<'b, 'c, 's> {
    /// The private-v4 caller checks all original ownership before invoking completion.
    pub(super) fn finish(mut self) -> Result<InstanceContext<'b, 'c, 's>, InstantiationFailure> {
        discovery::pending(&mut self.builder)?;
        discovery::value_arguments(&mut self.builder)?;
        self.builder.check_errors()?;
        self.builder.finish()
    }
}
