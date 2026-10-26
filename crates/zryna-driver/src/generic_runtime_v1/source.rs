//! Only target element rows may replace the one fixed compiled-in marker.

use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;

pub(super) const HEADER: &[u8] =
    include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h");
const PARTS: [&str; 3] = [
    include_str!("../../../../runtime/native/generic_execution_v1/allocation.h"),
    include_str!("../../../../runtime/native/generic_execution_v1/string_vec.h"),
    include_str!("../../../../runtime/native/generic_execution_v1/controls.h"),
];
const TEMPLATE_BYTES: usize = 18_363;
const TEMPLATE_SHA256: &str = "df9be4eee10a50225414335502f6b8153874b8d6d4b9ea0903932f505d77f980";
const HEADER_SHA256: &str = "24669a8f02db050f00dd351738f337f5f1d7e5bfa4387c0e942372991945c070";
const MARKER: &str = "/* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */";
// Decimal u32/u64/u64 fields plus fixed C syntax occupy at most 108 bytes.
const ELEMENT_ROW_RESERVATION: usize = 108;

pub(super) fn template_sha256() -> [u8; 32] {
    let mut hash = Sha256::new();
    for part in PARTS {
        hash.update(part.as_bytes());
    }
    hash.finalize().into()
}

fn matches_hex(bytes: &[u8; 32], expected: &str) -> bool {
    const HEX: &[u8] = b"0123456789abcdef";
    expected.len() == 64
        && bytes.iter().enumerate().all(|(i, byte)| {
            expected.as_bytes()[2 * i] == HEX[usize::from(byte >> 4)]
                && expected.as_bytes()[2 * i + 1] == HEX[usize::from(byte & 15)]
        })
}

pub(super) fn render(program: &VerifiedExecution<'_>) -> Result<Vec<u8>, Diagnostic> {
    let abi = program.runtime().declarations();
    if abi.native_header.as_slice() != HEADER
        || abi.native_linux_x86_64.len() != 17
        || program.runtime().operations().len() != 17
        || !matches_hex(&template_sha256(), TEMPLATE_SHA256)
        || !matches_hex(&super::identity::digest(HEADER), HEADER_SHA256)
        || PARTS.iter().map(|part| part.len()).sum::<usize>() != TEMPLATE_BYTES
        || PARTS.iter().map(|part| part.matches(MARKER).count()).sum::<usize>() != 1
        || PARTS.iter().map(|part| part.matches("uint32_t zryna_rt_o1_").count()).sum::<usize>()
            != 17
    {
        return Err(super::rejected());
    }
    let elements = program.runtime().element_layouts();
    let count = elements.iter().filter(|e| e.target() == StorageTarget::LinuxX8664V1).count();
    let reservation = count
        .checked_mul(ELEMENT_ROW_RESERVATION)
        .and_then(|size| size.checked_add(TEMPLATE_BYTES))
        .ok_or_else(super::limit)?;
    if reservation > zryna_backend_native::MAX_NATIVE_OBJECT_BYTES {
        return Err(super::limit());
    }
    let mut source = String::new();
    source.try_reserve_exact(reservation).map_err(|_| super::limit())?;
    for part in PARTS {
        if let Some((before, after)) = part.split_once(MARKER) {
            source.push_str(before);
            for element in elements.iter().filter(|e| e.target() == StorageTarget::LinuxX8664V1) {
                writeln!(
                    source,
                    "  case {}U: *stride = UINT64_C({}); *alignment = {}U; return 1;",
                    element.element().index(),
                    element.stride(),
                    element.alignment()
                )
                .map_err(|_| super::rejected())?;
            }
            source.push_str(after);
        } else {
            source.push_str(part);
        }
    }
    Ok(source.into_bytes())
}
