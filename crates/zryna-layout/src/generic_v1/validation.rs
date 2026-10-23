//! Source binding, shape, complete inventory and inherited resource preflight.

use super::{Failure, raw, reject, reserve};
use crate::raw::TypeKind as Base;
use zryna_source::SourceMap;

pub(super) fn preflight(graph: &raw::Graph, sources: &SourceMap) -> Result<(), Failure> {
    if graph.types.len() > crate::MAX_TYPE_NODES {
        return Err(reject(None, "ZRYNA-L7201", "closed type count exceeds 65536"));
    }
    if graph.declarations.len() > crate::MAX_TYPE_NODES {
        return Err(reject(None, "ZRYNA-L7201", "original declaration count exceeds 65536"));
    }
    original_inventory(graph, sources)?;
    let mut primitives = [0usize; 3];
    let mut members = 0usize;
    let mut edges = 0usize;
    let mut represented = reserve(graph.declarations.len())?;
    represented.resize(graph.declarations.len(), false);
    for (index, node) in graph.types.iter().enumerate() {
        if usize::try_from(node.id.0).ok() != Some(index) {
            return Err(reject(None, "ZRYNA-L3001", "closed graph node IDs are not dense"));
        }
        if node.span.is_some_and(|span| sources.resolve(span).is_err()) {
            return Err(reject(
                None,
                "ZRYNA-L3001",
                "closed nominal span belongs to another source authority",
            ));
        }
        shape(graph, node)?;
        if let raw::TypeKind::Base(kind) = &node.kind {
            match kind {
                Base::Bool => primitives[0] += 1,
                Base::I32 => primitives[1] += 1,
                Base::String => primitives[2] += 1,
                _ => {}
            }
        }
        let (local_members, local_edges) = counts(&node.kind);
        if local_members > crate::MAX_MEMBERS_PER_DECLARATION {
            return Err(reject(node.span, "ZRYNA-L7201", "declaration member count exceeds 1024"));
        }
        members = crate::checked_budget_total(members, local_members, crate::MAX_MEMBERS)
            .ok_or_else(|| {
                reject(node.span, "ZRYNA-L7201", "closed graph member count exceeds 65536")
            })?;
        edges = crate::checked_budget_total(edges, local_edges, crate::MAX_DEPENDENCY_EDGES)
            .ok_or_else(|| {
                reject(node.span, "ZRYNA-L7201", "layout dependency count exceeds 262144")
            })?;
        if let Some((module, index, _, _)) = nominal(&node.kind) {
            let declaration = declaration_index(graph, module, index)?;
            represented[declaration] = true;
        }
    }
    if primitives != [1, 1, 1] {
        return Err(reject(None, "ZRYNA-L3001", "exactly one bool, i32 and String is required"));
    }
    for (declaration, represented) in graph.declarations.iter().zip(represented) {
        if declaration.parameters == 0 && !represented {
            return Err(reject(
                Some(declaration.span),
                "ZRYNA-L3001",
                "nongeneric original declaration has no closed stored node",
            ));
        }
    }
    reachability(graph)
}

mod shape;
use shape::shape;

pub(super) fn nominal(kind: &raw::TypeKind) -> Option<(raw::ModuleId, u32, u32, raw::NominalKind)> {
    match kind {
        raw::TypeKind::Struct { module, declaration, arguments, .. } => Some((
            *module,
            *declaration,
            u32::try_from(arguments.len()).unwrap_or(u32::MAX),
            raw::NominalKind::Struct,
        )),
        raw::TypeKind::Enum { module, declaration, arguments, .. } => Some((
            *module,
            *declaration,
            u32::try_from(arguments.len()).unwrap_or(u32::MAX),
            raw::NominalKind::Enum,
        )),
        raw::TypeKind::Base(Base::Struct { module, declaration, .. }) => {
            Some((*module, *declaration, 0, raw::NominalKind::Struct))
        }
        raw::TypeKind::Base(Base::Enum { module, declaration, .. }) => {
            Some((*module, *declaration, 0, raw::NominalKind::Enum))
        }
        _ => None,
    }
}

fn counts(kind: &raw::TypeKind) -> (usize, usize) {
    match kind {
        raw::TypeKind::Base(kind) => crate::member_and_edge_count(kind),
        raw::TypeKind::Struct { arguments, fields, .. } => {
            (fields.len(), arguments.len() + fields.len())
        }
        raw::TypeKind::Enum { arguments, variants, .. } => (
            variants.len(),
            arguments.len() + variants.iter().filter(|v| v.payload.is_some()).count(),
        ),
        raw::TypeKind::Option { .. } => (2, 2),
        raw::TypeKind::Result { .. } => (2, 4),
    }
}

