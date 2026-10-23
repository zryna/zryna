//! The checked terminator publishes its generated Shared owner only on canonical success.
use super::super::{
    Context, Writer,
    context::{Function, charge},
    internal, storage,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedUpgradeEdge, VerifiedWeakUpgrade};

pub(super) fn emit(
    context: &Context<'_, '_>,
    function: &Function<'_, '_>,
    upgrade: VerifiedWeakUpgrade<'_, '_>,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    context.control(upgrade.referent_type())?;
    let source = upgrade.source_place().ok_or_else(internal)?;
    writer.text("const $upgradeSource=")?;
    storage::place(context, function, source, writer)?;
    writer.text(";\nlet $upgradeStatus;\ntry {\n$upgradeStatus=$upgrade($upgradeSource);\n$status_upgrade($upgradeStatus);\n} catch($failure) {\n")?;
    storage::cleanup(context, function, upgrade.outer_failure(), None, writer)?;
    writer.text("throw $failure;\n}\nif($upgradeStatus===0) {\n")?;
    writer.format(format_args!("$v{}=$upgradeSource;\n", upgrade.success_binding().id()))?;
    edge(context, upgrade.success(), 1, writer)?;
    writer.text("} else {\n")?;
    edge(context, upgrade.expired(), 0, writer)?;
    writer.text("}\n")
}

fn edge(
    context: &Context<'_, '_>,
    edge: VerifiedUpgradeEdge<'_, '_>,
    offset: usize,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    let target = edge.target();
    let arguments = edge.arguments();
    charge(&context.work, arguments.len())?;
    writer.text("const $edge=[")?;
    for (index, argument) in arguments.enumerate() {
        if index != 0 {
            writer.text(",")?;
        }
        writer.format(format_args!("$v{}", argument.id()))?;
    }
    writer.text("];\n")?;
    let parameters = target.value_parameters().skip(offset);
    charge(&context.work, parameters.len())?;
    for (index, parameter) in parameters.enumerate() {
        writer.format(format_args!("$v{}=$edge[{index}];\n", parameter.id()))?;
    }
    writer.format(format_args!("$block={}; continue;\n", target.id()))
}
