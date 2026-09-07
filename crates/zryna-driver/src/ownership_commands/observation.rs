use crate::{CommandFailure, pipeline::invariant_failure};
use zryna_abi::{
    ScalarOutcome, ScalarTarget, ScalarTrapCode, ScalarType, ScalarValue, VerifiedInvocation,
};

pub(super) const MAX_FRAME: usize = 12 + 4096 * 4;

#[derive(Clone, Copy)]
pub(super) struct Fault(u32);
impl Fault {
    #[cfg(test)]
    pub(super) fn new(code: u32, ordinal: u32) -> Result<Self, CommandFailure> {
        if !(2..=5).contains(&code) || !(1..=1_048_576).contains(&ordinal) {
            return Err(CommandFailure {
                kind: crate::CommandFailureKind::Request,
                diagnostics: vec![zryna_diagnostics::Diagnostic::error(
                    "ZRYNA-C3303",
                    None,
                    "invalid candidate fault selection",
                    "use a defined trap and bounded positive ordinal",
                )],
            });
        }
        Ok(Self(0x2000_0000 | (code << 24) | ordinal))
    }
    #[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
    pub(super) fn physical_allocation(ordinal: u32) -> Self {
        assert!((1..=1_048_576).contains(&ordinal));
        Self(0x2600_0000 | ordinal)
    }
    pub(super) const fn command(self) -> u32 {
        self.0
    }
}

fn invalid() -> CommandFailure {
    super::execution_failure("ZRYNA-C3302", "invalid candidate observation frame or trace")
}

