use std::collections::BTreeMap;

use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::raw;
use zryna_layout as layout;
use zryna_syntax::v4::RawStatementKind;

use super::super::layout_graph::{build_graph, semantic_type};
use super::super::{Errors, SemanticInput, map_node_types, span};
use super::projection_resolution::ProjectionResolver;
use super::projection_topology::{
    MaterializedProjectionTopology, ProjectionDescriptor, ProjectionTopology,
};

struct ScratchTopology {
    initial_places: usize,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    descriptors: Vec<ProjectionDescriptor>,
}

impl ProjectionTopology for ScratchTopology {
    fn cached(&self, key: (u32, u8, u32)) -> Option<raw::PlaceId> {
        self.projections.get(&key).copied()
    }

    fn used_places(&self) -> usize {
        self.initial_places + self.descriptors.len()
    }

    fn insert(&mut self, descriptor: ProjectionDescriptor) -> Option<raw::PlaceId> {
        let place = raw::PlaceId(u32::try_from(self.used_places()).ok()?);
        self.projections.insert(descriptor.key, place);
        self.descriptors.push(descriptor);
        Some(place)
    }
}

pub(in crate::data_ownership_v1) struct ProjectionCheck {
    pub(in crate::data_ownership_v1) places: Vec<raw::Place>,
    pub(in crate::data_ownership_v1) diagnostics: Vec<Diagnostic>,
    pub(in crate::data_ownership_v1) resolved: Vec<Option<raw::PlaceId>>,
}

pub(in crate::data_ownership_v1) fn compare(
    input: SemanticInput<'_>,
    expressions: &[u32],
    initial_places: usize,
    omit_declaration: Option<&str>,
) -> ProjectionCheck {
    assert!(initial_places > 0);
    let mut errors = Errors::new(input.sources());
    let (graph, declarations) = build_graph(input, &mut errors);
    let layouts = layout::verify(&graph, input.sources(), layout::StorageTarget::Linear32V1)
        .expect("authenticated projection layouts");
    let node_types = map_node_types(&graph, &layouts, &mut errors);
    let file = &input.syntax().files()[0];
    let function = &file.functions()[0];
    let (name, type_syntax, mutable) = function
        .body
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            RawStatementKind::LocalDeclaration { name, type_syntax, mutable, .. } => {
                Some((name.text.clone(), *type_syntax, *mutable))
            }
            _ => None,
        })
        .or_else(|| {
            function.parameters.first().map(|p| (p.name.text.clone(), p.type_syntax, false))
        })
        .expect("fixture root binding");
    let ty = semantic_type(file, type_syntax, 0, &declarations, &graph, &node_types, &mut errors)
        .expect("fixture root type");
    assert!(errors.finish().is_empty());
    let bindings =
        BTreeMap::from([(name, super::super::Binding { ty, place: raw::PlaceId(0), mutable })]);
    let declarations = declarations
        .into_iter()
        .filter(|decl| Some(decl.name.as_str()) != omit_declaration)
        .collect::<Vec<_>>();
    let at = span(input.sources(), function.span);
    // Counter-frontier padding only: duplicate root IDs are not valid full IR.
    // Source authentication and independent full lowering are checked separately.
    let mut places =
        vec![
            raw::Place { id: raw::PlaceId(0), ty: ty.ir, span: at, kind: raw::PlaceKind::Local(0) };
            initial_places
        ];
    let original_places = places.clone();
    let mut projections = BTreeMap::new();
    let mut scratch =
        ScratchTopology { initial_places, projections: BTreeMap::new(), descriptors: Vec::new() };
    let mut scratch_errors = Errors::new(input.sources());
    let mut resolver = ProjectionResolver {
        input,
        file,
        function,
        module: 0,
        declarations: &declarations,
        graph: &graph,
        node_types: &node_types,
        layouts: &layouts,
        bindings: &bindings,
        errors: &mut scratch_errors,
    };
    let expected =
        expressions.iter().map(|id| resolver.resolve(*id, &mut scratch)).collect::<Vec<_>>();
    assert_eq!(places, original_places, "scratch resolution cannot mutate raw places");
    assert!(projections.is_empty(), "scratch resolution cannot mutate real topology");
    let mut materialized_errors = Errors::new(input.sources());
    resolver.errors = &mut materialized_errors;
    let mut materialized = MaterializedProjectionTopology {
        places: &mut places,
        projections: &mut projections,
        reserved_places: 0,
    };
    let actual =
        expressions.iter().map(|id| resolver.resolve(*id, &mut materialized)).collect::<Vec<_>>();
    assert_eq!(actual, expected, "same typed descriptor results");
    assert_eq!(projections, scratch.projections, "same canonical prefix reuse");
    let predicted = scratch
        .descriptors
        .into_iter()
        .enumerate()
        .map(|(index, descriptor)| raw::Place {
            id: raw::PlaceId(u32::try_from(initial_places + index).expect("bounded place")),
            ty: descriptor.ty.ir,
            span: descriptor.at,
            kind: descriptor.kind,
        })
        .collect::<Vec<_>>();
    assert_eq!(&places[initial_places..], predicted, "same ordered places and first spans");
    let diagnostics = materialized_errors.finish();
    assert_eq!(diagnostics, scratch_errors.finish(), "same exact diagnostic recipes and order");
    ProjectionCheck {
        places: places.split_off(initial_places),
        diagnostics,
        resolved: actual.into_iter().map(|place| place.map(|place| place.place)).collect(),
    }
}
