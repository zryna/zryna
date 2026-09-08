//! Check vector counts before parser allocation for the fixed executable graph.

use wasmparser::{BinaryReader, ComponentExternName, ComponentExternalKind, InstantiationArg};
use zryna_diagnostics::Diagnostic;

use super::type_budget;

pub(super) fn core_instances(bytes: &[u8], offset: u64) -> Result<(), Diagnostic> {
    let mut reader = BinaryReader::new(bytes, offset);
    let count = reader.read_var_u32().map_err(malformed)?;
    if count > 2 {
        return Err(invalid());
    }
    for _ in 0..count {
        if reader.read_u8().map_err(malformed)? != 0 {
            return Err(invalid());
        }
        reader.read_var_u32().map_err(malformed)?;
        let arguments = reader.read_var_u32().map_err(malformed)?;
        if arguments > 1 {
            return Err(invalid());
        }
        for _ in 0..arguments {
            let argument: InstantiationArg<'_> = reader.read().map_err(malformed)?;
            if argument.name.len() > 256 {
                return Err(invalid());
            }
        }
    }
    end(&reader)
}

pub(super) fn canonical(bytes: &[u8], offset: u64) -> Result<(), Diagnostic> {
    let mut reader = BinaryReader::new(bytes, offset);
    if reader.read_var_u32().map_err(malformed)? != 1
        || reader.read_u8().map_err(malformed)? != 0
        || reader.read_u8().map_err(malformed)? != 0
    {
        return Err(invalid());
    }
    reader.read_var_u32().map_err(malformed)?;
    if reader.read_var_u32().map_err(malformed)? != 0 {
        return Err(invalid());
    }
    reader.read_var_u32().map_err(malformed)?;
    end(&reader)
}

pub(super) fn component_instance(bytes: &[u8], offset: u64) -> Result<(), Diagnostic> {
    let mut reader = BinaryReader::new(bytes, offset);
    if reader.read_var_u32().map_err(malformed)? != 1
        || reader.read_u8().map_err(malformed)? != 1
        || reader.read_var_u32().map_err(malformed)? != 1
    {
        return Err(invalid());
    }
    let name: ComponentExternName<'_> = reader.read().map_err(malformed)?;
    type_budget::external_name(name)?;
    let _: ComponentExternalKind = reader.read().map_err(malformed)?;
    reader.read_var_u32().map_err(malformed)?;
    end(&reader)
}

fn end(reader: &BinaryReader<'_>) -> Result<(), Diagnostic> {
    if !reader.eof() {
        return Err(invalid());
    }
    Ok(())
}

fn malformed(_: wasmparser::BinaryReaderError) -> Diagnostic {
    invalid()
}

fn invalid() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4015",
        None,
        "command instantiation or canonical syntax exceeds the fixed execution graph",
        "use only the two retained core instances and single command run lift",
    )
}
