//! Explicit library admission surface. No CLI selector or native execution grant is added.
pub use super::approval::{ApprovedNativeRecipe, OperatorApproval as NativeRecipeOperatorApproval};
pub use super::plan::{
    CompilePlanRequest as NativeCompileRecipeRequest, NativeCompilePlan,
    NativeCompileStep as NativeCompileRecipeStep, NativeRecipeError,
    ReadOnlyMaterial as NativeRecipeReadOnlyMaterial, SourceInput as NativeCompileSourceInput,
};
/// Original opaque ABI/link issuer, constructed only through the existing verified pipeline.
pub type NativeRecipeRequirements =
    crate::native::native_c_v0::resource_identity::HandleLinkRequirements;

/// Retains one exact authenticated source/ABI compile plan without host effects.
///
/// This does not start a compiler, open a cache, publish an artifact or approve native execution.
/// # Errors
/// Rejects invalid source-plan, source-map, ABI issuer, input inventory or compile-step identities.
pub fn prepare_native_recipe(
    request: NativeCompileRecipeRequest<'_>,
) -> Result<NativeCompilePlan, NativeRecipeError> {
    super::plan::prepare(request)
}

/// Matches independently supplied operator approval without starting a host session.
///
/// The caller must obtain expectations through its trusted operator channel. Package data and a
/// fresh recipe's self-reported digests cannot authenticate that channel. The returned opaque
/// value still requires actual host enforcement, complete object audit and confirmed cleanup.
/// # Errors
/// Rejects forbidden source modes and mismatched or out-of-bounds independent expectations.
pub fn approve_native_recipe(
    plan: NativeCompilePlan,
    mode: zryna_package::SourceTrustMode,
    operator: NativeRecipeOperatorApproval,
) -> Result<ApprovedNativeRecipe, NativeRecipeError> {
    super::approval::admit(plan, mode, operator)
}

/// Prepares and admits a recipe, rejecting forbidden source modes before plan work.
/// # Errors
/// Rejects forbidden trust modes and any invalid or mismatched independent plan approval.
pub fn prepare_and_approve_native_recipe(
    request: NativeCompileRecipeRequest<'_>,
    mode: zryna_package::SourceTrustMode,
    operator: NativeRecipeOperatorApproval,
) -> Result<ApprovedNativeRecipe, NativeRecipeError> {
    super::approval::prepare_and_admit(request, mode, operator)
}
