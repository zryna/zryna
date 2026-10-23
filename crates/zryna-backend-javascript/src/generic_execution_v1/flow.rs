//! Checked private signatures, simultaneous CFG edges and scalar entry wrappers.
use super::{
    Context, Writer,
    context::{Function, charge},
    internal, operations, storage,
};
use zryna_diagnostics::Diagnostic;
mod upgrade;
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, VerifiedParameter, raw};

pub(super) fn function(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    if !f.loans.is_empty() {
        return Err(internal());
    }
    w.format(format_args!("function $g{}(", f.view.index()))?;
    parameters(f.parameters.len(), w)?;
    w.text(") {\n")?;
    for id in 0..f.values.len() {
        charge(&c.work, 1)?;
        w.format(format_args!("let $v{id};\n"))?;
    }
    for (ordinal, parameter) in f.parameters.iter().enumerate() {
        match parameter {
            VerifiedParameter::Value(value) => {
                w.format(format_args!("$v{}=$a{ordinal};\n", value.id()))?;
            }
            VerifiedParameter::Loan(_) => return Err(internal()),
        }
    }
    for place in &f.places {
        charge(&c.work, 1)?;
        match place.kind() {
            raw::PlaceKind::Parameter(ordinal) => {
                w.format(format_args!("let $p{}=$a{ordinal};\n", place.id()))?;
            }
            raw::PlaceKind::Local(_) => w.format(format_args!("let $p{};\n", place.id()))?,
            _ => {}
        }
    }
    w.text("let $block=0; while(true) { switch($block) {\n")?;
    for block in &f.blocks {
        charge(&c.work, 1)?;
        w.format(format_args!("case {}: {{\n", block.id()))?;
        for op in block.operations() {
            operations::emit(c, f, op, w)?;
        }
        terminator(c, f, *block, w)?;
        w.text("}\n")?;
    }
    w.text("default: throw new Error('ZRYNA-I7001: invalid private block');\n} } }\n")
}
fn terminator(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    block: VerifiedBlock<'_, '_>,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    match block.terminator() {
        raw::Terminator::Return { value, .. } => {
            w.text("const $return=")?;
            if let Some(value) = value {
                w.format(format_args!("$v{value}"))?;
            } else {
                w.text("undefined")?;
            }
            w.text(";\n")?;
            storage::cleanup(c, f, block.exit_cleanup().ok_or_else(internal)?, None, w)?;
            w.text("return $return;\n")
        }
        raw::Terminator::Jump(jump) => edge(c, f, jump, 0, w),
        raw::Terminator::Branch { condition, yes, no } => {
            w.format(format_args!("if($v{condition}) {{\n"))?;
            edge(c, f, yes, 0, w)?;
            w.text("} else {\n")?;
            edge(c, f, no, 0, w)?;
            w.text("}\n")
        }
        raw::Terminator::ClosedEnumMatch { scrutinee, mode, arms, .. } => {
            enum_match(c, f, *scrutinee, *mode, arms, w)
        }
        raw::Terminator::Trap { identity, .. } => {
            storage::cleanup(c, f, block.exit_cleanup().ok_or_else(internal)?, None, w)?;
            let trap = match identity {
                raw::TrapIdentity::Bounds => "zryna.trap.bounds-v1",
                raw::TrapIdentity::Allocation => "zryna.trap.allocation-v1",
                raw::TrapIdentity::Capacity => "zryna.trap.capacity-v1",
                raw::TrapIdentity::Refcount => "zryna.trap.refcount-v1",
                raw::TrapIdentity::Utf8 => "zryna.trap.utf8-v1",
            };
            w.format(format_args!("throw new Error('{trap}');\n"))
        }
        raw::Terminator::WeakUpgradeBranch { .. } => {
            upgrade::emit(c, f, block.weak_upgrade().ok_or_else(internal)?, w)
        }
    }
}
fn enum_match(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    scrutinee: raw::PlaceOrLoan,
    mode: raw::MatchMode,
    arms: &[raw::EnumArm],
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    if mode != raw::MatchMode::Value {
        return Err(internal());
    }
    w.text("const $match=")?;
    storage::source(c, f, scrutinee, w)?;
    w.text(";\n")?;
    storage::source(c, f, scrutinee, w)?;
    w.text("=undefined;\nswitch($match.tag) {\n")?;
    for arm in arms {
        charge(&c.work, 1)?;
        w.format(format_args!("case {}: {{\n", arm.ordinal))?;
        let offset = if let Some(binding) = &arm.binding {
            match binding {
                raw::MatchBinding::Value(value) => {
                    w.format(format_args!("$v{}=$match.payload;\n", value.id))?;
                }
                raw::MatchBinding::Loan(_) => return Err(internal()),
            }
            1
        } else {
            0
        };
        edge(c, f, &arm.edge, offset, w)?;
        w.text("}\n")?;
    }
    w.text("default: throw new Error('ZRYNA-RT-TAG');\n}\n")
}
fn edge(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    edge: &raw::Edge,
    offset: usize,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    let target = f.blocks.get(edge.target as usize).ok_or_else(internal)?;
    w.text("const $edge=[")?;
    for (index, value) in edge.arguments.iter().enumerate() {
        charge(&c.work, 1)?;
        if index != 0 {
            w.text(",")?;
        }
        w.format(format_args!("$v{value}"))?;
    }
    w.text("];\n")?;
    for (index, parameter) in target.value_parameters().skip(offset).enumerate() {
        charge(&c.work, 1)?;
        w.format(format_args!("$v{}=$edge[{index}];\n", parameter.id()))?;
    }
    w.format(format_args!("$block={}; continue;\n", edge.target))
}
pub(super) fn exports(
    c: &Context<'_, '_>,
    w: &mut Writer,
    ordinary: bool,
) -> Result<(), Diagnostic> {
    for (export, target) in c.program.scalar_abi().exports().zip(c.program.export_functions()) {
        let f = c.functions.get(*target).ok_or_else(internal)?;
        let id = export.index();
        w.format(format_args!("function $export{id}("))?;
        parameters(f.parameters.len(), w)?;
        w.format(format_args!(
            ") {{\n$zryna$checkArity(arguments.length,{});\n",
            f.parameters.len()
        ))?;
        for (ordinal, parameter) in f.parameters.iter().enumerate() {
            let VerifiedParameter::Value(value) = parameter else {
                return Err(internal());
            };
            let check = scalar_check(value.ty())?;
            w.format(format_args!("$a{ordinal}={check}($a{ordinal});\n"))?;
        }
        w.text("if($busy) throw new Error('ZRYNA-RT-REENTRY'); $busy=true;\ntry { return ")?;
        if ordinary {
            w.text("($runtimeEnter(),")?;
        }
        w.format(format_args!("{}($g{target}(", scalar_check(f.view.result_type())?))?;
        parameters(f.parameters.len(), w)?;
        w.text("))")?;
        if ordinary {
            w.text(")")?;
        }
        w.text("; } finally { $busy=false;")?;
        if ordinary {
            w.text(" $runtimeExit();")?;
        }
        w.format(format_args!(
            " }}\n}}\nexport {{ $export{id} as {} }};\n",
            export.javascript_name().as_str()
        ))?;
    }
    if c.program.scalar_abi().exports().len() == 0 {
        w.text("export {};\n")?;
    }
    Ok(())
}
fn scalar_check(ty: raw::ValueType) -> Result<&'static str, Diagnostic> {
    match ty {
        raw::ValueType::Stored(0) => Ok("$zryna$bool"),
        raw::ValueType::Stored(1) => Ok("$zryna$i32"),
        _ => Err(internal()),
    }
}
fn parameters(count: usize, w: &mut Writer) -> Result<(), Diagnostic> {
    for id in 0..count {
        if id != 0 {
            w.text(",")?;
        }
        w.format(format_args!("$a{id}"))?;
    }
    Ok(())
}
