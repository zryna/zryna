//! Actual decoded data/control roles, with bounded operand and structured-region stacks.
use super::predicates::Window;
use super::{Diagnostic, Event, Expr, Model, Region, call, invariant, push, sections};
use wasmparser::{FunctionBody, Operator as O};
mod control;
mod values;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Runtime,
    RawRelease,
    StrongBegin,
    StrongFinish,
    WeakRelease,
    Allocate,
    VectorAllocate,
    VectorRelease,
    Clone,
    Drop,
    Release,
    SharedDrop,
    Source,
}
/// Whether source local3 retains the recognized `WeakUpgrade` result status.
#[derive(Clone, Copy, PartialEq, Eq)]
enum UpgradeStatus {
    Absent,
    Stored,
}
struct State {
    runtime: Option<Window>,
    values: [Expr; 32],
    len: usize,
    regions: [Region; 32],
    depth: usize,
    alias: Expr,
    last_call: bool,
    last_target: Option<u32>,
    pending: bool,
    upgrade_status: UpgradeStatus,
    status: Option<Expr>,
    role: Role,
    read: bool,
}
impl State {
    fn container(&self) -> bool {
        self.runtime.is_some()
    }
    fn counts(&self) -> bool {
        matches!(self.role, Role::StrongBegin | Role::StrongFinish | Role::WeakRelease)
    }
    fn track_live(&self) -> bool {
        self.counts() || self.role == Role::RawRelease
    }
    fn source(&self) -> bool {
        self.role == Role::Source
    }
    fn semantic(&self) -> bool {
        !matches!(
            self.role,
            Role::Runtime
                | Role::Allocate
                | Role::RawRelease
                | Role::StrongBegin
                | Role::StrongFinish
                | Role::WeakRelease
        )
    }
    fn child(&self) -> bool {
        self.role == Role::Clone
    }
    fn release(&self) -> bool {
        matches!(self.role, Role::Release | Role::SharedDrop)
    }

    fn vector(&self) -> bool {
        matches!(self.role, Role::VectorAllocate | Role::VectorRelease)
    }
    fn wide(&self) -> bool {
        self.role == Role::Allocate
    }
    fn shared(&self) -> bool {
        self.role == Role::SharedDrop
    }
    fn push(&mut self, e: Expr) -> Result<(), Diagnostic> {
        *self.values.get_mut(self.len).ok_or_else(invariant)? = e;
        self.len += 1;
        Ok(())
    }
    fn pop(&mut self) -> Result<Expr, Diagnostic> {
        self.len = self.len.checked_sub(1).ok_or_else(invariant)?;
        Ok(self.values[self.len])
    }
    fn region(&mut self, r: Region) -> Result<(), Diagnostic> {
        *self.regions.get_mut(self.depth).ok_or_else(invariant)? = r;
        self.depth += 1;
        Ok(())
    }
    fn inside(&self, r: Region) -> bool {
        self.regions[..self.depth].contains(&r)
    }
}
pub(super) fn events(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<Vec<Event>, Diagnostic> {
    let mut state = State {
        runtime: if index >= m.container_base as usize && index < m.total_functions() as usize {
            Some(Window::new(u32::try_from(index).map_err(|_| invariant())? - m.container_base))
        } else {
            None
        },
        values: [Expr::Unknown; 32],
        len: 0,
        regions: [Region::None; 32],
        depth: 0,
        alias: Expr::Local(4),
        last_call: false,
        last_target: None,
        pending: false,
        upgrade_status: UpgradeStatus::Absent,
        status: None,
        role: if m.container_count != 0 && index == m.container_base as usize + 1 {
            Role::RawRelease
        } else if m.container_count != 0 && index == m.container_base as usize + 7 {
            Role::StrongBegin
        } else if m.container_count != 0 && index == m.container_base as usize + 8 {
            Role::StrongFinish
        } else if m.container_count != 0 && index == m.container_base as usize + 9 {
            Role::WeakRelease
        } else if m.container_count != 0 && index == m.container_base as usize {
            Role::Allocate
        } else if m.container_count != 0 && index == m.container_base as usize + 2 {
            Role::VectorAllocate
        } else if m.container_count != 0 && index == m.container_base as usize + 3 {
            Role::VectorRelease
        } else if index >= m.source_base as usize && index < m.wrapper_base as usize {
            Role::Source
        } else if index >= 5 && index < 5 + m.types.len() {
            Role::Clone
        } else if index >= 5 + m.types.len() && index < m.source_base as usize {
            match m.types[index - 5 - m.types.len()].category() {
                zryna_layout::TypeCategory::Shared => Role::SharedDrop,
                zryna_layout::TypeCategory::String
                | zryna_layout::TypeCategory::Vec
                | zryna_layout::TypeCategory::Weak => Role::Release,
                _ => Role::Drop,
            }
        } else {
            Role::Runtime
        },
        read: index >= m.container_base as usize
            || index == 0
            || (index >= 5 && index < m.observer as usize)
            || (m.observed() && index == m.recorder as usize),
    };
    let mut events = Vec::new();
    let mut reader = body.get_operators_reader().map_err(|_| invariant())?;
    while !reader.eof() {
        m.take(1)?;
        let op = reader.read().map_err(|_| invariant())?;
        if let Some(window) = &mut state.runtime {
            window.push(&op, m)?;
        }
        if !values::step(&mut state, &op, &mut events, m)? {
            control::step(&mut state, &op, &mut events, m)?;
        }
        state.last_call = matches!(op, O::Call { .. })
            || (state.last_call && matches!(op, O::LocalTee { local_index: 2 | 3 }));
    }
    if state.depth != 0 {
        return Err(invariant());
    }
    Ok(events)
}
