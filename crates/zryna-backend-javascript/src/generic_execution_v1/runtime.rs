//! Retained successor runtime operation identities and their exact status tables.
use super::{Context, Writer, context::charge, emission_budget, internal};
pub(super) mod accounting;
mod containers;
mod loans;
pub(super) mod ordinary;
use zryna_diagnostics::Diagnostic;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::{LogicalOperation, raw::StatusDisposition};

pub(super) fn declarations(
    c: &Context<'_, '_>,
    w: &mut Writer,
    mode: super::Mode,
) -> Result<(), Diagnostic> {
    let runtime = c.program.runtime();
    validate_runtime(c)?;
    match mode {
        super::Mode::Ordinary(quota) => {
            ordinary::declarations(c, w, quota)?;
            w.text("let $busy = false;\n")?;
        }
        super::Mode::Observed => {
            w.text("const $rt = globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')];\nif (!$rt) throw new Error('ZRYNA-RT-BINDING');\nlet $busy = false;\n")?;
        }
    }
    let requirements = containers::requirements(c)?;
    let base = [
        ("from", LogicalOperation::StringFromUtf8Copy),
        ("clone", LogicalOperation::StringClone),
        ("release", LogicalOperation::StringRelease),
    ];
    let extra = containers::VEC_OPERATIONS
        .into_iter()
        .filter(|_| c.containers.has_vec)
        .chain(containers::CONTROL_OPERATIONS.into_iter().filter(|_| c.containers.has_control))
        .chain(requirements.additions());
    for (private, logical) in base.into_iter().chain(extra) {
        let added = !matches!(
            logical,
            LogicalOperation::StringFromUtf8Copy
                | LogicalOperation::StringClone
                | LogicalOperation::StringRelease
        );
        if added {
            charge(
                &c.work,
                runtime
                    .operations()
                    .len()
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(runtime.declarations().javascript.len()))
                    .ok_or_else(emission_budget)?,
            )?;
        }
        let id =
            runtime.operations().find(|op| op.id().logical() == logical).ok_or_else(internal)?.id();
        let declaration = runtime.operation(id).ok_or_else(internal)?.declaration();
        if declaration.name != logical.name()
            || !runtime
                .declarations()
                .javascript
                .iter()
                .any(|helper| helper.operation == declaration.name)
        {
            return Err(internal());
        }
        if matches!(mode, super::Mode::Ordinary(_)) {
            w.format(format_args!("const ${private}=$rt.{};\nfunction $status_{private}($status) {{ switch ($status) {{\n",logical.name()))?;
        } else {
            w.format(format_args!("if (typeof $rt.{} !== 'function') throw new Error('ZRYNA-RT-BINDING');\nconst ${private}=$rt.{}.bind($rt);\nfunction $status_{private}($status) {{ switch ($status) {{\n", logical.name(), logical.name()))?;
        }
        if added {
            charge(
                &c.work,
                declaration
                    .statuses
                    .len()
                    .checked_mul(runtime.declarations().statuses.len())
                    .ok_or_else(emission_budget)?,
            )?;
        }
        for numeric in &declaration.statuses {
            let status = runtime
                .declarations()
                .statuses
                .iter()
                .find(|s| s.numeric == *numeric)
                .ok_or_else(internal)?;
            w.format(format_args!("case {numeric}: "))?;
            match status.disposition {
                StatusDisposition::Success => w.text("return;\n")?,
                StatusDisposition::ControlledTrap => {
                    let trap = status.trap_identity.as_ref().ok_or_else(internal)?;
                    let identity = super::invocation::traps::identity(trap).ok_or_else(internal)?;
                    w.throws_source(identity)?;
                }
                StatusDisposition::HostFailure => w.text("throw new Error('ZRYNA-RT-ABI');\n")?,
                StatusDisposition::Branch => {
                    if logical != LogicalOperation::WeakUpgrade || *numeric != 5 {
                        return Err(internal());
                    }
                    w.text("return;\n")?;
                }
            }
        }
        w.text("default: throw new Error('ZRYNA-RT-ABI');\n} }\n")?;
    }
    helpers(c, w, mode)?;
    containers::declarations(c, w, requirements)
}

fn helpers(c: &Context<'_, '_>, w: &mut Writer, mode: super::Mode) -> Result<(), Diagnostic> {
    w.text("function $handle($result,$status) {\nif (!$result || !Number.isInteger($result.status)) throw new Error('ZRYNA-RT-ABI');\n$status($result.status);\nif (!$result.handle || typeof $result.handle !== 'object') throw new Error('ZRYNA-RT-ABI');\nreturn $result.handle;\n}\n")?;
    if c.accounting.is_some() {
        if !matches!(mode, super::Mode::Ordinary(_)) {
            return Err(internal());
        }
        accounting::declarations(c, w)?;
    } else if matches!(mode, super::Mode::Ordinary(_)) {
        w.text("function $fresh($bytes) { const $buffer=new Uint8Array($bytes.length); for(let $i=0;$i<$bytes.length;$i++) $buffer[$i]=$bytes[$i]; return $handle($from($buffer,$bytes.length),$status_from); }\n")?;
    } else {
        w.text("function $fresh($bytes) { return $handle($from(new Uint8Array($bytes),$bytes.length),$status_from); }\n")?;
    }
    if c.accounting.is_none() {
        w.text("function $stringClone($source) { return $handle($clone($source),$status_clone); }\nfunction $free($value) { $status_release($release($value)); }\n")?;
    }
    w.trusted("function $index($array,$index) { if (($index|0)!==$index || $index<0 || $index>=$array.length) throw new Error('zryna.trap.bounds-v1'); return $array[$index]; }\n")?;
    Ok(())
}

fn validate_runtime(c: &Context<'_, '_>) -> Result<(), Diagnostic> {
    let runtime = c.program.runtime();
    if !runtime.is_bound_to(
        c.program.layouts(StorageTarget::Linear32V1),
        c.program.layouts(StorageTarget::LinuxX8664V1),
    ) {
        return Err(internal());
    }
    Ok(())
}
