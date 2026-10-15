//! Independent operator expectations. A matched approval is still not host enforcement.
use super::plan::{NativeCompilePlan, NativeRecipeError, denied};
use crate::package_build::{BuildHostToolIdentity, ExecutionPolicyIdentity};
use zryna_package::{NativeRecipeDisposition, SourceTrustMode, native_recipe_disposition};

/// Supplied only by the trusted caller/operator channel, never derived from package content.
/// These are approval data; constructing this record alone cannot start a recipe.
#[derive(Clone, Debug)]
pub struct OperatorApproval {
    /// Exact source-plan identity independently approved by the operator.
    pub source_plan_key: String,
    /// Exact internal compile recipe identity independently approved by the operator.
    pub recipe_sha256: String,
    /// Independently approved complete tool-root inventory digest.
    pub tool_root_sha256: String,
    /// Raw64-character Docker image digest, without an ambient tag or transport endpoint.
    pub image_sha256: String,
    /// Exact independently approved execution policy.
    pub policy: ExecutionPolicyIdentity,
    /// Complete independently approved compiler tool inventory.
    pub approved_tools: Vec<BuildHostToolIdentity>,
    /// Exact declared output path.
    pub output_path: String,
    /// Independently expected object size, not a value learned from fresh recipe output.
    pub output_bytes: usize,
    /// Independently expected object digest, not a self-reported recipe checksum.
    pub output_sha256: [u8; 32],
    /// Complete sorted independent inventory of permitted undefined symbols.
    pub dependency_symbols: Vec<String>,
}
/// Only admit constructs this result. A genuine later session must still enforce all controls.
#[derive(Debug)]
pub struct ApprovedNativeRecipe {
    plan: NativeCompilePlan,
    operator: OperatorApproval,
    mode: SourceTrustMode,
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn symbol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && (value.as_bytes()[0].is_ascii_alphabetic() || value.as_bytes()[0] == b'_')
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
/// Rejects caller mode before source/plan checks; performs no host operation.
pub(super) fn prepare_and_admit(
    request: super::plan::CompilePlanRequest<'_>,
    mode: SourceTrustMode,
    operator: OperatorApproval,
) -> Result<ApprovedNativeRecipe, NativeRecipeError> {
    if native_recipe_disposition(mode) != NativeRecipeDisposition::RequiresApprovalAndIsolation {
        return Err(denied("TRUST-OPERATION", "native recipes are forbidden in the selected mode"));
    }
    admit(super::plan::prepare(request)?, mode, operator)
}

/// Admission performs no I/O, process start, cache use, publication or native host grant.
pub(super) fn admit(
    plan: NativeCompilePlan,
    mode: SourceTrustMode,
    operator: OperatorApproval,
) -> Result<ApprovedNativeRecipe, NativeRecipeError> {
    if native_recipe_disposition(mode) != NativeRecipeDisposition::RequiresApprovalAndIsolation {
        return Err(denied("TRUST-OPERATION", "native recipes are forbidden in the selected mode"));
    }
    if operator.source_plan_key != plan.source_plan().cache_key()
        || operator.recipe_sha256 != plan.recipe_sha256()
        || !digest(&operator.image_sha256)
        || !digest(&operator.tool_root_sha256)
        || operator.policy != plan.configuration().execution_policy
        || operator.approved_tools != plan.configuration().host_tools
        || operator.output_path != plan.output_path()
    {
        return Err(denied(
            "TRUST-NATIVE",
            "exact independent recipe, tool, policy and output approval is required",
        ));
    }
    if operator.output_bytes == 0
        || operator.output_bytes > zryna_backend_native::MAX_NATIVE_OBJECT_BYTES
        || operator.dependency_symbols.len() > 64
        || operator.dependency_symbols.iter().any(|name| !symbol(name))
        || operator.dependency_symbols.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(denied(
            "TRUST-NATIVE",
            "approved object or dependency inventory is outside its bounds",
        ));
    }
    Ok(ApprovedNativeRecipe { plan, operator, mode })
}
impl ApprovedNativeRecipe {
    /// Retained immutable source/ABI plan, without execution success.
    #[must_use]
    pub fn plan(&self) -> &NativeCompilePlan {
        &self.plan
    }
    /// Exact independent operator expectations checked during admission.
    #[must_use]
    pub fn operator(&self) -> &OperatorApproval {
        &self.operator
    }
    /// Caller-selected mode accepted for this admission.
    #[must_use]
    pub fn mode(&self) -> SourceTrustMode {
        self.mode
    }
}

/// Audits data against independent output expectations; it issues no host/session success.
#[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
pub(super) fn capture_output(
    approved: &ApprovedNativeRecipe,
    bytes: &[u8],
) -> Result<crate::native::native_c_v0::foreign_library::CapturedForeignLibrary, NativeRecipeError>
{
    use crate::native::native_c_v0::foreign_library::{
        ForeignLibraryInput, capture_foreign_library,
    };
    let requirements = approved.plan().requirements();
    let library = requirements
        .libraries()
        .first()
        .ok_or_else(|| denied("TRUST-NATIVE", "retained library is absent"))?;
    let authority = requirements
        .object()
        .program()
        .source()
        .private_authority()
        .body_authority()
        .declaration_authority();
    let header_bytes = authority
        .header_bytes(library.id())
        .ok_or_else(|| denied("TRUST-NATIVE", "retained library header is absent"))?;
    let policy_bytes = authority
        .policy_bytes(library.id())
        .ok_or_else(|| denied("TRUST-NATIVE", "retained library policy is absent"))?;
    let operator = approved.operator();
    let dependencies = operator.dependency_symbols.iter().map(String::as_str).collect::<Vec<_>>();
    capture_foreign_library(
        requirements,
        &ForeignLibraryInput {
            library_id: library.id(),
            header_bytes,
            policy_bytes,
            object_bytes: bytes,
            object_size: operator.output_bytes,
            object_sha256: &operator.output_sha256,
            dependency_symbols: &dependencies,
        },
    )
    .map_err(NativeRecipeError::Object)
}