pub(crate) fn decode(
    frame: &[u8],
    result: ScalarType,
    target: crate::OwnershipTarget,
) -> Result<crate::OwnershipManifestResult, CommandFailure> {
    use crate::{OwnershipTraceEvent as E, OwnershipValueKind as V};
    if frame.len() < 12 || frame.len() > MAX_FRAME || !frame.len().is_multiple_of(4) {
        return Err(invalid());
    }
    let words = frame
        .chunks_exact(4)
        .map(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
        .collect::<Vec<_>>();
    if usize::try_from(words[2]).ok() != Some(words.len() - 3) {
        return Err(invalid());
    }
    let outcome = match words[0] {
        0 => crate::pipeline_runtime::normalize_frame(
            ScalarTarget::CoreWebAssembly,
            result,
            words[1].to_le_bytes(),
        ),
        code @ 1..=5 if words[1] == 0 => ScalarOutcome::Trapped {
            code: match code {
                1 => ScalarTrapCode::Bounds,
                2 => ScalarTrapCode::Allocation,
                3 => ScalarTrapCode::Capacity,
                4 => ScalarTrapCode::Refcount,
                _ => ScalarTrapCode::Utf8,
            },
        },
        _ => return Err(invalid()),
    };
    if words[0] == 0 && !matches!(outcome, ScalarOutcome::Returned { .. }) {
        return Err(invalid());
    }
    let mut trace = Vec::new();
    let mut index = 3;
    while index < words.len() {
        let word = words[index];
        index += 1;
        trace.push(match word {
            0x1000_0001..=0x1000_0005 => E::Drop {
                value: match word & 15 {
                    1 => V::String,
                    2 => V::Sequence,
                    3 => V::Enum,
                    4 => V::Shared,
                    _ => V::Weak,
                },
            },
            0x1000_0006 => E::ReleaseImplicitWeak,
            0x1000_0007 => E::ReleaseControl,
            0x2000_0000..=0x2000_ffff => {
                if words.len() - index < 2 || words[index] > 0xffff || words[index + 1] > 1_048_576
                {
                    return Err(invalid());
                }
                let event = E::Cleanup {
                    module: word & 0xffff,
                    function: words[index],
                    place: words[index + 1],
                };
                index += 2;
                event
            }
            _ => return Err(invalid()),
        });
    }
    Ok(crate::OwnershipManifestResult::new(target, outcome).with_trace(trace))
}

pub(super) fn javascript(
    stem: &str,
    invocation: &VerifiedInvocation<'_>,
    fault: Option<Fault>,
) -> Result<Vec<u8>, CommandFailure> {
    let configure = fault.map_or(String::new(), |fault| format!("observe({});", fault.command()));
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
    Ok(format!("import {{ {export} as invoke, $zryna$observation as observe }} from {path};\n{configure}\nlet value = invoke({arguments});\nconst status = observe(0);\n{carrier}\n{FRAME}").into_bytes())
}

pub(super) fn webassembly(
    invocation: &VerifiedInvocation<'_>,
    fault: Option<Fault>,
) -> Result<Vec<u8>, CommandFailure> {
    let configure = fault.map_or(String::new(), |fault| format!("observe({});", fault.command()));
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
    Ok(format!("const chunks = []; for await (const chunk of process.stdin) chunks.push(chunk);\nconst {{instance}} = await WebAssembly.instantiate(Buffer.concat(chunks), {{}});\nconst observe = instance.exports.$zryna$observation;\n{configure}\nconst value = instance.exports[{export}]({arguments});\nconst status = observe(0);\n{FRAME}").into_bytes())
}

const FRAME: &str = "if (!Number.isInteger(status) || status < 0 || status > 5 || typeof value !== 'number' || value !== (value | 0) || Object.is(value, -0)) process.exit(70);\nconst count = observe(0x40000000); if (!Number.isInteger(count) || count < 0 || count > 4097) process.exit(70);\nconst frame = Buffer.alloc(12 + Math.min(count, 4096) * 4); frame.writeUInt32LE(status, 0); frame.writeInt32LE(value, 4); frame.writeUInt32LE(count, 8);\nfor (let i = 0; i < Math.min(count, 4096); ++i) frame.writeUInt32LE(observe(0x40000001 + i) >>> 0, 12 + i * 4); process.stdout.write(frame);\n";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_status_channel_rejects_host_errors_and_partial_frames() {
        let names = ["bounds", "allocation", "capacity", "refcount", "utf8"];
        for (index, name) in names.iter().enumerate() {
            let mut frame = [0; 12];
            frame[0] = u8::try_from(index + 1).expect("fixed test authority");
            assert_eq!(
                serde_json::to_value(
                    decode(&frame, ScalarType::I32, crate::OwnershipTarget::JavaScript)
                        .expect("fixed test authority")
                        .outcome()
                )
                .expect("fixed test authority"),
                serde_json::json!({"kind": "trapped", "code": format!("zryna.trap.{name}-v1")})
            );
            frame[4] = 1;
            assert!(decode(&frame, ScalarType::I32, crate::OwnershipTarget::JavaScript).is_err());
        }
        for status in [6, 255] {
            let mut frame = [0; 12];
            frame[0] = status;
            assert!(decode(&frame, ScalarType::I32, crate::OwnershipTarget::JavaScript).is_err());
        }
    }
    #[test]
    fn trace_frame_exact_boundary_and_hostile_words_are_execution_owned() {
        let mut words = vec![2u32, 0, 4096];
        words.extend(std::iter::repeat_n(0x1000_0001, 4096));
        let bytes =
            |words: &[u32]| words.iter().flat_map(|word| word.to_le_bytes()).collect::<Vec<_>>();
        let target = crate::OwnershipTarget::Native;
        assert_eq!(
            decode(&bytes(&words), ScalarType::I32, target)
                .expect("fixed test authority")
                .trace()
                .len(),
            4096
        );
        let mut bad = Vec::new();
        let mut extra = words.clone();
        extra[2] += 1;
        extra.push(0x1000_0001);
        bad.push(bytes(&extra));
        let mut truncated = words.clone();
        truncated[2] += 1;
        bad.push(bytes(&truncated));
        bad.extend([
            vec![],
            vec![0; 11],
            vec![0; 13],
            bytes(&[2, 0, 1, 0]),
            bytes(&[2, 0, 1, 0x2000_0001]),
            bytes(&[2, 0, 3, 0x2000_0001, 65536, 0]),
            bytes(&[2, 0, 3, 0x2000_0001, 0, 1_048_577]),
            bytes(&[2, 1, 0]),
        ]);
        for frame in bad {
            let failure =
                decode(&frame, ScalarType::I32, target).expect_err("hostile claim must fail");
            assert_eq!(failure.kind(), crate::CommandFailureKind::Execution);
            assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-C3302");
        }
    }
    #[test]
    fn invalid_scalar_carrier_is_rejected_before_publication() {
        let mut frame = [0; 12];
        frame[4] = 2;
        let failure = decode(&frame, ScalarType::Bool, crate::OwnershipTarget::WebAssembly)
            .expect_err("hostile claim must fail");
        assert_eq!(failure.kind(), crate::CommandFailureKind::Execution);
        assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-C3302");
    }
}
