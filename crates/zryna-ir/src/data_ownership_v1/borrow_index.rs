use super::{indexed_borrows, layout_type, raw};
use zryna_layout::{TypeCategory, VerifiedLayouts};

#[derive(Clone, Debug)]
struct Entry {
    definition: Option<(raw::TypeId, raw::BorrowAccess)>,
    region: Option<raw::PlaceId>,
    origin: Option<(raw::BorrowDefinition, usize)>,
}

/// Built after dense identity validation, once per function. Each projection reads
/// only its already indexed parent; consumers never walk instruction or parent chains.
#[derive(Clone, Debug, Default)]
pub(super) struct BorrowIndex {
    entries: Vec<Entry>,
    #[cfg(test)]
    pub(super) construction_steps: usize,
    #[cfg(test)]
    pub(super) parent_steps: usize,
}

impl BorrowIndex {
    pub(super) fn new(function: &raw::Function, layouts: &VerifiedLayouts) -> Self {
        let mut index = Self::default();
        for parameter in &function.borrow_parameters {
            #[cfg(test)]
            {
                index.construction_steps += 1;
            }
            index.entries.push(Entry {
                definition: Some((parameter.referent, parameter.access)),
                region: None,
                origin: None,
            });
        }
        for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
            #[cfg(test)]
            {
                index.construction_steps += 1;
            }
            index.insert(function, layouts, &instruction.kind);
        }
        index
    }

    fn insert(
        &mut self,
        function: &raw::Function,
        layouts: &VerifiedLayouts,
        kind: &raw::InstructionKind,
    ) {
        use raw::InstructionKind as I;
        let entry = match kind {
            I::BeginBorrow(definition) => Entry {
                definition: function
                    .places
                    .get(definition.place.0 as usize)
                    .map(|place| (place.ty, definition.access)),
                region: Some(definition.place),
                origin: None,
            },
            I::BeginIndexedBorrow { definition, .. } | I::BeginIndexedAccess { definition, .. } => {
                Entry {
                    definition: indexed_borrows::element_type(function, definition.place, layouts)
                        .map(|ty| (ty, definition.access)),
                    region: Some(definition.place),
                    origin: matches!(kind, I::BeginIndexedAccess { .. })
                        .then_some((definition.clone(), 0)),
                }
            }
            I::ProjectIndexedBorrow { parent, borrow, .. } => {
                #[cfg(test)]
                {
                    self.parent_steps += 1;
                }
                let parent = self.entries.get(parent.0 as usize).filter(|_| parent.0 < borrow.0);
                let origin = parent
                    .and_then(|entry| entry.origin.clone())
                    .map(|(origin, depth)| (origin, depth + 1));
                let definition =
                    origin.as_ref().and_then(|_| parent?.definition).and_then(|(ty, access)| {
                        let record = layout_type(layouts, ty)?;
                        (record.category() == TypeCategory::FixedArray)
                            .then(|| record.referenced_type())
                            .flatten()
                            .map(|element| (raw::TypeId(element.index()), access))
                    });
                Entry {
                    definition,
                    region: origin.as_ref().map(|(origin, _)| origin.place),
                    origin,
                }
            }
            _ => return,
        };
        self.entries.push(entry);
    }

    pub(super) fn definition(&self, id: raw::BorrowId) -> Option<(raw::TypeId, raw::BorrowAccess)> {
        self.entries.get(id.0 as usize)?.definition
    }

    pub(super) fn region(&self, id: raw::BorrowId) -> Option<raw::PlaceId> {
        self.entries.get(id.0 as usize)?.region
    }

    pub(super) fn origin(&self, id: raw::BorrowId) -> Option<(&raw::BorrowDefinition, usize)> {
        self.entries.get(id.0 as usize)?.origin.as_ref().map(|(origin, depth)| (origin, *depth))
    }
}
