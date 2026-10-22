//! Authenticated compile-only native recipes; host enforcement owns execution success.

mod api;
mod approval;
#[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
mod archive;
mod plan;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) mod engine;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod session;

#[cfg(test)]
mod tests;

pub use api::{
    ApprovedNativeRecipe, NativeCompilePlan, NativeCompileRecipeRequest, NativeCompileRecipeStep,
    NativeCompileSourceInput, NativeRecipeError, NativeRecipeOperatorApproval,
    NativeRecipeReadOnlyMaterial, NativeRecipeRequirements, approve_native_recipe,
    prepare_and_approve_native_recipe, prepare_native_recipe,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use archive::{
    Metadata as NativeToolRootMetadata, RootEntry as NativeToolRootEntry,
    RootKind as NativeToolRootKind,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use session::{
    NativeRecipeSuccess, NativeToolRootApproval, execute_native_recipe, native_recipe_environment,
    native_recipe_policy_identity, native_tool_root_identity,
};
