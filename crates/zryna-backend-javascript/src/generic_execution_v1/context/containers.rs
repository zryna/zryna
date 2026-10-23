//! Dense exact-brand runtime metadata; each retained slot and source row is paid before work.
use super::{Context, charge, collect, emission_budget, internal};
use std::cell::Cell;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::{StorageTarget, generic_v1::TypeView};
use zryna_ownership_runtime_abi::generic_v1::{ControlLayout, ElementLayout};
pub(in crate::generic_execution_v1) struct Containers {
    pub(in crate::generic_execution_v1) elements: Vec<Option<ElementLayout>>,
    pub(in crate::generic_execution_v1) controls: Vec<Option<ControlLayout>>,
    pub(in crate::generic_execution_v1) has_vec: bool,
    pub(in crate::generic_execution_v1) has_control: bool,
}
impl Containers {
    pub(in crate::generic_execution_v1) fn new(
        program: &VerifiedExecution<'_>,
        types: &[TypeView<'_>],
        work: &Cell<usize>,
    ) -> Result<Self, Diagnostic> {
        let runtime = program.runtime();
        let element_count = if runtime.element_layouts().is_empty() { 0 } else { types.len() };
        let control_count = if runtime.control_layouts().is_empty() { 0 } else { types.len() };
        let mut elements = collect((0..element_count).map(|_| None), work)?;
        let mut controls = collect((0..control_count).map(|_| None), work)?;
        charge(
            work,
            runtime
                .element_layouts()
                .len()
                .checked_add(runtime.control_layouts().len())
                .ok_or_else(emission_budget)?,
        )?;
        let mut has_vec = false;
        let mut has_control = false;
        for record in runtime.element_layouts() {
            if record.target() != StorageTarget::Linear32V1 {
                continue;
            }
            let index = record.element().index() as usize;
            if types.get(index).ok_or_else(internal)?.id() != record.element() {
                return Err(internal());
            }
            if elements.get_mut(index).ok_or_else(internal)?.replace(*record).is_some() {
                return Err(internal());
            }
            has_vec = true;
        }
        for record in runtime.control_layouts() {
            if record.target() != StorageTarget::Linear32V1 {
                continue;
            }
            let index = record.payload().index() as usize;
            if types.get(index).ok_or_else(internal)?.id() != record.payload() {
                return Err(internal());
            }
            if controls.get_mut(index).ok_or_else(internal)?.replace(*record).is_some() {
                return Err(internal());
            }
            has_control = true;
        }
        Ok(Self { elements, controls, has_vec, has_control })
    }
}
impl Context<'_, '_> {
    pub(in crate::generic_execution_v1) fn element(
        &self,
        id: u32,
    ) -> Result<ElementLayout, Diagnostic> {
        charge(&self.work, 1)?;
        self.containers.elements.get(id as usize).copied().flatten().ok_or_else(internal)
    }
    pub(in crate::generic_execution_v1) fn control(
        &self,
        id: u32,
    ) -> Result<ControlLayout, Diagnostic> {
        charge(&self.work, 1)?;
        self.containers.controls.get(id as usize).copied().flatten().ok_or_else(internal)
    }
}
