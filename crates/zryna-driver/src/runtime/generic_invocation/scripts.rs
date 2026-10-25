//! Small compiler-owned scripts carry bytes; target exceptions remain host failures.
use super::invalid;
use zryna_abi::ScalarType;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::invocation;

pub(super) fn javascript(entry: &str, encode: &str) -> Result<Vec<u8>, Diagnostic> {
    let entry = serde_json::to_string(entry).map_err(|_| invalid())?;
    let encode = serde_json::to_string(encode).map_err(|_| invalid())?;
    Ok(format!(
        "const chunks = [];\nfor await (const chunk of process.stdin) chunks.push(chunk);\n\
         const bytes = Buffer.concat(chunks);\n\
         const module = await import('data:text/javascript;base64,' + bytes.toString('base64'));\n\
         const invoke = module[{entry}], encode = module[{encode}];\n\
         if (typeof invoke !== 'function' || typeof encode !== 'function') process.exit(70);\n\
         const record = encode(invoke());\n\
         if (!(record instanceof Uint8Array) || record.byteLength !== 20) process.exit(70);\n\
         process.stdout.write(record);\n"
    )
    .into_bytes())
}

pub(super) fn webassembly(entry: &str, result: ScalarType) -> Result<Vec<u8>, Diagnostic> {
    let entry = serde_json::to_string(entry).map_err(|_| invalid())?;
    let tag = invocation::scalar_tag(result);
    let magic = invocation::RECORD_MAGIC;
    let end = invocation::RECORD_END;
    Ok(format!(
        "const chunks = [];\nfor await (const chunk of process.stdin) chunks.push(chunk);\n\
         const bytes = Buffer.concat(chunks);\n\
         const {{ instance }} = await WebAssembly.instantiate(bytes, {{}});\n\
         const invoke = instance.exports[{entry}];\n\
         if (typeof invoke !== 'function') process.exit(70);\n\
         const packed = invoke();\n\
         if (typeof packed !== 'bigint') process.exit(70);\n\
         const bits = BigInt.asUintN(64, packed);\n\
         const record = Buffer.alloc(20);\n\
         record.writeUInt32LE({magic}, 0);\n\
         record.writeUInt32LE(Number(bits >> 32n), 4);\n\
         record.writeUInt32LE({tag}, 8);\n\
         record.writeUInt32LE(Number(bits & 0xffffffffn), 12);\n\
         record.writeUInt32LE({end}, 16);\n\
         process.stdout.write(record);\n"
    )
    .into_bytes())
}
