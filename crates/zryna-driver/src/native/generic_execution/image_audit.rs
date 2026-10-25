//! Completed generic invocation image, including source-required NOBITS TLS storage.

use super::dependencies::ClosureInventory;
use crate::generic_runtime_v1::RuntimeSource;
use object::{Object as _, ObjectSection as _, ObjectSymbol as _};
use sha2::{Digest as _, Sha256};
use zryna_diagnostics::Diagnostic;

const TLS_NAME: &str = "zryna_gexecution_traversal_tls";
const ENTRY: &str = "zryna_gexecution_invoke_v1";

mod identity;
pub(crate) use identity::ImageIdentity;

pub(super) fn check(
    source: &RuntimeSource<'_, '_, '_>,
    bytes: &[u8],
    closure: ClosureInventory,
) -> Result<ImageIdentity, Diagnostic> {
    if bytes.len() > crate::native::MAX_NATIVE_EXECUTABLE_BYTES {
        return Err(rejected());
    }
    let file = object::File::parse(bytes).map_err(|_| rejected())?;
    if file.format() != object::BinaryFormat::Elf
        || file.architecture() != object::Architecture::X86_64
        || file.endianness() != object::Endianness::Little
        || !file.is_64()
        || file.kind() != object::ObjectKind::Executable
        || file.entry() == 0
        || !file.symbols().any(|symbol| {
            symbol.name() == Ok(ENTRY)
                && symbol.is_global()
                && !symbol.is_undefined()
                && !symbol.is_weak()
                && symbol.kind() == object::SymbolKind::Text
                && symbol.address() != 0
                && symbol.size() != 0
        })
    {
        return Err(rejected());
    }
    let tls = source.identity().required_tls_bytes();
    tls_inventory(&file, tls)?;
    // The standalone carrier must not be linked beside ordinary public source exports.
    let ordinary = source
        .program()
        .scalar_abi()
        .exports()
        .map(|export| export.native_linux_x86_64_symbol().as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for symbol in file.symbols() {
        if !symbol.is_undefined() && symbol.name().is_ok_and(|name| ordinary.contains(name)) {
            return Err(rejected());
        }
    }
    headers(bytes, tls)?;
    let sha256: [u8; 32] = Sha256::digest(bytes).into();
    if closure.executable.sha256 != sha256 {
        return Err(rejected());
    }
    ImageIdentity::issue(sha256, tls, closure)
}

fn tls_inventory(file: &object::File<'_>, tls: u64) -> Result<(), Diagnostic> {
    let mut tls_sections = 0;
    for section in file.sections() {
        if let object::SectionFlags::Elf { sh_flags } = section.flags() {
            if sh_flags & 5 == 5 {
                return Err(rejected());
            }
            if sh_flags & 0x400 != 0 || section.name() == Ok(".tbss") {
                tls_sections += 1;
                if tls == 0
                    || section.name() != Ok(".tbss")
                    || section.kind() != object::SectionKind::UninitializedTls
                    || section.size() != tls
                    || section.align() != 16
                    || sh_flags != 0x403
                    || section.file_range().is_some()
                    || !section.data().map_err(|_| rejected())?.is_empty()
                {
                    return Err(rejected());
                }
            }
        }
    }
    let mut tls_symbols = 0;
    for symbol in file.symbols().filter(|symbol| symbol.kind() == object::SymbolKind::Tls) {
        tls_symbols += 1;
        if tls == 0 || tls_symbols > 1 {
            return Err(rejected());
        }
        let section = symbol
            .section_index()
            .and_then(|index| file.section_by_index(index).ok())
            .ok_or_else(rejected)?;
        if symbol.name() != Ok(TLS_NAME)
            || !symbol.is_local()
            || symbol.is_undefined()
            || symbol.size() != tls
            || symbol.address() != 0
            || section.name() != Ok(".tbss")
        {
            return Err(rejected());
        }
    }
    if tls_symbols != usize::from(tls != 0) || tls_sections != tls_symbols {
        return Err(rejected());
    }
    Ok(())
}

fn word(bytes: &[u8], offset: usize, width: usize) -> Result<u64, Diagnostic> {
    let row =
        bytes.get(offset..offset.checked_add(width).ok_or_else(rejected)?).ok_or_else(rejected)?;
    let mut value = [0; 8];
    value.get_mut(..width).ok_or_else(rejected)?.copy_from_slice(row);
    Ok(u64::from_le_bytes(value))
}

fn headers(bytes: &[u8], required_tls: u64) -> Result<(), Diagnostic> {
    let start = usize::try_from(word(bytes, 32, 8)?).map_err(|_| rejected())?;
    let count = usize::try_from(word(bytes, 56, 2)?).map_err(|_| rejected())?;
    if count == 0 || count > 128 || word(bytes, 54, 2)? != 56 {
        return Err(rejected());
    }
    let end =
        start.checked_add(count.checked_mul(56).ok_or_else(rejected)?).ok_or_else(rejected)?;
    let rows = bytes.get(start..end).ok_or_else(rejected)?;
    let mut stack = 0;
    let mut tls = 0;
    for row in rows.chunks_exact(56) {
        let flags = word(row, 4, 4)?;
        match word(row, 0, 4)? {
            1 => {
                if flags & 3 == 3 {
                    return Err(rejected());
                }
            }
            7 => {
                if required_tls == 0
                    || flags != 4
                    || word(row, 32, 8)? != 0
                    || word(row, 40, 8)? != required_tls
                    || word(row, 48, 8)? != 16
                    || word(row, 8, 8)? > bytes.len() as u64
                {
                    return Err(rejected());
                }
                tls += 1;
            }
            0x6474_e551 => {
                if flags & 1 != 0 {
                    return Err(rejected());
                }
                stack += 1;
            }
            _ => {}
        }
    }
    if stack != 1 || tls != usize::from(required_tls != 0) {
        return Err(rejected());
    }
    Ok(())
}

fn rejected() -> Diagnostic {
    super::super::native_error(
        "ZRYNA-C4168",
        "the completed generic invocation image violates its typed source and closed ELF inventory",
        "retain the standalone reserved invocation entry, exact source-required TLS, NX image and captured GNU provider closure",
    )
}
