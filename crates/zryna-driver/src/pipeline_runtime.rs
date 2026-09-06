//! Shared scalar runtime harness rendering and result normalization.

use zryna_abi::{
    RawHostScalar, ScalarHostErrorCode, ScalarOutcome, ScalarTarget, ScalarValue,
    VerifiedInvocation,
};

use crate::pipeline::{CommandFailure, invariant_failure};

pub(crate) fn render_javascript_harness(
    stem: &str,
    invocation: &VerifiedInvocation<'_>,
) -> Result<Vec<u8>, CommandFailure> {
    let arguments = render_javascript_arguments(invocation);
    let module_path =
        serde_json::to_string(&format!("./javascript/{stem}.mjs")).map_err(invariant_failure)?;
    let export = invocation.export().javascript_name().as_str();
    let result = match invocation.export().result() {
        zryna_abi::ScalarType::I32 => concat!(
            "if (typeof value !== 'number' || value !== (value | 0) || (value === 0 && 1 / value < 0)) process.exit(70);\n",
            "const frame = Buffer.allocUnsafe(4);\n",
            "frame.writeInt32LE(value, 0);\n",
            "process.stdout.write(frame);\n",
        ),
        zryna_abi::ScalarType::Bool => concat!(
            "if (typeof value !== 'boolean') process.exit(70);\n",
            "process.stdout.write(Uint8Array.of(value ? 1 : 0));\n",
        ),
    };
    Ok(format!(
        "import {{ {export} as invoke }} from {module_path};\nconst value = invoke({arguments});\n{result}"
    )
    .into_bytes())
}

fn render_javascript_arguments(invocation: &VerifiedInvocation<'_>) -> String {
    invocation
        .arguments()
        .iter()
        .map(|argument| match argument {
            ScalarValue::I32(value) => value.to_string(),
            ScalarValue::Bool(value) => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn render_webassembly_harness(
    _stem: &str,
    invocation: &VerifiedInvocation<'_>,
) -> Result<Vec<u8>, CommandFailure> {
    let arguments = render_webassembly_arguments(invocation);
    let export = serde_json::to_string(invocation.export().webassembly_name().as_str())
        .map_err(invariant_failure)?;
    Ok(format!(
        "const chunks = [];\nfor await (const chunk of process.stdin) chunks.push(chunk);\nconst bytes = Buffer.concat(chunks);\nconst {{ instance }} = await WebAssembly.instantiate(bytes, {{}});\nconst invoke = instance.exports[{export}];\nif (typeof invoke !== 'function') process.exit(70);\nconst value = invoke({arguments});\nif (typeof value !== 'number' || value !== (value | 0)) process.exit(70);\nconst frame = Buffer.allocUnsafe(4);\nframe.writeInt32LE(value, 0);\nprocess.stdout.write(frame);\n"
    )
    .into_bytes())
}

fn render_webassembly_arguments(invocation: &VerifiedInvocation<'_>) -> String {
    invocation
        .arguments()
        .iter()
        .map(|argument| match argument {
            ScalarValue::I32(value) => value.to_string(),
            ScalarValue::Bool(value) => i32::from(*value).to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn normalize_frame(
    target: ScalarTarget,
    result_type: zryna_abi::ScalarType,
    frame: [u8; 4],
) -> ScalarOutcome {
    let raw = i32::from_le_bytes(frame);
    let carrier = match target {
        ScalarTarget::JavaScript => RawHostScalar::JavaScriptNumber(f64::from(raw)),
        ScalarTarget::CoreWebAssembly | ScalarTarget::NativeLinuxX8664 => RawHostScalar::I32(raw),
    };
    normalize_carrier(target, result_type, carrier)
}

pub(crate) fn normalize_carrier(
    target: ScalarTarget,
    result_type: zryna_abi::ScalarType,
    carrier: RawHostScalar,
) -> ScalarOutcome {
    match zryna_abi::normalize_result(target, result_type, carrier) {
        Ok(value) => ScalarOutcome::Returned { value },
        Err(_) => ScalarOutcome::HostError { code: ScalarHostErrorCode::InvalidTargetResult },
    }
}
