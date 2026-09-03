use zryna_syntax::v4 as syntax;

use super::super::function_catalog::{FunctionResolution, FunctionSignature};
use super::super::span;
use super::super::type_model::Ty;
use super::PrivateVecLowerer;

impl PrivateVecLowerer<'_, '_, '_> {
    pub(super) fn resolve_owned_callee(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        expected: Ty,
    ) -> Option<FunctionSignature> {
        let signature = match self.catalog.resolve(self.module, &callee.text) {
            FunctionResolution::Exact(signature) => signature.clone(),
            FunctionResolution::WrongCase => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("call name '{}' has the wrong portable ASCII case", callee.text),
                    "use the callee's exact declared spelling",
                );
                return None;
            }
            FunctionResolution::Missing => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("function '{}' is not declared in this module", callee.text),
                    "call one exact private same-module Vec function",
                );
                return None;
            }
        };
        let exact_parameters = !signature.has_borrow_parameters()
            && (signature.parameters.is_empty()
                || signature.parameters.as_slice() == [self.vec_ty]);
        if !signature.private
            || signature.result != expected
            || expected != self.vec_ty
            || !exact_parameters
        {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "call signature is outside the sealed Vec producer/identity checkpoint",
                "call a private zero-argument producer or one-exact-Vec identity function",
            );
            return None;
        }
        Some(signature)
    }
}
