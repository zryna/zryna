//! Build predicate edges from every original declaration and type occurrence.

use zryna_syntax::v5::RawDataDeclarationKind;

use super::super::signatures;
use super::{
    BodyTypeFailure, DeclarationContext, Graph, Kind, Origin, Row, Rule, Statuses, Supports,
    Tables, resources,
};

impl Graph {
    pub(in super::super) fn derive(
        context: &DeclarationContext<'_>,
        tables: &Tables,
    ) -> Result<Self, BodyTypeFailure> {
        let types = tables
            .sources
            .iter()
            .try_fold(0, |sum, nodes| resources::checked_add(sum, nodes.len()))?;
        let data =
            context.syntax().files().iter().try_fold(0, |sum, unit| {
                resources::checked_add(sum, unit.data_declarations.len())
            })?;
        let members = context
            .syntax()
            .files()
            .iter()
            .flat_map(|unit| &unit.data_declarations)
            .try_fold(0, |sum, declaration| {
                resources::checked_add(
                    sum,
                    match &declaration.kind {
                        RawDataDeclarationKind::Struct { fields, .. } => fields.len(),
                        RawDataDeclarationKind::Enum { variants, .. } => {
                            variants.iter().filter(|variant| variant.payload_type.is_some()).count()
                        }
                    },
                )
            })?;
        let source_edges = tables.sources.iter().flatten().try_fold(0, |sum, node| {
            let count = node.head.map_or(0, |head| {
                head.children.iter().flatten().count()
                    + usize::from(matches!(head.kind, Kind::Nominal(_)))
            });
            resources::checked_add(sum, count)
        })?;
        let mut graph = Self {
            rows: resources::reserve(resources::checked_add(types, data)?)?,
            offsets: resources::reserve(tables.sources.len())?,
            nominals: resources::reserve(data)?,
            supports: resources::reserve(data)?,
            edges: resources::reserve(resources::checked_add(source_edges, members)?)?,
        };
        for nodes in &tables.sources {
            graph.offsets.push(graph.rows.len());
            for node in nodes {
                graph.rows.push(Row {
                    rule: Rule::Source(node.head),
                    values: Statuses::TRUE,
                    first_reverse: None,
                    support: None,
                });
            }
        }
        for module in context.modules() {
            for declaration in module.data_declarations() {
                let row = graph.rows.len();
                graph.nominals.push((declaration.identity(), row));
                graph.rows.push(Row {
                    rule: Rule::Nominal,
                    values: Statuses::TRUE,
                    first_reverse: None,
                    support: Some(graph.supports.len()),
                });
                graph.supports.push(Supports { false_count: [0; 32], unknown_count: [0; 32] });
                match signatures::data(context, declaration.identity()) {
                    RawDataDeclarationKind::Struct { fields, .. } => {
                        for field in fields {
                            graph.edge(
                                graph.source(module.identity().index(), field.type_syntax),
                                row,
                                true,
                            );
                        }
                    }
                    RawDataDeclarationKind::Enum { variants, .. } => {
                        for variant in variants {
                            if let Some(payload) = variant.payload_type {
                                graph.edge(
                                    graph.source(module.identity().index(), payload),
                                    row,
                                    true,
                                );
                            }
                        }
                    }
                }
            }
        }
        graph.nominals.sort_unstable_by_key(|(owner, _)| *owner);
        for row in 0..types {
            let Rule::Source(Some(head)) = graph.rows[row].rule else { continue };
            for ty in head.children.into_iter().flatten() {
                let Origin::Source { module, occurrence } = ty.origin else {
                    return Err(BodyTypeFailure::InternalFailure);
                };
                graph.edge(graph.source(module, occurrence), row, false);
            }
            if let Kind::Nominal(owner) = head.kind {
                graph.edge(graph.nominal(owner), row, false);
            }
        }
        graph.solve()
    }
}
