//! Ordinary module-private fixed17 kernel identity, exact same-program emission and quota proof.
use super::super::{Context, Writer, context::charge, emission_budget, internal};
use zryna_diagnostics::Diagnostic;
use zryna_layout::StorageTarget;
pub(in crate::generic_execution_v1) mod quota;
use quota::Quota;
#[cfg(test)]
mod reserve_tests;
const KERNEL: &str = concat!(
    "// Pure private successor runtime; no source/host callbacks or ambient runtime injection.\nlet $runtimeEnter,$runtimeExit;\nconst $rt=(()=>{\n",
    include_str!("ordinary/kernel/allocation.js"),
    include_str!("ordinary/kernel/strings.js"),
    include_str!("ordinary/kernel/containers.js"),
    "return Object.freeze(Object.assign(Object.create(null),{\nallocate:function($n,$a){$step();return $allocate($n,$a);},\ngrow:function($p,$o,$n,$a){$step();return $grow($p,$o,$n,$a);},\nrelease:function($p,$n,$a){$step();return $release($p,$n,$a);},\nstringFromUtf8Copy:function($b,$n){$step();return $from($b,$n);},\nstringClone:function($s){$step();return $clone($s);},\nstringConcat:function($a,$b){$step();return $concat($a,$b);},\nstringRelease:function($s){$step();return $stringFree($s);},\nvecAllocate:function($l,$n){$step();return $vecAllocate($l,$n);},\nvecReserve:function($l,$s,$n){$step();return $vecReserve($l,$s,$n);},\nvecReleaseStorage:function($l,$s){$step();return $vecFree($l,$s);},\nstrongClone:function($c){$step();return $strongClone($c);},\nweakDowngrade:function($c){$step();return $weakDowngrade($c);},\nweakClone:function($c){$step();return $weakClone($c);},\nweakUpgrade:function($c){$step();return $upgrade($c);},\nstrongReleaseBegin:function($c){$step();return $strongBegin($c);},\nstrongReleaseFinish:function($c){$step();return $strongFinish($c);},\nweakRelease:function($c){$step();return $weakFree($c);}\n}));\n})();\n",
);
const IMPLEMENTATION: &str = "987fa645b5f81a9c6fd03f7edcbd23203e7cc61e8aa206b1b08a5ef61d3b1421";
pub(in crate::generic_execution_v1) fn declarations(
    c: &Context<'_, '_>,
    w: &mut Writer,
    quota: Quota,
) -> Result<(), Diagnostic> {
    let runtime = c.program.runtime();
    charge(
        &c.work,
        KERNEL.len().checked_add(c.containers.elements.len()).ok_or_else(emission_budget)?,
    )?;
    w.text("const $runtimeManifest=Object.freeze(Object.assign(Object.create(null),{abi:'zryna-generic-ownership-runtime-v1',mode:'ordinary',helpers:17,declarations:'")?;
    hex(runtime.identity().fingerprint(), w)?;
    w.text("',linear:'")?;
    hex(*c.program.layouts(StorageTarget::Linear32V1).fingerprint(), w)?;
    w.text("',linux:'")?;
    hex(*c.program.layouts(StorageTarget::LinuxX8664V1).fingerprint(), w)?;
    let accounting = if c.accounting.is_some() { super::accounting::MANIFEST } else { "" };
    if c.accounting.is_some() {
        let expected = super::accounting::quota()?;
        if quota.allocations != expected.allocations || quota.transitions != expected.transitions {
            return Err(internal());
        }
    }
    w.format(format_args!("',implementation:'{IMPLEMENTATION}'{accounting}}}));\nconst $runtimeQuota=Object.freeze({{allocations:{},transitions:{}}});\nconst $runtimeElements=Object.freeze(Object.assign(Object.create(null),{{",quota.allocations,quota.transitions))?;
    for (id, layout) in c.containers.elements.iter().enumerate() {
        if let Some(layout) = layout {
            w.format(format_args!(
                "{id}:Object.freeze({{stride:{},alignment:{}}}),",
                layout.stride(),
                layout.alignment()
            ))?;
        }
    }
    w.text("}));\n")?;
    w.text(KERNEL)
}
fn hex(bytes: [u8; 32], w: &mut Writer) -> Result<(), Diagnostic> {
    for b in bytes {
        w.format(format_args!("{b:02x}"))?;
    }
    Ok(())
}
/// Exact closed emission comparison checks every byte, including inventory/manifest/body/mode.
/// This is deliberately private: callers cannot provide a replacement emitter or authority.
pub(in crate::generic_execution_v1) fn audit(
    c: &Context<'_, '_>,
    actual: &str,
    mode: super::super::Mode,
) -> Result<(), Diagnostic> {
    let mut counter = Writer::counter(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES);
    super::super::render(c, &mut counter, mode)?;
    charge(&c.work, counter.bytes().checked_add(actual.len()).ok_or_else(emission_budget)?)?;
    let mut expected =
        Writer::output(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES, counter.bytes())?;
    super::super::render(c, &mut expected, mode)?;
    if expected.finish() != actual {
        return Err(internal());
    }
    Ok(())
}

#[cfg(test)]
mod accounting_tests;
