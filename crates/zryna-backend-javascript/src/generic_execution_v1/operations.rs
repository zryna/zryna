//! Every expression is selected by an operation view issued by the new seal.
use super::{
    Context, Writer,
    context::{Function, charge},
    internal, storage,
};
use zryna_diagnostics::Diagnostic;
mod containers;
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, raw};

pub(super) fn emit(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    charge(&c.work, 1)?;
    let clone = op.clone_frontier();
    let plan = if let Some(clone) = clone {
        storage::check_prefix(clone, op.site())?;
        clone.prefix_failure()
    } else {
        op.failure()
    };
    if plan.is_some() {
        w.text("try {\n")?;
    }
    if let Some(result) = op.result() {
        w.format(format_args!("$v{}=", result.id()))?;
    }
    expression(c, f, op, w)?;
    w.text(";\n")?;
    if let Some(plan) = plan {
        w.text("} catch($failure) {\n")?;
        storage::cleanup(c, f, plan, clone.map(|_| op.site()), w)?;
        w.text("throw $failure;\n}\n")?;
    }
    Ok(())
}
fn expression(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    use raw::Operation as O;
    match op.kind() {
        O::Unit => w.text("undefined"),
        O::BoolLiteral(value) => w.format(format_args!("{value}")),
        O::I32Literal(value) => w.format(format_args!("{value}")),
        O::ScalarBinary { kind, left, right } => binary(*kind, *left, *right, w),
        O::I32Neg { value } => w.format(format_args!("((-$v{value})|0)")),
        O::ReadPlace { place, mode } => read_place(c, f, *place, *mode, w),
        O::InitializePlace { place, value } => {
            storage::place(c, f, f.place(*place)?, w)?;
            w.format(format_args!("=$v{value}"))
        }
        O::ReplacePlace { place, value } => {
            w.text("(() => {\n")?;
            storage::drop_old(c, f, op, f.place(*place)?, w)?;
            storage::place(c, f, f.place(*place)?, w)?;
            w.format(format_args!("=$v{value}; }})()"))
        }
        O::DropPlace { place } => {
            w.text("(() => {\n")?;
            storage::drop_old(c, f, op, f.place(*place)?, w)?;
            w.text("})()")
        }
        O::Clone { .. } => {
            let clone = op.clone_frontier().ok_or_else(internal)?;
            w.format(format_args!("$clone{}(", clone.ty()))?;
            storage::clone_source(c, f, clone, w)?;
            w.text(")")
        }
        _ => compound(c, f, op, w),
    }
}
fn compound(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    use raw::Operation as O;
    match op.kind() {
        O::ClosedGenericCall { arguments, .. } | O::SourceCall { arguments, .. } => {
            let target = op.call_target().ok_or_else(internal)?;
            w.format(format_args!("$g{}(", target.index()))?;
            for (index, argument) in arguments.iter().enumerate() {
                charge(&c.work, 1)?;
                if index != 0 {
                    w.text(",")?;
                }
                match argument {
                    raw::CallArgument::Value(value) => w.format(format_args!("$v{value}"))?,
                    raw::CallArgument::Loan(_) => return Err(internal()),
                }
            }
            w.text(")")
        }
        O::StructConstruct { fields, .. } | O::FixedArrayConstruct { elements: fields, .. } => {
            values(c, fields, w)
        }
        O::ClosedEnumConstruct { ordinal, payload, .. } => {
            w.format(format_args!("{{tag:{ordinal},payload:"))?;
            if let Some(value) = payload {
                w.format(format_args!("$v{value}"))?;
            } else {
                w.text("undefined")?;
            }
            w.text("}")
        }
        O::StringFromUtf8 { bytes, .. } => {
            w.text("$fresh([")?;
            for (index, byte) in bytes.iter().enumerate() {
                charge(&c.work, 1)?;
                if index != 0 {
                    w.text(",")?;
                }
                w.format(format_args!("{byte}"))?;
            }
            w.text("])")
        }
        O::VecConstruct { .. }
        | O::SharedConstruct { .. }
        | O::WeakDowngrade { .. }
        | O::IndexCopy { .. }
        | O::VecPush { .. } => containers::emit(c, f, op, w),
        _ => Err(internal()),
    }
}

fn read_place(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    id: u32,
    mode: raw::ReadMode,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    let place = f.place(id)?;
    match mode {
        raw::ReadMode::Copy => {
            w.format(format_args!("$copy{}(", place.ty()))?;
            storage::place(c, f, place, w)?;
            w.text(")")
        }
        raw::ReadMode::Move => {
            w.text("(() => {const $m=")?;
            storage::place(c, f, place, w)?;
            w.text(";")?;
            storage::place(c, f, place, w)?;
            w.text("=undefined;return $m;})()")
        }
    }
}
fn values(c: &Context<'_, '_>, values: &[u32], w: &mut Writer) -> Result<(), Diagnostic> {
    w.text("[")?;
    for (index, value) in values.iter().enumerate() {
        charge(&c.work, 1)?;
        if index != 0 {
            w.text(",")?;
        }
        w.format(format_args!("$v{value}"))?;
    }
    w.text("]")
}
fn binary(
    kind: raw::ScalarBinaryKind,
    left: u32,
    right: u32,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    use raw::ScalarBinaryKind as B;
    match kind {
        B::Add => w.format(format_args!("(($v{left}+$v{right})|0)")),
        B::Sub => w.format(format_args!("(($v{left}-$v{right})|0)")),
        B::Mul => w.format(format_args!("Math.imul($v{left},$v{right})")),
        comparison => {
            let operator = match comparison {
                B::Eq => "===",
                B::Ne => "!==",
                B::LtS => "<",
                B::LeS => "<=",
                B::GtS => ">",
                B::GeS => ">=",
                _ => return Err(internal()),
            };
            w.format(format_args!("($v{left}{operator}$v{right})"))
        }
    }
}
