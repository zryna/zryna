use crate::{CommandFailure, pipeline::invariant_failure};
use zryna_abi::{
    ScalarOutcome, ScalarTarget, ScalarTrapCode, ScalarType, ScalarValue, VerifiedInvocation,
};

pub(super) fn decode(frame: [u8; 8], result: ScalarType) -> Result<ScalarOutcome, CommandFailure> {
    let status = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
    let value: [u8; 4] = [frame[4], frame[5], frame[6], frame[7]];
    let trap = match status {
        0 => {
            return Ok(crate::pipeline_runtime::normalize_frame(
                ScalarTarget::CoreWebAssembly,
                result,
                value,
            ));
        }
        1 => ScalarTrapCode::Bounds,
        2 => ScalarTrapCode::Allocation,
        3 => ScalarTrapCode::Capacity,
        4 => ScalarTrapCode::Refcount,
        5 => ScalarTrapCode::Utf8,
        _ => {
            return Err(super::execution_failure(
                "ZRYNA-C3302",
                "invalid candidate observation status",
            ));
        }
    };
    if value != [0; 4] {
        return Err(super::execution_failure(
            "ZRYNA-C3302",
            "trapped candidate returned a scalar payload",
        ));
    }
    Ok(ScalarOutcome::Trapped { code: trap })
}

pub(super) fn javascript(
    stem: &str,
    invocation: &VerifiedInvocation<'_>,
) -> Result<Vec<u8>, CommandFailure> {
    let path =
        serde_json::to_string(&format!("./javascript/{stem}.mjs")).map_err(invariant_failure)?;
    let export = invocation.export().javascript_name().as_str();
    let arguments = invocation
        .arguments()
        .iter()
        .map(|value| match value {
            ScalarValue::I32(value) => value.to_string(),
            ScalarValue::Bool(value) => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let carrier = match invocation.export().result() {
        ScalarType::Bool => {
            "if (status === 0 && typeof value !== 'boolean') process.exit(70); value = status === 0 ? (value ? 1 : 0) : value;"
        }
        ScalarType::I32 => "",
    };
    Ok(format!("import {{ {export} as invoke, $zryna$observation as observe }} from {path};\nlet value = invoke({arguments});\nconst status = observe();\n{carrier}\n{FRAME}").into_bytes())
}

pub(super) fn webassembly(invocation: &VerifiedInvocation<'_>) -> Result<Vec<u8>, CommandFailure> {
    let export = serde_json::to_string(invocation.export().webassembly_name().as_str())
        .map_err(invariant_failure)?;
    let arguments = invocation
        .arguments()
        .iter()
        .map(|value| match value {
            ScalarValue::I32(value) => value.to_string(),
            ScalarValue::Bool(value) => i32::from(*value).to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!("const chunks = []; for await (const chunk of process.stdin) chunks.push(chunk);\nconst {{instance}} = await WebAssembly.instantiate(Buffer.concat(chunks), {{}});\nconst value = instance.exports[{export}]({arguments});\nconst status = instance.exports.$zryna$observation(0);\n{FRAME}").into_bytes())
}

const FRAME: &str = "if (!Number.isInteger(status) || status < 0 || status > 5 || typeof value !== 'number' || value !== (value | 0) || Object.is(value, -0)) process.exit(70);\nconst frame = Buffer.alloc(8); frame.writeUInt32LE(status, 0); frame.writeInt32LE(value, 4); process.stdout.write(frame);\n";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_status_channel_rejects_host_errors_and_partial_frames() {
        let names = ["bounds", "allocation", "capacity", "refcount", "utf8"];
        for (index, name) in names.iter().enumerate() {
            let mut frame = [0; 8];
            frame[0] = u8::try_from(index + 1).unwrap();
            assert_eq!(
                serde_json::to_value(decode(frame, ScalarType::I32).unwrap()).unwrap(),
                serde_json::json!({"kind": "trapped", "code": format!("zryna.trap.{name}-v1")})
            );
            frame[4] = 1;
            assert!(decode(frame, ScalarType::I32).is_err());
        }
        for status in [6, 255] {
            let mut frame = [0; 8];
            frame[0] = status;
            assert!(decode(frame, ScalarType::I32).is_err());
        }
    }
}
