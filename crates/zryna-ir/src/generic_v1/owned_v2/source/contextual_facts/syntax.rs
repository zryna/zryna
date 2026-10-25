//! Metered source materialization uses an explicit key-depth-bounded stack.
use super::{Budget, Engine, Failure, Frame, add, budget, reject};
use crate::generic_v1::source::Target;
use zryna_syntax::v5::{RawDataDeclarationKind, RawTypeSyntaxKind};

#[derive(Clone, Copy)]
pub(super) enum Resolved {
    Stored(u32),
    Unit,
    Borrow(u32, bool),
}
enum Task {
    Node(u32, usize),
    Finish { tag: u32, module: u32, index: u32, count: usize },
    Borrow(bool),
}
impl Engine<'_, '_> {
    pub(super) fn syntax(
        &mut self,
        frame: Frame,
        args: &[u32],
        occurrence: u32,
        credit: &mut Budget,
    ) -> Result<Resolved, Failure> {
        let mut tasks = Vec::new();
        let mut values = Vec::new();
        push(&mut tasks, Task::Node(occurrence, 0), credit)?;
        while let Some(task) = tasks.pop() {
            match task {
                Task::Node(at, depth) => {
                    self.source_node(frame, args, (at, depth), &mut tasks, &mut values, credit)?;
                }
                Task::Borrow(exclusive) => {
                    let Some(Resolved::Stored(child)) = values.pop() else {
                        return Err(reject("borrow referent is not stored"));
                    };
                    push(&mut values, Resolved::Borrow(child, exclusive), credit)?;
                }
                Task::Finish { tag, module, index, count } => {
                    let mut children = [0; 2];
                    for slot in (0..count).rev() {
                        let Some(Resolved::Stored(child)) = values.pop() else {
                            return Err(reject("unit/borrow is not a stored child"));
                        };
                        children[slot] = child;
                    }
                    let id = self.application(tag, module, index, &children[..count], credit)?;
                    push(&mut values, Resolved::Stored(id), credit)?;
                }
            }
        }
        if values.len() != 1 {
            return Err(Failure::InternalFailure);
        }
        values.pop().ok_or(Failure::InternalFailure)
    }
    fn source_node(
        &mut self,
        frame: Frame,
        args: &[u32],
        position: (u32, usize),
        tasks: &mut Vec<Task>,
        values: &mut Vec<Resolved>,
        credit: &mut Budget,
    ) -> Result<(), Failure> {
        let (at, depth) = position;
        credit.nominal(1)?;
        if depth > 64 {
            return Err(budget("contextual application depth exceeds 64"));
        }
        let kind = &self
            .originals
            .units
            .get(frame.module as usize)
            .and_then(|u| u.type_syntax.get(at as usize))
            .ok_or(Failure::InternalFailure)?
            .kind;
        match kind {
            RawTypeSyntaxKind::Named { name } => {
                let value = if let Some(slot) = self.slot(frame, &name.text, credit)? {
                    Resolved::Stored(
                        *args.get(slot).ok_or_else(|| reject("unbound contextual source slot"))?,
                    )
                } else {
                    match name.text.as_str() {
                        "unit" => Resolved::Unit,
                        "bool" => Resolved::Stored(self.intern([0; 6], credit)?),
                        "i32" => Resolved::Stored(self.intern([1, 0, 0, 0, 0, 0], credit)?),
                        _ => {
                            let (tag, module, index, count) =
                                self.head(frame.module, &name.text, credit)?;
                            if count != 0 {
                                return Err(reject(
                                    "nominal type requires exact explicit arguments",
                                ));
                            }
                            Resolved::Stored(self.application(tag, module, index, &[], credit)?)
                        }
                    }
                };
                push(values, value, credit)?;
            }
            RawTypeSyntaxKind::String { .. } => {
                let id = self.intern([2, 0, 0, 0, 0, 0], credit)?;
                push(values, Resolved::Stored(id), credit)?;
            }
            RawTypeSyntaxKind::Application { name, type_arguments } => {
                if self.slot(frame, &name.text, credit)?.is_some() {
                    return Err(reject("opaque parameter cannot be an application head"));
                }
                let (tag, module, index, count) = self.head(frame.module, &name.text, credit)?;
                if count == 0 || count != type_arguments.arguments.len() {
                    return Err(reject("contextual application differs from exact original arity"));
                }
                push(tasks, Task::Finish { tag, module, index, count }, credit)?;
                for child in type_arguments.arguments.iter().rev() {
                    if *child >= at {
                        return Err(reject("source type forest is forward/cyclic"));
                    }
                    push(tasks, Task::Node(*child, add(depth, 1)?), credit)?;
                }
            }
            RawTypeSyntaxKind::Borrow { argument, .. }
            | RawTypeSyntaxKind::BorrowMut { argument, .. } => {
                if *argument >= at {
                    return Err(reject("source borrow has a forward/cyclic referent"));
                }
                push(
                    tasks,
                    Task::Borrow(matches!(kind, RawTypeSyntaxKind::BorrowMut { .. })),
                    credit,
                )?;
                push(tasks, Task::Node(*argument, depth), credit)?;
            }
            _ => {
                return Err(reject(
                    "contextual source requires scalar/String/nominal/Option/Result",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn head(
        &self,
        module: u32,
        name: &str,
        credit: &mut Budget,
    ) -> Result<(u32, u32, u32, usize), Failure> {
        credit.nominal(add(name.len(), 1)?)?;
        match name {
            "Option" => return Ok((0x14, 0, 0, 1)),
            "Result" => return Ok((0x15, 0, 0, 2)),
            _ => {}
        }
        let Target::Data(module, index) =
            self.originals.resolve_v4(module, name, &mut |n| credit.nominal(n))?
        else {
            return Err(reject("stored source name denotes a function"));
        };
        credit.nominal(1)?;
        let original = &self.originals.units[module as usize].data_declarations[index as usize];
        let count = original.type_parameters.as_ref().map_or(0, |p| p.parameters.len());
        if count > 2 {
            return Err(reject("contextual nominal arity exceeds two"));
        }
        let tag = match (&original.kind, count) {
            (RawDataDeclarationKind::Struct { .. }, 0) => 0x10,
            (RawDataDeclarationKind::Enum { .. }, 0) => 0x11,
            (RawDataDeclarationKind::Struct { .. }, _) => 0x12,
            (RawDataDeclarationKind::Enum { .. }, _) => 0x13,
        };
        Ok((tag, module, index, count))
    }
    fn slot(
        &self,
        frame: Frame,
        name: &str,
        credit: &mut Budget,
    ) -> Result<Option<usize>, Failure> {
        credit.nominal(1)?;
        let parameters = frame.parameters(self.originals)?;
        if let Some(parameters) = parameters {
            for (slot, parameter) in parameters.parameters.iter().enumerate() {
                credit.nominal(add(parameter.name.text.len().min(name.len()), 1)?)?;
                if parameter.name.text == name {
                    return Ok(Some(slot));
                }
            }
        }
        Ok(None)
    }
}
pub(super) fn push<T>(rows: &mut Vec<T>, value: T, credit: &mut Budget) -> Result<(), Failure> {
    credit.nominal(1)?;
    rows.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
    rows.push(value);
    Ok(())
}
