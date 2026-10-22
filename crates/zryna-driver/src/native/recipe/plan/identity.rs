//! Internal reproducible admission identities, never a serialized execution grant.
use super::{CompilePlanRequest, NativeRecipeError, denied};
use crate::package_build::BuildPlanIdentity;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

pub(super) fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hex(digest: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in digest {
        value.push(char::from(DIGITS[usize::from(*byte >> 4)]));
        value.push(char::from(DIGITS[usize::from(*byte & 15)]));
    }
    value
}
pub(super) fn canonical(value: &Value) -> Result<Vec<u8>, NativeRecipeError> {
    let mut value = value.clone();
    value.sort_all_objects();
    let mut bytes = serde_json::to_vec(&value)
        .map_err(|_| denied("TRUST-PLAN", "compile identity cannot be encoded"))?;
    bytes.push(b'\n');
    Ok(bytes)
}
pub(super) fn recipe(
    request: &CompilePlanRequest<'_>,
    source_plan: &BuildPlanIdentity,
    sysroot_inventory: &[Value],
) -> Result<String, NativeRecipeError> {
    let mut inputs = vec![json!({"kind":"source","graphRole":"target/runtime","package":{
        "id":request.step.source.package_id,"sourceSha256":request.step.source.source_sha256},"path":request.step.source.path})];
    inputs.extend(
        request.step.sysroots.iter().map(|material| json!({"kind":"sysroot","id":material.id})),
    );
    let library = &request.requirements.libraries()[0];
    let configuration = request.configuration;
    let bindings = json!({"sourcePlanKey":source_plan.cache_key(),"step":{
        "id":request.step.id,"tool":request.step.tool,"target":"native-linux-x86_64","inputs":inputs,
        "invocation":{"adapter":"zryna.native-compile-adapter.v0","mode":"compile-only"},"output":request.step.output},
        "sysroots":sysroot_inventory,"ffiPackage":request.ffi_package_id,
        "ffi":{"identity":"zryna-native-c-interop-v0","version":"0","targetTriple":"x86_64-unknown-linux-gnu",
        "callingConvention":"sysv-amd64-c-v0","carrierModel":"native-c-interop-v0-carriers",
        "ownershipModel":"native-c-interop-v0-resources","decisionIssue":364,"runtime":configuration.targets[0].runtime,
        "declarationSha256":hex(request.requirements.declaration_sha256()),
        "objectSha256":hex(request.requirements.object_sha256()),
        "library":library.id(),"headerSha256":hex(library.header_sha256()),"policySha256":hex(library.policy_sha256())},
        "output":configuration.outputs[0]});
    let mut identity = b"ZRYNA-NATIVE-COMPILE-ADMISSION-V0\0recipe\0".to_vec();
    identity.extend(canonical(&bindings)?);
    Ok(sha(&identity))
}
