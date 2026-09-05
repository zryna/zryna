use super::{Errors, PrivateOwnedAggregateLowerer};

impl<'a, 'f> PrivateOwnedAggregateLowerer<'a, 'f, '_> {
    pub(super) fn structured_scratch<'e>(
        &self,
        errors: &'e mut Errors<'a>,
    ) -> PrivateOwnedAggregateLowerer<'a, 'f, 'e> {
        PrivateOwnedAggregateLowerer {
            input: self.input,
            file: self.file,
            function: self.function,
            module: self.module,
            declarations: self.declarations,
            graph: self.graph,
            node_types: self.node_types,
            layouts: self.layouts,
            catalog: self.catalog,
            mixed_function: self.mixed_function,
            errors,
            bindings: self.bindings.clone(),
            projections: self.projections.clone(),
            moved_projections: self.moved_projections.clone(),
            partial_roots: self.partial_roots.clone(),
            places: self.places.clone(),
            instructions: self.instructions.clone(),
            constructor_types: self.constructor_types.clone(),
            constructor_storage: self.preparation_storage(),
            preparation_facts: self.preparation_facts.clone(),
            cleanup_plans: self.cleanup_plans.clone(),
            cleanup_actions: self.cleanup_actions,
            aggregate_operands: self.aggregate_operands,
            aggregate_subobject_moves: self.aggregate_subobject_moves,
            projected_aggregate_clones: self.projected_aggregate_clones,
            projected_aggregate_assignments: self.projected_aggregate_assignments,
            reserved_transitions: self.reserved_transitions,
            owners: self.owners.clone(),
            next_value: self.next_value,
            next_local: self.next_local,
        }
    }
}
