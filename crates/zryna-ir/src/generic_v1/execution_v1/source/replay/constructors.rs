use super::{
    Body, Closed, Failure, Kind, UntrustedSpan, Value, charge, lookup, originals, raw, reject,
    reserve,
};
use zryna_syntax::v4::{RawFieldInitializer, RawIdentifierSyntax};
use zryna_syntax::v5::RawTypeArgumentList;
impl Body<'_, '_> {
    fn arguments(&mut self, list: Option<&RawTypeArgumentList>) -> Result<Vec<Vec<u8>>, Failure> {
        let items = list.map_or(&[][..], |l| l.arguments.as_slice());
        charge(self.spent, items.len())?;
        let mut out = reserve(items.len())?;
        for at in items {
            let Closed::Stored(key) = originals::resolve(&self.resolver, *at, self.spent)? else {
                return Err(reject("constructor argument must be stored"));
            };
            let id = lookup(&self.program.type_keys, &key, self.spent)?;
            self.used[id as usize] = true;
            out.push(key);
        }
        Ok(out)
    }
    pub(super) fn construct_enum(
        &mut self,
        name: &RawIdentifierSyntax,
        variant: &RawIdentifierSyntax,
        args: Option<&RawTypeArgumentList>,
        payload: Option<u32>,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let args = self.arguments(args)?;
        let desc = originals::describe(&self.resolver, &name.text, &args, self.spent)?;
        if desc.structure {
            return Err(reject("enum constructor names a struct"));
        }
        let mut ordinal = None;
        for (i, (n, _)) in desc.members.iter().enumerate() {
            charge(self.spent, n.len().min(variant.text.len()) + 1)?;
            if *n == variant.text {
                ordinal = Some(i);
                break;
            }
        }
        let ordinal = ordinal.ok_or_else(|| reject("unknown original enum variant"))?;
        let expected =
            desc.members[ordinal].1.as_ref().map(|t| self.closed_borrowed(t)).transpose()?;
        let value = payload.map(|i| self.expression(i, depth + 1)).transpose()?;
        if value.as_ref().map(|v| v.ty) != expected {
            return Err(reject("inexact enum payload"));
        }
        if let Some(v) = value {
            self.consume(v, span)?;
        }
        let ty = self.closed_type(desc.ty)?;
        let raw::ValueType::Stored(id) = ty else { return Err(Failure::InternalFailure) };
        self.emit(
            ty,
            raw::Operation::ClosedEnumConstruct {
                ty: id,
                ordinal: u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
                payload: value.map(|v| v.id),
            },
            span,
        )
    }
    pub(super) fn construct_struct(
        &mut self,
        name: &RawIdentifierSyntax,
        args: Option<&RawTypeArgumentList>,
        fields: &[RawFieldInitializer],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let args = self.arguments(args)?;
        let desc = originals::describe(&self.resolver, &name.text, &args, self.spent)?;
        if !desc.structure || fields.len() != desc.members.len() {
            return Err(reject("inexact struct constructor inventory"));
        }
        charge(self.spent, fields.len() * 2)?;
        let mut prepared = reserve(fields.len())?;
        let mut ordered = reserve(fields.len())?;
        ordered.resize(fields.len(), None);
        for field in fields {
            let (name, at) = match &field.kind {
                zryna_syntax::v4::RawFieldInitializerKind::Explicit { name, value, .. }
                | zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, value } => {
                    (name, *value)
                }
            };
            let mut ordinal = None;
            for (i, (expected, _)) in desc.members.iter().enumerate() {
                charge(self.spent, name.text.len().min(expected.len()) + 1)?;
                if name.text == *expected {
                    ordinal = Some(i);
                    break;
                }
            }
            let ordinal = ordinal.ok_or_else(|| reject("unknown original struct field"))?;
            if ordered[ordinal].is_some() {
                return Err(reject("duplicate original struct field"));
            }
            let value = self.expression(at, depth + 1)?;
            if Some(value.ty)
                != desc.members[ordinal].1.as_ref().map(|t| self.closed_borrowed(t)).transpose()?
            {
                return Err(reject("inexact original struct field type"));
            }
            ordered[ordinal] = Some(value.id);
            prepared.push(value);
        }
        for value in prepared {
            self.consume(value, span)?;
        }
        charge(self.spent, ordered.len())?;
        let mut values = reserve(ordered.len())?;
        for id in ordered {
            values.push(id.ok_or_else(|| reject("missing original struct field"))?);
        }
        let ty = self.closed_type(desc.ty)?;
        let raw::ValueType::Stored(id) = ty else { return Err(Failure::InternalFailure) };
        self.emit(ty, raw::Operation::StructConstruct { ty: id, fields: values }, span)
    }
    pub(super) fn array(
        &mut self,
        syntax: u32,
        elements: &[u32],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let ty = self.resolve(syntax)?;
        let raw::ValueType::Stored(id) = ty else {
            return Err(reject("array construction requires stored type"));
        };
        charge(self.spent, 2)?;
        let Some(Kind::FixedArray { length }) = self.world.kind(id) else {
            return Err(reject("array constructor type differs"));
        };
        if usize::try_from(length).map_err(|_| Failure::InternalFailure)? != elements.len() {
            return Err(reject("fixed array length differs"));
        }
        let element = self
            .world
            .arguments(id)
            .and_then(|a| a.first())
            .copied()
            .ok_or(Failure::InternalFailure)?;
        charge(self.spent, elements.len())?;
        let mut prepared = reserve(elements.len())?;
        for at in elements {
            let value = self.expression(*at, depth + 1)?;
            if value.ty != raw::ValueType::Stored(element) {
                return Err(reject("fixed array element differs"));
            }
            prepared.push(value);
        }
        charge(self.spent, prepared.len())?;
        let mut out = reserve(prepared.len())?;
        for value in prepared {
            self.consume(value, span)?;
            out.push(value.id);
        }
        self.emit(ty, raw::Operation::FixedArrayConstruct { ty: id, elements: out }, span)
    }
    pub(super) fn closed_borrowed(&mut self, t: &Closed) -> Result<raw::ValueType, Failure> {
        match t {
            Closed::Stored(key) => {
                let id = lookup(&self.program.type_keys, key, self.spent)?;
                self.used[id as usize] = true;
                Ok(raw::ValueType::Stored(id))
            }
            Closed::Unit => Ok(raw::ValueType::Unit),
            Closed::Borrow(..) => Err(reject("stored member cannot be a loan")),
        }
    }
}