fn reachability(graph: &raw::Graph) -> Result<(), Failure> {
    let mut seen = reserve(graph.types.len())?;
    seen.resize(graph.types.len(), false);
    let mut pending = reserve(graph.types.len())?;
    for root in &graph.program_roots {
        let id = usize::try_from(root.0).map_err(|_| Failure::InternalFailure)?;
        if id >= graph.types.len() {
            return Err(reject(None, "ZRYNA-L3001", "unknown program root"));
        }
        if !seen[id] {
            seen[id] = true;
            pending.push(id);
        }
    }
    for (id, node) in graph.types.iter().enumerate() {
        if (nominal(&node.kind).is_some()
            || matches!(node.kind, raw::TypeKind::Base(Base::Bool | Base::I32 | Base::String)))
            && !seen[id]
        {
            seen[id] = true;
            pending.push(id);
        }
    }
    while let Some(id) = pending.pop() {
        for child in all_children(&graph.types[id].kind) {
            if !seen[child] {
                seen[child] = true;
                pending.push(child);
            }
        }
    }
    if seen.iter().any(|seen| !seen) {
        return Err(reject(None, "ZRYNA-L3001", "orphan closed type was claimed"));
    }
    Ok(())
}

fn all_children(kind: &raw::TypeKind) -> Vec<usize> {
    let physical = physical_kind(kind);
    let mut children =
        crate::all_children(&physical).iter().map(|id| id.0 as usize).collect::<Vec<_>>();
    if !matches!(kind, raw::TypeKind::Base(_)) {
        children.extend(super::keys::children(kind));
    }
    children
}

pub(super) fn physical(graph: &raw::Graph) -> Result<crate::raw::Graph, Failure> {
    let mut types = reserve(graph.types.len())?;
    for node in &graph.types {
        types.push(crate::raw::TypeNode {
            id: node.id,
            span: node.span,
            kind: physical_kind(&node.kind),
        });
    }
    Ok(crate::raw::Graph {
        modules: graph.modules.clone(),
        types,
        program_roots: graph.program_roots.clone(),
    })
}

fn physical_kind(kind: &raw::TypeKind) -> Base {
    match kind {
        raw::TypeKind::Base(kind) => kind.clone(),
        raw::TypeKind::Struct { module, declaration, fields, .. } => {
            Base::Struct { module: *module, declaration: *declaration, fields: fields.clone() }
        }
        raw::TypeKind::Enum { module, declaration, variants, .. } => {
            Base::Enum { module: *module, declaration: *declaration, variants: variants.clone() }
        }
        raw::TypeKind::Option { argument } => Base::Enum {
            module: raw::ModuleId(0),
            declaration: 0,
            variants: vec![
                raw::Variant { ordinal: 0, payload: None },
                raw::Variant { ordinal: 1, payload: Some(*argument) },
            ],
        },
        raw::TypeKind::Result { okay, error } => Base::Enum {
            module: raw::ModuleId(0),
            declaration: 0,
            variants: vec![
                raw::Variant { ordinal: 0, payload: Some(*okay) },
                raw::Variant { ordinal: 1, payload: Some(*error) },
            ],
        },
    }
}

fn declaration_index(
    graph: &raw::Graph,
    module: raw::ModuleId,
    index: u32,
) -> Result<usize, Failure> {
    graph
        .declarations
        .binary_search_by_key(&(module.0, index), |declaration| {
            (declaration.module.0, declaration.index)
        })
        .map_err(|_| reject(None, "ZRYNA-L3001", "unknown original nominal declaration"))
}

fn original_inventory(graph: &raw::Graph, sources: &SourceMap) -> Result<(), Failure> {
    if graph.modules.len() != sources.len() {
        return Err(reject(
            None,
            "ZRYNA-L3001",
            "module inventory differs from original source map",
        ));
    }
    let mut count = 0usize;
    for (id, module) in graph.modules.iter().enumerate() {
        let id = u32::try_from(id).map_err(|_| Failure::InternalFailure)?;
        if module.id.0 != id || sources.verify_file_id(id).ok() != Some(module.source_file) {
            return Err(reject(None, "ZRYNA-L3001", "module identity is nondense or foreign"));
        }
        for index in 0..module.data_declarations {
            let declaration = graph.declarations.get(count).ok_or_else(|| {
                reject(None, "ZRYNA-L3001", "original declaration inventory is incomplete")
            })?;
            if declaration.members as usize > crate::MAX_MEMBERS_PER_DECLARATION {
                return Err(reject(
                    Some(declaration.span),
                    "ZRYNA-L7201",
                    "declaration member count exceeds 1024",
                ));
            }
            if declaration.module != module.id
                || declaration.index != index
                || declaration.parameters > 2
                || declaration.members == 0
                || declaration.span.file() != module.source_file
                || sources.resolve(declaration.span).is_err()
            {
                return Err(reject(
                    None,
                    "ZRYNA-L3001",
                    "original declaration identity, arity or source span differs",
                ));
            }
            count = count.checked_add(1).ok_or(Failure::InternalFailure)?;
        }
    }
    if count != graph.declarations.len() {
        return Err(reject(None, "ZRYNA-L3001", "extra original declarations were claimed"));
    }
    Ok(())
}
