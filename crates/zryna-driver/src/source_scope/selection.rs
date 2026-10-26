//! Source-visible templates never acquire host-callable scalar specializations.
use crate::CommandFailure;
use zryna_semantics::bounded_generics_v1::{DeclarationContext, DeclarationKind, DeclarationView};

pub(super) fn check(
    declarations: &DeclarationContext<'_>,
    selected: Option<&str>,
) -> Result<(), CommandFailure> {
    let Some(selected) = selected else {
        return Ok(());
    };
    // Even an explicit specialization spelling requests the same forbidden host boundary.
    // This does not parse, instantiate or admit a specialization as an executable export.
    let name = selected.split_once('<').map_or(selected, |(name, _)| name);
    for module in declarations.modules() {
        if module.identity().source_file() != declarations.entry() {
            continue;
        }
        for function in module.functions() {
            if function.name() == name && function.is_exported() {
                reject_template(function)?;
            }
        }
        for binding in module.imports() {
            if binding.local_name() == name {
                reject_template(binding.target())?;
            }
        }
    }
    Ok(())
}

fn reject_template(declaration: DeclarationView<'_>) -> Result<(), CommandFailure> {
    if declaration.identity().kind() == DeclarationKind::Function
        && declaration.type_parameters().next().is_some()
    {
        return Err(super::failure::source(vec![zryna_diagnostics::Diagnostic::error_at(
            "ZRYNA-M7005",
            declaration.name_span(),
            "a source template cannot be selected as a scalar ABI entrypoint",
            "export and select a nongeneric scalar wrapper around the template call",
        )]));
    }
    Ok(())
}
