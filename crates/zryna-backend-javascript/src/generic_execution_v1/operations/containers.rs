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
        w.format(format_args!("$vecPush($el{child},"))?;
        storage::either(c, f, push.target_place(), push.target_loan(), w)?;
        w.text(",")?;
        super::super::shapes::records::value(c, f, push.value().id(), w)?;
        super::super::runtime::accounting::argument(c, w, true)?;
        return w.text(")");
    }
    if let Some(vector) = op.vec_construct() {
        let child = vector.element_type();
        c.element(child)?;
        w.format(format_args!("$vecNew($el{child},["))?;
        let values = vector.elements();
        charge(&c.work, values.len())?;
        let initialized = values.len();
        for (index, value) in values.enumerate() {
            if index != 0 {
                w.text(",")?;
            }
            super::super::shapes::records::value(c, f, value.id(), w)?;
        }
        w.text("]")?;
        if c.accounting.is_some() {
            w.format(format_args!(",{initialized}"))?;
            super::super::runtime::accounting::argument(c, w, true)?;
        }
        return w.text(")");
    }
    if let Some(shared) = op.shared_construct() {
        let child = shared.referent_type();
        c.control(child)?;
        w.format(format_args!("$controlNew($ctl{child},"))?;
        super::super::shapes::records::value(c, f, shared.payload().id(), w)?;
        super::super::runtime::accounting::argument(c, w, true)?;
        return w.text(")");
    }
    if let Some(weak) = op.weak_downgrade() {
        c.control(weak.referent_type())?;
        w.text("$countDowngrade(")?;
        storage::either(c, f, weak.source_place(), weak.source_loan(), w)?;
        super::super::runtime::accounting::argument(c, w, true)?;
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
        storage::either(c, f, index.source_place(), index.source_loan(), w)?;
        return w.format(format_args!(",$v[{}]))", index.index().id()));
    }
    Err(internal())
}
