use super::{
    Operation, PlaygroundDaemonOperatorApproval, Result, approval::Approved, config, reject,
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

/// Computes a composition identity without authenticating or approving its origin.
///
/// # Errors
/// Rejects malformed or noncanonical default-deny seccomp documents.
pub fn playground_daemon_policy_sha256(
    record: &PlaygroundDaemonOperatorApproval,
) -> Result<String> {
    let approved = Approved {
        record: record.clone(),
        compiler_seccomp: seccomp(&record.compiler_seccomp_json)?,
        evaluator_seccomp: seccomp(&record.evaluator_seccomp_json)?,
    };
    identity(&approved)
}

pub(super) fn identity(approval: &Approved) -> Result<String> {
    let compiler =
        config::container(Operation::Compile, false, approval, "", "", ["input", "output"]);
    let evaluator =
        config::container(Operation::Evaluate, false, approval, "", "", ["input", "output"]);
    let holder = config::container(Operation::Compile, true, approval, "", "", ["input", "output"]);
    let mut policy = json!({"id":"zryna.playground-local-daemon","version":2,
        "compositionKind":approval.record.composition_kind,
        "compiler":compiler,"evaluator":evaluator,"holder":holder,
        "volumes":[config::volume(false),config::volume(true)],
        "compilerSeconds":10,"evaluationSeconds":5,"cleanupSeconds":2,
        "stagingOverhead":{"memoryBytes":16_777_216,"pids":2,"cpuQuota":10_000},
        "sourceBytes":4096,"requestBytes":32768,"frameBytes":1_277_956,
        "stderrBytes":4096,"inputArchiveBytes":2_162_688,"outputArchiveBytes":2_162_688,
        "compilerRootSha256":approval.record.compiler_root_sha256,
        "evaluatorRootSha256":approval.record.evaluator_root_sha256,
        "compilerExecutableSha256":approval.record.compiler_executable_sha256,
        "bindingTemplateSha256":approval.record.binding_template_sha256,
        "witClosureSha256":approval.record.wit_closure_sha256,
        "compositionSha256":approval.record.composition_sha256});
    policy["compilerMaterials"] = json!(approval.record.compiler_materials);
    policy["evaluatorMaterials"] = json!(approval.record.evaluator_materials);
    policy.sort_all_objects();
    let mut bytes = serde_json::to_vec(&policy)
        .map_err(|_| reject("PLAYGROUND-HOST-POLICY", "fixed policy cannot be encoded"))?;
    bytes.push(b'\n');
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(super) fn seccomp(text: &str) -> Result<Value> {
    if text.is_empty() || text.len() > 65_536 {
        return Err(reject("PLAYGROUND-HOST-POLICY", "finite seccomp policy required"));
    }
    let value: Value = serde_json::from_str(text)
        .map_err(|_| reject("PLAYGROUND-HOST-POLICY", "invalid seccomp policy"))?;
    if serde_json::to_vec(&value).ok().as_deref() != Some(text.as_bytes())
        || value["defaultAction"] != "SCMP_ACT_ERRNO"
        || value["defaultErrnoRet"] != 1
        || value["architectures"] != json!(["SCMP_ARCH_X86_64"])
        || value.as_object().is_none_or(|object| {
            object.len() != 4
                || object.keys().any(|key| {
                    !["defaultAction", "defaultErrnoRet", "architectures", "syscalls"]
                        .contains(&key.as_str())
                })
        })
    {
        return Err(reject(
            "PLAYGROUND-HOST-POLICY",
            "canonical default-deny x86-64 policy required",
        ));
    }
    let rules = value["syscalls"]
        .as_array()
        .filter(|rules| !rules.is_empty() && rules.len() <= 256)
        .ok_or_else(|| reject("PLAYGROUND-HOST-POLICY", "finite seccomp rules required"))?;
    for rule in rules {
        let object = rule
            .as_object()
            .ok_or_else(|| reject("PLAYGROUND-HOST-POLICY", "closed seccomp rule required"))?;
        if object.keys().any(|key| !["names", "action", "args", "errnoRet"].contains(&key.as_str()))
            || !matches!(rule["action"].as_str(), Some("SCMP_ACT_ALLOW" | "SCMP_ACT_ERRNO"))
            || rule.get("errnoRet").is_some_and(|value| {
                value.as_u64().is_none_or(|value| !(1..=4095).contains(&value))
            })
            || rule["action"] == "SCMP_ACT_ALLOW" && rule.get("errnoRet").is_some()
        {
            return Err(reject("PLAYGROUND-HOST-POLICY", "unsupported seccomp rule authority"));
        }
        let names = rule["names"]
            .as_array()
            .filter(|names| !names.is_empty() && names.len() <= 256)
            .ok_or_else(|| reject("PLAYGROUND-HOST-POLICY", "finite syscall names required"))?;
        for name in names {
            let name = name
                .as_str()
                .filter(|name| {
                    !name.is_empty()
                        && name.len() <= 64
                        && name.bytes().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                        })
                })
                .ok_or_else(|| reject("PLAYGROUND-HOST-POLICY", "invalid syscall name"))?;
            prohibit_kernel_authority(rule, name)?;
        }
        if let Some(args) = rule.get("args") {
            let args = args.as_array().filter(|args| args.len() <= 6).ok_or_else(|| {
                reject("PLAYGROUND-HOST-POLICY", "finite syscall filters required")
            })?;
            for arg in args {
                let object = arg.as_object().ok_or_else(|| {
                    reject("PLAYGROUND-HOST-POLICY", "closed syscall filter required")
                })?;
                if object
                    .keys()
                    .any(|key| !["index", "op", "value", "valueTwo"].contains(&key.as_str()))
                    || arg["index"].as_u64().is_none_or(|index| index > 5)
                    || !matches!(arg["op"].as_str(), Some("SCMP_CMP_EQ" | "SCMP_CMP_MASKED_EQ"))
                    || arg["value"].as_u64().is_none()
                    || arg.get("valueTwo").is_some_and(|value| value.as_u64().is_none())
                {
                    return Err(reject("PLAYGROUND-HOST-POLICY", "unsupported syscall filter"));
                }
            }
        }
    }
    Ok(value)
}

fn prohibit_kernel_authority(rule: &Value, name: &str) -> Result<()> {
    if rule["action"] == "SCMP_ACT_ALLOW"
        && [
            "ptrace",
            "process_vm_readv",
            "process_vm_writev",
            "bpf",
            "keyctl",
            "add_key",
            "request_key",
            "init_module",
            "finit_module",
            "delete_module",
            "kexec_load",
            "kexec_file_load",
            "reboot",
            "swapon",
            "swapoff",
            "open_by_handle_at",
            "name_to_handle_at",
        ]
        .contains(&name)
    {
        return Err(reject("PLAYGROUND-HOST-POLICY", "prohibited kernel authority"));
    }
    Ok(())
}
