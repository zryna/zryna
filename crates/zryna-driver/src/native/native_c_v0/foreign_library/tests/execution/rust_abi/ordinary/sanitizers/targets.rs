//! Actual forward-call machine bytes plus independent closed hook/layout expectations.
use super::*;
pub(super) fn compare(
    bytes: &[u8],
    disassembly: &[u8],
    bindings: &policy::Bindings,
) -> Result<u64, Diagnostic> {
    let file = machine::parse(bytes)?;
    let (shim, code) = machine::text(&file, "__wrap_sum_bytes")?;
    let end = shim
        .checked_add(u64::try_from(code.len()).map_err(|_| rejected())?)
        .ok_or_else(rejected)?;
    let real = machine::text(&file, "sum_bytes")?.0;
    let text = std::str::from_utf8(disassembly).map_err(|_| rejected())?;
    let mut mnemonics = Vec::new();
    let mut sites = std::collections::BTreeSet::new();
    let mut hooks = std::collections::BTreeSet::new();
    let mut forward = None;
    for line in text.lines() {
        let Some((left, tail)) = line.split_once(':') else { continue };
        let Ok(site) = u64::from_str_radix(left.trim(), 16) else { continue };
        let parts = tail.split_whitespace().collect::<Vec<_>>();
        let Some(mnemonic) = parts.first() else { return Err(rejected()) };
        if site < shim || site >= end || !sites.insert(site) {
            return Err(rejected());
        }
        mnemonics.push(*mnemonic);
        if matches!(*mnemonic, "call" | "callq") {
            if parts.len() != 3 {
                return Err(rejected());
            }
            let called = u64::from_str_radix(parts[1], 16).map_err(|_| rejected())?;
            if machine::call(&file, site)? != called {
                return Err(rejected());
            }
            if parts[2] == "<sum_bytes>" && called == real {
                if forward.replace(site).is_some() {
                    return Err(rejected());
                }
            } else {
                let name = parts[2]
                    .strip_prefix('<')
                    .and_then(|s| s.strip_suffix('>'))
                    .ok_or_else(rejected)?;
                if !bindings.hooks.contains(&name) {
                    return Err(rejected());
                }
                machine::hook(&file, called, name)?;
                hooks.insert(name);
            }
        }
    }
    bindings.layout(bytes, &mnemonics)?;
    if hooks != bindings.hooks.iter().copied().collect() {
        return Err(rejected());
    }
    forward.ok_or_else(rejected)
}
pub(super) fn verify(
    session: &mut Session<'_>,
    stages: &[native::NativeStage],
    case: Case,
) -> Result<observation::Observation, Diagnostic> {
    let bytes =
        session.artifacts.iter().find(|(n, _)| n == "final.elf").ok_or_else(rejected)?.1.clone();
    let original = &session.measurement.ok_or_else(rejected)?.object;
    super::super::targets::compare_functions(original, &bytes)?;
    let changed = super::super::targets::substituted_function_control(&bytes, "sum_bytes")?;
    if super::super::targets::compare_functions(original, &changed).is_ok() {
        return Err(rejected());
    }
    let stage = stages.first().ok_or_else(rejected)?;
    let args = vec![
        "-d".into(),
        "--no-show-raw-insn".into(),
        "--disassemble=__wrap_sum_bytes".into(),
        stage.capability_file_path("invocation.elf")?.into_os_string(),
    ];
    let tool = session.tools.objdump.clone();
    process::invoke(session, stage, "inspect-shim", &tool, args, false)?;
    process::successful(session)?;
    let data =
        session.records.last().and_then(|r| r.output.as_ref()).ok_or_else(rejected)?.stdout.clone();
    let observation = observation::Observation::capture(session)?;
    observation.check(session, stages)?;
    session.phase = "unavailable-independent-sanitizer-bindings";
    let bindings = policy::reviewed(case).ok_or_else(rejected)?;
    bindings.materials(case)?;
    let callsite = compare(&bytes, &data, bindings)?;
    let redirected = machine::redirect(&bytes, callsite)?;
    super::super::targets::compare_functions(original, &redirected)?;
    if compare(&redirected, &data, bindings).is_ok() {
        return Err(rejected());
    }
    compare(&bytes, &data, bindings)?;
    observation.check(session, stages)?;
    session.artifacts.push(("detached-call-redirect-never-run.elf".into(), redirected));
    session.targets_checked = true;
    Ok(observation)
}
