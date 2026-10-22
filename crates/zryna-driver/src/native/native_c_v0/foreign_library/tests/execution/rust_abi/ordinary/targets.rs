//! Closed original Rust function-byte and real-call comparison before ordinary execution.
use super::*;
use object::{Object as _, ObjectSection as _, ObjectSymbol as _};

fn symbol_bytes<'a>(file: &object::File<'a>, name: &str) -> Result<(u64, &'a [u8]), Diagnostic> {
    let mut symbols = file.symbols().filter(|s| s.name().ok() == Some(name));
    let symbol = symbols.next().ok_or_else(rejected)?;
    if symbols.next().is_some()
        || symbol.is_undefined()
        || symbol.kind() != object::SymbolKind::Text
        || symbol.size() == 0
    {
        return Err(rejected());
    }
    let section = file
        .section_by_index(symbol.section_index().ok_or_else(rejected)?)
        .map_err(|_| rejected())?;
    let bytes = section
        .data_range(symbol.address(), symbol.size())
        .map_err(|_| rejected())?
        .ok_or_else(rejected)?;
    Ok((symbol.address(), bytes))
}
pub(super) fn compare_functions(original: &[u8], final_elf: &[u8]) -> Result<(), Diagnostic> {
    let original = object::File::parse(original).map_err(|_| rejected())?;
    let final_file = object::File::parse(final_elf).map_err(|_| rejected())?;
    for name in ["add", "sum_bytes"] {
        if symbol_bytes(&original, name)?.1 != symbol_bytes(&final_file, name)?.1 {
            return Err(rejected());
        }
    }
    Ok(())
}
pub(super) fn compare_target(
    disassembly: &[u8],
    address: u64,
    target: &str,
) -> Result<(), Diagnostic> {
    let text = std::str::from_utf8(disassembly).map_err(|_| rejected())?;
    let mut matched = 0;
    for line in text.lines() {
        let Some((_, tail)) = line.split_once(':') else {
            continue;
        };
        let parts = tail.split_whitespace().collect::<Vec<_>>();
        if matches!(parts.first(), Some(&"call" | &"callq")) {
            if parts.len() != 3 {
                return Err(rejected());
            }
            let called = u64::from_str_radix(parts[1], 16).map_err(|_| rejected())?;
            if parts[2] == format!("<{target}>") && called == address {
                matched += 1;
            } else if parts[2] != "<__assert_fail@plt>" {
                return Err(rejected());
            }
        }
    }
    if matched != 1 {
        return Err(rejected());
    }
    Ok(())
}
pub(super) fn verify_rust_targets(
    session: &mut Session<'_>,
    stage: &native::NativeStage,
    target: &str,
) -> Result<(), Diagnostic> {
    session.phase = "rust-function-byte-comparison";
    let final_bytes = session
        .artifacts
        .iter()
        .find(|(name, _)| name == "final.elf")
        .ok_or_else(rejected)?
        .1
        .clone();
    let rust = session.measurement.ok_or_else(rejected)?;
    compare_functions(&rust.object, &final_bytes)?;
    let file = object::File::parse(final_bytes.as_slice()).map_err(|_| rejected())?;
    let address = symbol_bytes(&file, target)?.0;
    let args = vec![
        "-d".into(),
        "--no-show-raw-insn".into(),
        format!("--disassemble=__wrap_{target}").into(),
        stage.capability_file_path("invocation.elf")?.into_os_string(),
    ];
    let objdump = session.tools.objdump.clone();
    process::invoke(session, stage, "inspect-shim", &objdump, args, false)?;
    process::successful(session)?;
    let data = &session.records.last().and_then(|r| r.output.as_ref()).ok_or_else(rejected)?.stdout;
    session.phase = "rust-real-call-target-comparison";
    compare_target(data, address, target)?;
    session.targets_checked = true;
    Ok(())
}

/// Explicitly hostile alteration of retained final function bytes; never executed.
pub(super) fn substituted_function_control(
    bytes: &[u8],
    target: &str,
) -> Result<Vec<u8>, Diagnostic> {
    let file = object::File::parse(bytes).map_err(|_| rejected())?;
    let mut symbols = file.symbols().filter(|s| s.name().ok() == Some(target));
    let symbol = symbols.next().ok_or_else(rejected)?;
    if symbols.next().is_some() {
        return Err(rejected());
    }
    let section = file
        .section_by_index(symbol.section_index().ok_or_else(rejected)?)
        .map_err(|_| rejected())?;
    let offset = section
        .file_range()
        .ok_or_else(rejected)?
        .0
        .checked_add(symbol.address().checked_sub(section.address()).ok_or_else(rejected)?)
        .ok_or_else(rejected)?;
    let offset = usize::try_from(offset).map_err(|_| rejected())?;
    let mut changed = bytes.to_vec();
    *changed.get_mut(offset).ok_or_else(rejected)? ^= 1;
    Ok(changed)
}
