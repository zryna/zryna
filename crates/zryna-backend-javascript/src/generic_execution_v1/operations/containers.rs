//! The checked operation view binds exact element, payload, source and result identities.
use super::super::{
    Context, Writer,
    context::{Function, charge},
    internal, storage,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedOperation;
use zryna_layout::TypeCategory;
pub(super) fn emit(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    if let Some(push) = op.vec_push() {
        let child = push.element_type();
        c.element(child)?;
        let target = push.target_place().ok_or_else(internal)?;
        w.format(format_args!("$vecPush($el{child},"))?;
        storage::place(c, f, target, w)?;
        return w.format(format_args!(",$v{})", push.value().id()));
    }
    if let Some(vector) = op.vec_construct() {
        let child = vector.element_type();
        c.element(child)?;
        w.format(format_args!("$vecNew($el{child},["))?;
        let values = vector.elements();
        charge(&c.work, values.len())?;
        for (index, value) in values.enumerate() {
            if index != 0 {
                w.text(",")?;
            }
            w.format(format_args!("$v{}", value.id()))?;
        }
        return w.text("])");
    }
    if let Some(shared) = op.shared_construct() {
        let child = shared.referent_type();
        c.control(child)?;
        return w.format(format_args!("$controlNew($ctl{child},$v{})", shared.payload().id()));
    }
    if let Some(weak) = op.weak_downgrade() {
        c.control(weak.referent_type())?;
        w.text("$countDowngrade(")?;
        storage::place(c, f, weak.source_place().ok_or_else(internal)?, w)?;
        return w.text(")");
    }
    if let Some(index) = op.index_copy() {
        let child = index.element_type();
        let helper = match c.ty(index.container_type())?.category() {
            TypeCategory::Vec => "$vecIndex",
            TypeCategory::FixedArray => "$index",
            _ => return Err(internal()),
        };
        w.format(format_args!("$copy{child}({helper}("))?;
        storage::place(c, f, index.source_place().ok_or_else(internal)?, w)?;
        return w.format(format_args!(",$v{}))", index.index().id()));
    }
    Err(internal())
}
