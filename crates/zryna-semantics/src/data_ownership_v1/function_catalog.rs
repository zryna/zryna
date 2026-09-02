use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::raw as raw_layout;
use zryna_source::Span;

use super::diagnostics::Errors;
use super::layout_graph::{Decl, semantic_type};
use super::type_model::Ty;
use super::{SemanticInput, span};

#[derive(Clone)]
pub(super) struct FunctionSignature {
    pub(super) id: raw::FunctionId,
    pub(super) name: String,
    pub(super) parameters: Vec<Ty>,
    pub(super) result: Ty,
    pub(super) private: bool,
}

pub(super) struct FunctionCatalog {
    pub(super) modules: Vec<Vec<Option<FunctionSignature>>>,
}

pub(super) enum FunctionResolution<'a> {
    Exact(&'a FunctionSignature),
    WrongCase,
    Missing,
}

impl FunctionCatalog {
    pub(super) fn resolve(&self, module: usize, name: &str) -> FunctionResolution<'_> {
        let Some(signatures) = self.modules.get(module) else {
            return FunctionResolution::Missing;
        };
        if let Some(signature) =
            signatures.iter().flatten().find(|signature| signature.name == name)
        {
            return FunctionResolution::Exact(signature);
        }
        if signatures.iter().flatten().any(|signature| signature.name.eq_ignore_ascii_case(name)) {
            FunctionResolution::WrongCase
        } else {
            FunctionResolution::Missing
        }
    }
}

pub(super) fn build_function_catalog(
    input: SemanticInput<'_>,
    declarations: &[Decl],
    graph: &raw_layout::Graph,
    node_types: &[Option<Ty>],
    errors: &mut Errors<'_>,
) -> FunctionCatalog {
    let mut modules = Vec::with_capacity(input.syntax().files().len());
    for (module, file) in input.syntax().files().iter().enumerate() {
        let mut names = BTreeMap::<String, Span>::new();
        let mut signatures = Vec::with_capacity(file.functions().len());
        for (declaration, function) in file.functions().iter().enumerate() {
            let name_span = span(input.sources(), function.name.span);
            let folded = function.name.text.to_ascii_lowercase();
            if function.name.text.eq_ignore_ascii_case("concat") {
                errors.at(
                    "ZRYNA-M3002",
                    name_span,
                    "function name 'concat' collides with the sealed String builtin",
                    "rename the function so ordinary calls remain unambiguous",
                );
            }
            if names.insert(folded, name_span).is_some() {
                errors.at(
                    "ZRYNA-M3002",
                    name_span,
                    format!(
                        "function '{}' collides under portable ASCII case folding",
                        function.name.text
                    ),
                    "give every module-local function one portable case-insensitive unique name",
                );
            }
            let parameters = function
                .parameters
                .iter()
                .map(|parameter| {
                    semantic_type(
                        file,
                        parameter.type_syntax,
                        module,
                        declarations,
                        graph,
                        node_types,
                        errors,
                    )
                })
                .collect::<Option<Vec<_>>>();
            let result = semantic_type(
                file,
                function.result_type,
                module,
                declarations,
                graph,
                node_types,
                errors,
            );
            signatures.push(parameters.zip(result).map(|(parameters, result)| FunctionSignature {
                id: raw::FunctionId {
                    module: raw::ModuleId(u32::try_from(module).unwrap_or(u32::MAX)),
                    declaration: u32::try_from(declaration).unwrap_or(u32::MAX),
                },
                name: function.name.text.clone(),
                parameters,
                result,
                private: function.export_span.is_none(),
            }));
        }
        modules.push(signatures);
    }
    FunctionCatalog { modules }
}
