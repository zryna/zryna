use zryna_syntax::v4 as syntax;

use super::function_catalog::{FunctionCatalog, FunctionResolution, FunctionSignature};
use super::{Errors, SemanticInput, Ty, span};

pub(super) struct OwnedCallResolution<'s, 'a, 'e> {
    pub(super) input: SemanticInput<'a>,
    pub(super) module: usize,
    pub(super) catalog: &'s FunctionCatalog,
    pub(super) errors: &'e mut Errors<'a>,
}

impl OwnedCallResolution<'_, '_, '_> {
    pub(super) fn string(
        &mut self,
        ty: Ty,
        callee: &syntax::RawIdentifierSyntax,
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
                    "call one exact private same-module function",
                );
                return None;
            }
        };
        if !signature.private {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "owned calls require one private same-module callee",
                "keep String producers and identity functions internal",
            );
            return None;
        }
        if signature.result != ty
            || signature.has_borrow_parameters()
            || signature.parameters.len() > 1
            || signature.parameters.iter().any(|parameter| *parameter != ty)
        {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "owned call signature is outside the sealed String producer/identity checkpoint",
                "call a private zero-argument String producer or one-String identity function",
            );
            return None;
        }
        Some(signature)
    }

    pub(super) fn vec(
        &mut self,
        vec_ty: Ty,
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
            && (signature.parameters.is_empty() || signature.parameters.as_slice() == [vec_ty]);
        if !signature.private
            || signature.result != expected
            || expected != vec_ty
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
