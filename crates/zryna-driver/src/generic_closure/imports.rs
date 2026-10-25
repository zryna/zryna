//! V5 imports remain exact original source syntax; discovery-map IDs are never final IDs.
use super::{
    BTreeMap, DiscoveredSource, Import, ImportBinding, ModuleClosureError, ModuleEdge,
    NormalizedSourcePath, SourceMap, UntrustedSpan, support, syntax_v5,
};
use support::{invariant, target};
pub(super) fn collect(
    syntax: &syntax_v5::VerifiedProjectSyntaxV5,
) -> Result<BTreeMap<NormalizedSourcePath, Vec<Import>>, ModuleClosureError> {
    syntax
        .files()
        .iter()
        .map(|file| {
            let path = NormalizedSourcePath::new(file.path.clone()).map_err(|_| invariant())?;
            let imports = file
                .imports
                .iter()
                .map(|import| Import {
                    span: import.span,
                    specifier: import.specifier.text.clone(),
                    specifier_span: import.specifier.token_span,
                    bindings: import
                        .bindings
                        .iter()
                        .map(|binding| ImportBinding {
                            imported: binding.imported.text.clone(),
                            local: binding.local.text.clone(),
                            imported_span: binding.imported.span,
                            local_span: binding.local.span,
                        })
                        .collect(),
                })
                .collect();
            Ok((path, imports))
        })
        .collect()
}
pub(super) fn matches(
    final_imports: &BTreeMap<NormalizedSourcePath, Vec<Import>>,
    files: &BTreeMap<NormalizedSourcePath, DiscoveredSource>,
) -> bool {
    final_imports.len() == files.len()
        && files.iter().all(|(path, file)| {
            final_imports.get(path).is_some_and(|rows| {
                rows.len() == file.imports.len()
                    && rows.iter().zip(&file.imports).all(|(a, b)| same(a, b))
            })
        })
}
fn offsets(a: UntrustedSpan, b: UntrustedSpan) -> bool {
    a.start == b.start && a.end == b.end
}
fn same(a: &Import, b: &Import) -> bool {
    offsets(a.span, b.span)
        && a.specifier == b.specifier
        && offsets(a.specifier_span, b.specifier_span)
        && a.bindings.len() == b.bindings.len()
        && a.bindings.iter().zip(&b.bindings).all(|(a, b)| {
            a.imported == b.imported
                && a.local == b.local
                && offsets(a.imported_span, b.imported_span)
                && offsets(a.local_span, b.local_span)
        })
}
pub(super) fn final_edges(
    syntax: &syntax_v5::VerifiedProjectSyntaxV5,
    sources: &SourceMap,
) -> Result<Vec<ModuleEdge>, ModuleClosureError> {
    let mut edges = Vec::new();
    for (path, imports) in collect(syntax)? {
        for import in imports {
            let target = target(&path, &import.specifier)?;
            for binding in import.bindings {
                edges
                    .try_reserve(1)
                    .map_err(|_| support::budget("generic edge storage exhausted"))?;
                edges.push(ModuleEdge {
                    importer: path.clone(),
                    target: target.clone(),
                    specifier: import.specifier.clone(),
                    imported: binding.imported,
                    local: binding.local,
                    declaration_span: sources.verify_span(import.span).map_err(|_| invariant())?,
                    specifier_span: sources
                        .verify_span(import.specifier_span)
                        .map_err(|_| invariant())?,
                    imported_span: sources
                        .verify_span(binding.imported_span)
                        .map_err(|_| invariant())?,
                    local_span: sources.verify_span(binding.local_span).map_err(|_| invariant())?,
                });
            }
        }
    }
    edges.sort_by(|a, b| {
        (&a.importer, &a.specifier, &a.imported, &a.local).cmp(&(
            &b.importer,
            &b.specifier,
            &b.imported,
            &b.local,
        ))
    });
    Ok(edges)
}
