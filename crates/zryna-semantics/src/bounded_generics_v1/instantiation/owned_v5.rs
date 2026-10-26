//! Actual original roots validate privately; ownership precedes generated discovery and issuance.
use super::{BodyTypeContext, InstanceContext};
/// Complete discovery issued after original typing, supplied-root validation and ownership.
/// Preparation retains only private temporary state; generated completion and layouts follow ownership.
/// ```compile_fail
/// fn bypass(old: zryna_semantics::bounded_generics_v1::instantiation::InstanceContext<'_, '_, '_>) {
///     let _: zryna_semantics::bounded_generics_v1::instantiation::owned_v5::OwnedV5InstanceContext<'_, '_, '_> = old;
/// }
/// ```
#[derive(Debug)]
pub struct OwnedV5InstanceContext<'b, 'c, 's> {
    instances: InstanceContext<'b, 'c, 's>,
}
impl OwnedV5InstanceContext<'_, '_, '_> {
    /// Exact closed inventory for independently issuing both target layouts.
    #[must_use]
    pub const fn instances(&self) -> &InstanceContext<'_, '_, '_> {
        &self.instances
    }
}
/// Validates actual supplied roots and original definition errors in private temporary state,
/// checks every original's ownership, then completes generated discovery and issues its context.
/// # Errors
/// Rejects source typing, unsupported ownership forms, loans, moves and discovery budgets.
pub fn discover<'b, 'c, 's>(
    bodies: &'b BodyTypeContext<'c, 's>,
) -> Result<OwnedV5InstanceContext<'b, 'c, 's>, zryna_ir::generic_v1::Failure> {
    let prepared = super::prepare_originals(bodies).map_err(instantiation_failure)?;
    let declarations = bodies.declarations();
    zryna_ir::generic_v1::owned_v2::check_original_bodies_v5(
        declarations.syntax(),
        declarations.sources(),
    )
    .map_err(semantic_failure)?;
    prepared
        .finish()
        .map(|instances| OwnedV5InstanceContext { instances })
        .map_err(instantiation_failure)
}
/// Produces untrusted source-owned claims; separate owned decoding and IR sealing remain mandatory.
/// # Errors
/// Rejects source ownership, unsupported forms, foreign layouts and checked amplification.
pub fn produce_claim(
    owned: &OwnedV5InstanceContext<'_, '_, '_>,
    linear: &zryna_layout::generic_v1::VerifiedLayouts,
    linux: &zryna_layout::generic_v1::VerifiedLayouts,
) -> Result<zryna_ir::generic_v1::owned_v2::raw::Program, zryna_ir::generic_v1::Failure> {
    let instances = owned.instances();
    if instances.type_keys().len() != linear.types().len()
        || instances.type_keys().zip(linear.types()).any(|(a, b)| a != b.key())
    {
        return Err(zryna_ir::generic_v1::Failure::InternalFailure);
    }
    let keys = instances.function_keys().collect::<Vec<_>>();
    let original = instances.bodies.declarations();
    zryna_ir::generic_v1::owned_v2::produce_claim_v5(
        original.syntax(),
        original.sources(),
        linear,
        linux,
        &keys,
    )
    .map_err(semantic_failure)
}
fn instantiation_failure(failure: super::InstantiationFailure) -> zryna_ir::generic_v1::Failure {
    match failure {
        super::InstantiationFailure::Diagnostics(errors) => {
            zryna_ir::generic_v1::Failure::Diagnostics(errors)
        }
        super::InstantiationFailure::AllocationFailure => {
            zryna_ir::generic_v1::Failure::AllocationFailure
        }
        super::InstantiationFailure::InternalFailure => {
            zryna_ir::generic_v1::Failure::InternalFailure
        }
    }
}

fn semantic_failure(failure: zryna_ir::generic_v1::Failure) -> zryna_ir::generic_v1::Failure {
    match failure {
        zryna_ir::generic_v1::Failure::Diagnostics(mut errors) => {
            for e in &mut errors {
                if e.code == "ZRYNA-I7001" {
                    "ZRYNA-M3008".clone_into(&mut e.code);
                } else if e.code == "ZRYNA-I3201" {
                    "ZRYNA-M7201".clone_into(&mut e.code);
                }
            }
            zryna_ir::generic_v1::Failure::Diagnostics(errors)
        }
        other => other,
    }
}
