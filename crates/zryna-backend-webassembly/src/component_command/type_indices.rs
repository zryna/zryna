//! Single-pass type-index accounting before recursive component validation or WIT decoding.

use std::{collections::BTreeMap, sync::Arc};

use wasmparser::{
    ComponentAlias, ComponentExternalKind, ComponentOuterAliasKind, ComponentValType,
};
use zryna_diagnostics::Diagnostic;

use super::type_budget::invalid;

#[derive(Clone)]
pub(super) enum Shape {
    Value { depth: usize, aliases: usize },
    Function,
    Instance(Arc<Interface>),
}

pub(super) struct Interface {
    pub(super) exports: BTreeMap<String, Shape>,
    pub(super) identities: usize,
}

#[derive(Default)]
pub(super) struct TypeIndices {
    pub(super) outer: Vec<Shape>,
    instances: Vec<Arc<Interface>>,
    identities: usize,
}

impl TypeIndices {
    pub(super) fn identities(&self) -> usize {
        self.identities
    }

    pub(super) fn charge(&mut self, count: usize) -> Result<(), Diagnostic> {
        if count > 4096 - self.identities {
            return Err(invalid("command decoded type identity envelope exceeds 4096"));
        }
        self.identities += count;
        Ok(())
    }

    pub(super) fn value(
        &self,
        scope: Option<&[Shape]>,
        ty: ComponentValType,
    ) -> Result<usize, Diagnostic> {
        match ty {
            ComponentValType::Primitive(wasmparser::PrimitiveValType::ErrorContext) => {
                Err(invalid("command value uses an asynchronous error context"))
            }
            ComponentValType::Primitive(_) => Ok(0),
            ComponentValType::Type(index) => {
                match Self::get(scope.unwrap_or(&self.outer), index)? {
                    Shape::Value { depth, .. } => Ok(*depth),
                    _ => Err(invalid("command value index names a non-value type")),
                }
            }
        }
    }

    pub(super) fn get(scope: &[Shape], index: u32) -> Result<&Shape, Diagnostic> {
        // Only earlier entries can be referenced, excluding cycles without recursive traversal.
        scope
            .get(index as usize)
            .ok_or_else(|| invalid("command type references a missing or forward index"))
    }

    pub(super) fn alias(
        &self,
        scope: Option<&[Shape]>,
        alias: &ComponentAlias<'_>,
    ) -> Result<Shape, Diagnostic> {
        let shape = match *alias {
            ComponentAlias::Outer { kind: ComponentOuterAliasKind::Type, count, index } => {
                let types = match (scope, count) {
                    (None, 0) | (Some(_), 1) => &self.outer[..],
                    (Some(local), 0) => local,
                    _ => return Err(invalid("command type alias has unsupported scope depth")),
                };
                Self::get(types, index)?
            }
            ComponentAlias::InstanceExport {
                kind: ComponentExternalKind::Type,
                instance_index,
                name,
            } if scope.is_none() => self
                .instances
                .get(instance_index as usize)
                .and_then(|interface| interface.exports.get(name))
                .ok_or_else(|| invalid("command alias references an unknown imported type"))?,
            _ => return Err(invalid("command type alias has an unsupported kind")),
        };
        Self::aliased(shape)
    }

    pub(super) fn aliased(shape: &Shape) -> Result<Shape, Diagnostic> {
        match shape {
            Shape::Value { depth, aliases } if *aliases < 32 => {
                Ok(Shape::Value { depth: *depth, aliases: aliases + 1 })
            }
            _ => {
                Err(invalid("command type alias exceeds its identity depth or aliases a non-value"))
            }
        }
    }

    pub(super) fn interface_use(&mut self, index: u32, import: bool) -> Result<(), Diagnostic> {
        let Shape::Instance(interface) = Self::get(&self.outer, index)? else {
            return Err(invalid("command interface reference has a different type kind"));
        };
        let interface = Arc::clone(interface);
        // Reusing one binary instance type can create fresh decoded identities at each use.
        self.charge(interface.identities)?;
        if import {
            self.instances.push(interface);
        }
        Ok(())
    }

    pub(super) fn defined(depth: usize) -> Result<Shape, Diagnostic> {
        if depth > 32 {
            return Err(invalid(
                "command defined-type dependency depth exceeds 32 before decoding",
            ));
        }
        Ok(Shape::Value { depth, aliases: 0 })
    }
}
