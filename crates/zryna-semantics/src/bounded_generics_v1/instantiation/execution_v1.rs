//! Original-before-generated bridge into the distinct complete execution domain.
use super::{BodyTypeContext, InstanceContext, InstantiationFailure};
use zryna_ir::generic_v1::{Failure, execution_v1 as execution};
use zryna_layout::{StorageTarget, generic_v1::VerifiedLayouts};

/// Exact canonical instances published only after every original execution check.
///
/// This retains semantic demand, not a layout, executable program or backend authority.
/// ```compile_fail
/// fn convert(old: zryna_semantics::bounded_generics_v1::instantiation::InstanceContext<'_, '_, '_>) {
///     let _: zryna_semantics::bounded_generics_v1::instantiation::execution_v1::ExecutionInstances<'_, '_, '_> = old;
/// }
/// ```
#[derive(Debug)]
pub struct ExecutionInstances<'b, 'c, 's> {
    instances: InstanceContext<'b, 'c, 's>,
}
impl ExecutionInstances<'_, '_, '_> {
    /// The original checked closed demand; no new discovery identity is substituted.
    #[must_use]
    pub const fn instances(&self) -> &InstanceContext<'_, '_, '_> {
        &self.instances
    }
}

/// Checks actual original roots and every original body before generated completion.
///
/// # Errors
/// Preserves owning source diagnostics and infrastructure failures without a partial context.
pub fn discover<'b, 'c, 's>(
    bodies: &'b BodyTypeContext<'c, 's>,
) -> Result<ExecutionInstances<'b, 'c, 's>, Failure> {
    let prepared =
        super::preparation::prepare_execution_originals(bodies).map_err(instantiation_failure)?;
    let original = bodies.declarations();
    // The source owner supplies semantic codes/spans for source rejections. Malformed
    // claim/resource diagnostics retain their own identity; no blanket code remap occurs.
    execution::check_original_bodies(original.syntax(), original.sources())?;
    let instances = prepared.finish().map_err(instantiation_failure)?;
    Ok(ExecutionInstances { instances })
}

/// Produces untrusted complete claims from this exact original demand and both layouts.
///
/// Wire decoding and the independent execution issuer are still required afterward.
/// # Errors
/// Rejects foreign/incorrect layouts, source failures and fallible temporary allocation.
pub fn produce_claim(
    execution: &ExecutionInstances<'_, '_, '_>,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
) -> Result<execution::raw::Program, Failure> {
    let instances = execution.instances();
    check_layouts(instances, linear, linux)?;
    let count = instances.function_keys().len();
    let mut keys = Vec::new();
    keys.try_reserve_exact(count).map_err(|_| Failure::AllocationFailure)?;
    keys.extend(instances.function_keys());
    let original = instances.bodies().declarations();
    execution::produce_claim(
        original.syntax(),
        original.sources(),
        original.entry(),
        linear,
        linux,
        &keys,
    )
}

fn check_layouts(
    instances: &InstanceContext<'_, '_, '_>,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
) -> Result<(), Failure> {
    let source = instances.bodies().declarations().sources().identity();
    if linear.target() != StorageTarget::Linear32V1
        || linux.target() != StorageTarget::LinuxX8664V1
        || linear.source_map_identity() != source
        || linux.source_map_identity() != source
        || linear.universe_identity() != linux.universe_identity()
        || linear.original_modules() != linux.original_modules()
        || linear.original_declarations() != linux.original_declarations()
    {
        return Err(Failure::InternalFailure);
    }
    for layouts in [linear, linux] {
        if layouts.types().len() != instances.type_keys().len()
            || layouts.types().zip(instances.type_keys()).any(|(ty, key)| ty.key() != key)
        {
            return Err(Failure::InternalFailure);
        }
    }
    Ok(())
}

fn instantiation_failure(failure: InstantiationFailure) -> Failure {
    match failure {
        InstantiationFailure::Diagnostics(errors) => Failure::Diagnostics(errors),
        InstantiationFailure::AllocationFailure => Failure::AllocationFailure,
        InstantiationFailure::InternalFailure => Failure::InternalFailure,
    }
}
