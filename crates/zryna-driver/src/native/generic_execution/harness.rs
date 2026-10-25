//! Fixed reserved invocation entry and exact20-byte production output, without observation.

use zryna_abi::ScalarType;

const PREFIX: &str = "#include <stdint.h>\n#include <stdio.h>\n\
extern uint32_t zryna_gexecution_invoke_v1(int32_t *out_scalar);\n\
static void put32(uint8_t *out, uint32_t value) {\n\
  for (uint32_t i = 0; i < 4; ++i) out[i] = (uint8_t)(value >> (8U * i));\n\
}\n\
int main(void) {\n\
  int32_t scalar = 0;\n\
  uint32_t status = zryna_gexecution_invoke_v1(&scalar);\n\
  uint32_t bits = 0;\n\
  if (status == 0) bits = (uint32_t)scalar;\n\
  uint8_t record[20];\n\
  put32(record, UINT32_C(0x3149475a));\n\
  put32(record + 4, status);\n";
const SUFFIX: &str = "  put32(record + 12, bits);\n\
  put32(record + 16, UINT32_C(0x31444e45));\n\
  if (fwrite(record, 1, sizeof(record), stdout) != sizeof(record)) return 112;\n\
  if (fflush(stdout) != 0) return 113;\n\
  return 0;\n\
}\n";

pub(super) fn render(result: ScalarType) -> Result<Vec<u8>, zryna_diagnostics::Diagnostic> {
    let tag = match result {
        ScalarType::Bool => "  put32(record + 8, UINT32_C(0));\n",
        ScalarType::I32 => "  put32(record + 8, UINT32_C(1));\n",
    };
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(PREFIX.len() + tag.len() + SUFFIX.len())
        .map_err(|_| super::staging_error())?;
    for part in [PREFIX, tag, SUFFIX] {
        bytes.extend_from_slice(part.as_bytes());
    }
    Ok(bytes)
}
