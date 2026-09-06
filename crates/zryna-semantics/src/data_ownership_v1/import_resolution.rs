use std::collections::BTreeMap;

use zryna_source::{NormalizedSourcePath, resolve_explicit_zry_import};

use super::diagnostics::{Errors, span};
use super::{FunctionCatalog, SemanticInput};

#[derive(Clone, Copy)]
struct ImportEdge {
    target: usize,
    span: zryna_source::Span,
}

pub(super) fn resolve_imports(
    input: SemanticInput<'_>,
    layouts: &zryna_layout::VerifiedLayouts,
    catalog: &mut FunctionCatalog,
    errors: &mut Errors<'_>,
) {
    let paths = module_paths(input);
    let mut edges = vec![Vec::new(); input.syntax().files().len()];
    for (module, file) in input.syntax().files().iter().enumerate() {
        for import in file.imports() {
            let Ok(path) = resolve_explicit_zry_import(file.path(), &import.specifier.text) else {
                errors.at(
                    "ZRYNA-M3016",
                    span(input.sources(), import.specifier.token_span),
                    "module import does not resolve under the explicit portable .zry grammar",
                    "use one explicit relative lowercase .zry path inside the workspace",
                );
                continue;
            };
            let Some(&target_module) = paths.get(&path) else {
                errors.at(
                    "ZRYNA-M3016",
                    span(input.sources(), import.specifier.token_span),
                    format!("module '{path}' is absent from the authenticated source closure"),
                    "compile the complete driver-authenticated module closure",
                );
                continue;
            };
            edges[module].push(ImportEdge {
                target: target_module,
                span: span(input.sources(), import.specifier.token_span),
            });
            for binding in &import.bindings {
                // Imports bind declarations exported by the named module. Imported aliases are
                // callable there, but are not declarations and cannot be re-exported implicitly.
                let target = catalog.modules[target_module]
                    .iter()
                    .take(input.syntax().files()[target_module].functions().len())
                    .flatten()
                    .find(|signature| signature.name == binding.imported.text);
                let Some(target) = target else {
                    let wrong_case = catalog.modules[target_module]
                        .iter()
                        .take(input.syntax().files()[target_module].functions().len())
                        .flatten()
                        .any(|signature| {
                            signature.name.eq_ignore_ascii_case(&binding.imported.text)
                        });
                    errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), binding.imported.span),
                        if wrong_case {
                            format!(
                                "imported function '{}' has the wrong portable ASCII case",
                                binding.imported.text
                            )
                        } else {
                            format!(
                                "module '{path}' does not export function '{}'",
                                binding.imported.text
                            )
                        },
                        "import one explicitly exported function using its exact declared spelling",
                    );
                    continue;
                };
                let exported = input.syntax().files()[target_module]
                    .functions()
                    .get(target.id.declaration as usize)
                    .is_some_and(|function| function.export_span.is_some());
                if !exported {
                    errors.at(
                        "ZRYNA-M3016",
                        span(input.sources(), binding.imported.span),
                        format!(
                            "module '{path}' does not export function '{}'",
                            binding.imported.text
                        ),
                        "import one explicitly exported function from the resolved module",
                    );
                    continue;
                }
                if !supported_signature(
                    target,
                    layouts,
                    span(input.sources(), binding.imported.span),
                    errors,
                ) {
                    continue;
                }
                let local = &binding.local.text;
                if catalog.callable_names(module).any(|name| name.eq_ignore_ascii_case(local)) {
                    errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), binding.local.span),
                        format!("callable name '{local}' collides under portable ASCII case folding"),
                        "use an exact unique import alias that does not match another import or function",
                    );
                    continue;
                }
                let target = target.clone();
                catalog.bind_import(module, local.clone(), &target);
            }
        }
    }
    verify_graph(input, &edges, errors);
}

fn module_paths(input: SemanticInput<'_>) -> BTreeMap<NormalizedSourcePath, usize> {
    input
        .syntax()
        .files()
        .iter()
        .enumerate()
        .map(|(index, file)| (file.path().clone(), index))
        .collect()
}

fn supported_signature(
    target: &super::function_catalog::FunctionSignature,
    layouts: &zryna_layout::VerifiedLayouts,
    at: zryna_source::Span,
    errors: &mut Errors<'_>,
) -> bool {
    let supported = !target.has_borrow_parameters()
        && target
            .parameters
            .iter()
            .chain(std::iter::once(&target.result))
            .all(|ty| super::owned_aggregate_lowering::supported_mixed_shape(*ty, layouts));
    if !supported {
        errors.at(
            "ZRYNA-M3016",
            at,
            "named-import call signature is outside the sealed by-value ownership graph",
            "import only exact supported by-value signatures without borrow parameters",
        );
    }
    supported
}

#[cfg(test)]
#[path = "tests/imported_owned_signatures.rs"]
mod tests;

fn verify_graph(input: SemanticInput<'_>, edges: &[Vec<ImportEdge>], errors: &mut Errors<'_>) {
    let Some(entry) = input.syntax().files().iter().position(|file| file.id() == input.entry())
    else {
        return;
    };
    let mut state = vec![0_u8; edges.len()];
    let mut reachable = vec![false; edges.len()];
    let mut stack = vec![(entry, 0_usize)];
    state[entry] = 1;
    reachable[entry] = true;
    while let Some((module, next)) = stack.last_mut() {
        if *next == edges[*module].len() {
            state[*module] = 2;
            stack.pop();
            continue;
        }
        let edge = edges[*module][*next];
        *next += 1;
        reachable[edge.target] = true;
        match state[edge.target] {
            0 => {
                state[edge.target] = 1;
                stack.push((edge.target, 0));
            }
            1 => errors.at(
                "ZRYNA-M3016",
                edge.span,
                "the resolved module import graph contains a cycle",
                "remove the cyclic relative import chain",
            ),
            _ => {}
        }
    }
    for (index, reached) in reachable.into_iter().enumerate() {
        if !reached {
            errors.global(
                "ZRYNA-M3016",
                format!(
                    "authenticated source closure contains module '{}' unreachable from the selected entry",
                    input.syntax().files()[index].path()
                ),
                "pass one complete source-map-bound verified snapshot",
            );
        }
    }
}
