use zryna_syntax::v4 as syntax;

use super::super::function_catalog::FunctionSignature;
use super::super::type_model::Ty;
use super::PrivateVecLowerer;

impl PrivateVecLowerer<'_, '_, '_> {
    pub(super) fn resolve_owned_callee(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        expected: Ty,
    ) -> Option<FunctionSignature> {
        super::super::owned_call_resolution::OwnedCallResolution {
            input: self.input,
            module: self.module,
            catalog: self.catalog,
            errors: self.errors,
        }
        .vec(self.vec_ty, callee, expected)
    }
}
