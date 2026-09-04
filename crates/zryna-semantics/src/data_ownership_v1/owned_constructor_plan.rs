use std::collections::BTreeSet;

use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_layout::{self as layout, TypeCategory};

use super::owner_state::{OwnerDelta, OwnerState};
use super::type_model::Ty;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ConstructorKind {
    Struct,
    Enum { variant: u32 },
    FixedArray,
    Vec,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ConstructorPlanError {
    WrongType,
    WrongShape,
    MissingOwner,
    UnavailableOwner,
    DuplicateOwner,
}

pub(super) struct ConstructorShape {
    result: Ty,
    kind: ConstructorKind,
    operands: Vec<Ty>,
}

pub(super) struct PreparedConstructor {
    shape: ConstructorShape,
    values: Vec<raw::ValueId>,
    consumed: Vec<(raw::ValueId, raw::PlaceId)>,
}

impl ConstructorShape {
    pub(super) fn derive(
        layouts: &layout::VerifiedLayouts,
        result: Ty,
        kind: ConstructorKind,
        operand_count: usize,
        mut resolve: impl FnMut(layout::TypeId) -> Option<Ty>,
    ) -> Result<Self, ConstructorPlanError> {
        use ConstructorPlanError::{WrongShape, WrongType};
        let record = checked_type(layouts, result)?;
        if operand_count > ir::MAX_AGGREGATE_OPERANDS {
            return Err(WrongShape);
        }
        let ids = match kind {
            ConstructorKind::Struct if record.category() == TypeCategory::Struct => {
                record.fields().iter().map(|field| field.ty()).collect::<Vec<_>>()
            }
            ConstructorKind::Enum { variant } if record.category() == TypeCategory::Enum => record
                .variants()
                .get(variant as usize)
                .ok_or(WrongShape)?
                .payload()
                .into_iter()
                .collect(),
            ConstructorKind::FixedArray if record.category() == TypeCategory::FixedArray => {
                if record.array_length() != Some(operand_count as u64) {
                    return Err(WrongShape);
                }
                vec![record.referenced_type().ok_or(WrongType)?; operand_count]
            }
            ConstructorKind::Vec if record.category() == TypeCategory::Vec => {
                let element = record.referenced_type().ok_or(WrongType)?;
                if layouts.type_by_id(element).ok_or(WrongType)?.size() == 0 {
                    return Err(WrongShape);
                }
                vec![element; operand_count]
            }
            _ => return Err(WrongType),
        };
        if ids.len() != operand_count {
            return Err(WrongShape);
        }
        let operands = ids
            .into_iter()
            .map(|id| {
                let ty = resolve(id).ok_or(WrongType)?;
                if ty.layout != id {
                    return Err(WrongType);
                }
                checked_type(layouts, ty)?;
                Ok(ty)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { result, kind, operands })
    }

    pub(super) fn prepare(
        self,
        values: &[raw::ValueId],
        value_type: impl Fn(raw::ValueId) -> Option<raw::TypeId>,
        owners: &OwnerState,
    ) -> Result<PreparedConstructor, ConstructorPlanError> {
        use ConstructorPlanError::{
            DuplicateOwner, MissingOwner, UnavailableOwner, WrongShape, WrongType,
        };
        if values.len() != self.operands.len() {
            return Err(WrongShape);
        }
        let mut seen = BTreeSet::new();
        let mut consumed = Vec::new();
        for (value, ty) in values.iter().zip(&self.operands) {
            if value_type(*value) != Some(ty.ir) {
                return Err(WrongType);
            }
            let owner = owners.owner(*value);
            if ty.is_copy() {
                if owner.is_some() {
                    return Err(WrongType);
                }
                continue;
            }
            let owner = owner.ok_or(MissingOwner)?;
            if !owners.contains(owner) {
                return Err(UnavailableOwner);
            }
            if !seen.insert(owner) {
                return Err(DuplicateOwner);
            }
            consumed.push((*value, owner));
        }
        let values = values.to_vec();
        Ok(PreparedConstructor { shape: self, values, consumed })
    }
}

impl PreparedConstructor {
    pub(super) fn result_type(&self) -> Ty {
        self.shape.result
    }

    pub(super) fn instruction(
        &self,
        cleanup: Option<raw::CleanupPlanId>,
    ) -> Result<raw::InstructionKind, ConstructorPlanError> {
        if (self.shape.kind == ConstructorKind::Vec) != cleanup.is_some() {
            return Err(ConstructorPlanError::WrongShape);
        }
        Ok(match self.shape.kind {
            ConstructorKind::Struct => {
                raw::InstructionKind::StructConstruct { fields: self.values.clone(), cleanup: None }
            }
            ConstructorKind::Enum { variant } => raw::InstructionKind::EnumConstruct {
                variant,
                payload: self.values.first().copied(),
                cleanup: None,
            },
            ConstructorKind::FixedArray => raw::InstructionKind::FixedArrayConstruct {
                elements: self.values.clone(),
                cleanup: None,
            },
            ConstructorKind::Vec => raw::InstructionKind::VecConstruct {
                elements: self.values.clone(),
                cleanup: cleanup.expect("Vec preparation cleanup"),
            },
        })
    }

    pub(super) fn commit(self, owners: &mut OwnerState) -> Vec<OwnerDelta> {
        assert!(
            self.consumed.iter().all(|(value, owner)| owners.owner(*value) == Some(*owner)),
            "prepared constructor operand identities remain unchanged through result emission"
        );
        let values = self.consumed.iter().map(|(value, _)| *value).collect::<Vec<_>>();
        owners
            .transfer_batch(&values)
            .expect("prepared constructor operands remain pending through result emission")
    }
}

fn checked_type(
    layouts: &layout::VerifiedLayouts,
    ty: Ty,
) -> Result<layout::VerifiedType<'_>, ConstructorPlanError> {
    let record = layouts.type_by_id(ty.layout).ok_or(ConstructorPlanError::WrongType)?;
    if ty.ir != raw::TypeId(ty.layout.index())
        || ty.category != record.category()
        || ty.drop_kind != record.drop_kind()
        || ty.runtime_kind != record.runtime_kind()
    {
        return Err(ConstructorPlanError::WrongType);
    }
    Ok(record)
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct ConstructorValueTypes {
    types: Vec<raw::TypeId>,
    scanned_instructions: usize,
}

impl ConstructorValueTypes {
    pub(super) fn record_parameter(
        &mut self,
        definition: &raw::ValueDefinition,
    ) -> Result<(), ConstructorPlanError> {
        if self.scanned_instructions != 0 || definition.id.0 as usize != self.types.len() {
            return Err(ConstructorPlanError::WrongShape);
        }
        self.types.push(definition.ty);
        Ok(())
    }

    pub(super) fn observe(
        &mut self,
        instructions: &[raw::Instruction],
    ) -> Result<usize, ConstructorPlanError> {
        let pending = instructions
            .get(self.scanned_instructions..)
            .ok_or(ConstructorPlanError::WrongShape)?;
        let mut additional = Vec::new();
        for definition in pending.iter().filter_map(|instruction| instruction.result.as_ref()) {
            if definition.id.0 as usize != self.types.len() + additional.len() {
                return Err(ConstructorPlanError::WrongShape);
            }
            additional.push(definition.ty);
        }
        let observed = pending.len();
        self.types.extend(additional);
        self.scanned_instructions = instructions.len();
        Ok(observed)
    }

    pub(super) fn get(&self, value: raw::ValueId) -> Option<raw::TypeId> {
        self.types.get(value.0 as usize).copied()
    }
}
