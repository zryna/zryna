//! Authenticated member order, active payloads and independent match alternatives.
use super::{
    Checker, Closed, Failure, UntrustedSpan, Use, add, charge, clone_type, copy_key, describe,
    description, expressions, key, reject, reserve, resolve, symbolic_fact, v5,
};
use zryna_syntax::v4::{
    RawFieldInitializer, RawFieldInitializerKind, RawIdentifierSyntax, RawMatchArm,
};
impl<'s> Checker<'_, 's, '_> {
    pub(super) fn arguments(
        &mut self,
        args: Option<&v5::RawTypeArgumentList>,
    ) -> Result<Vec<Vec<u8>>, Failure> {
        let count = args.map_or(0, |args| args.arguments.len());
        charge(self.spent, count)?;
        let mut result = reserve(count)?;
        for at in args.map_or(&[][..], |args| args.arguments.as_slice()) {
            let ty = resolve(&self.resolver, *at, self.spent)?;
            symbolic_fact(self.resolver.originals, self.owner, &ty, self.spent)?;
            let Closed::Stored(key) = ty else {
                return Err(reject("original generic argument is not stored"));
            };
            result.push(key);
        }
        Ok(result)
    }
    pub(super) fn structure(
        &mut self,
        name: &RawIdentifierSyntax,
        args: Option<&v5::RawTypeArgumentList>,
        fields: &[RawFieldInitializer],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let arguments = self.arguments(args)?;
        let desc = describe(&self.resolver, &name.text, &arguments, self.spent)?;
        if !desc.structure || fields.len() != desc.members.len() {
            return Err(self.error(
                span,
                "ZRYNA-M7006",
                "original struct requires every declared field",
            ));
        }
        charge(self.spent, desc.members.len())?;
        let mut seen = reserve(desc.members.len())?;
        seen.resize(desc.members.len(), false);
        for field in fields {
            let (name, at) = match &field.kind {
                RawFieldInitializerKind::Shorthand { name, value }
                | RawFieldInitializerKind::Explicit { name, value, .. } => (name, *value),
            };
            let mut found = None;
            for (index, (declared, _)) in desc.members.iter().enumerate() {
                charge(self.spent, add(name.text.len().min(declared.len()), 1)?)?;
                if name.text == *declared {
                    found = Some(index);
                    break;
                }
            }
            let index = found.ok_or_else(|| {
                self.error(field.span, "ZRYNA-M7006", "original initializer names an unknown field")
            })?;
            if seen[index] {
                return Err(self.error(
                    field.span,
                    "ZRYNA-M7006",
                    "original initializer repeats a field",
                ));
            }
            seen[index] = true;
            let actual = self.expression(at, Use::Move, depth + 1)?;
            self.expect(
                &actual,
                desc.members[index].1.as_ref().ok_or(Failure::InternalFailure)?,
                field.span,
            )?;
        }
        symbolic_fact(self.resolver.originals, self.owner, &desc.ty, self.spent)?;
        Ok(desc.ty)
    }
    pub(super) fn enumeration(
        &mut self,
        name: &RawIdentifierSyntax,
        variant: &RawIdentifierSyntax,
        args: Option<&v5::RawTypeArgumentList>,
        payload: Option<u32>,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let arguments = self.arguments(args)?;
        let desc = describe(&self.resolver, &name.text, &arguments, self.spent)?;
        if desc.structure {
            return Err(self.error(span, "ZRYNA-M7006", "enum construction names a struct"));
        }
        let mut expected = None;
        let mut found = false;
        for (name, member) in &desc.members {
            charge(self.spent, add(name.len().min(variant.text.len()), 1)?)?;
            if *name == variant.text {
                expected = member.as_ref();
                found = true;
                break;
            }
        }
        if !found || expected.is_some() != payload.is_some() {
            return Err(self.error(
                span,
                "ZRYNA-M7006",
                "original enum variant/payload arity differs",
            ));
        }
        if let (Some(at), Some(expected)) = (payload, expected) {
            let actual = self.expression(at, Use::Move, depth + 1)?;
            self.expect(&actual, expected, span)?;
        }
        symbolic_fact(self.resolver.originals, self.owner, &desc.ty, self.spent)?;
        Ok(desc.ty)
    }
    pub(super) fn field(
        &mut self,
        base: u32,
        field: &str,
        usage: Use,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let base = self.expression(base, Use::Observe, depth + 1)?;
        let Closed::Stored(key) = &base else {
            return Err(reject("original field loan observation is unfinished"));
        };
        let shape = key::shape(key, self.spent)?;
        if !matches!(shape.tag, 0x10 | 0x12) {
            return Err(self.error(span, "ZRYNA-M7006", "original field requires exact struct"));
        }
        charge(self.spent, shape.children.len())?;
        let mut args = reserve(shape.children.len())?;
        for key in &shape.children {
            args.push(copy_key(key, self.spent)?);
        }
        let desc = description::identity(
            self.resolver.originals,
            shape.module,
            shape.index,
            &args,
            self.spent,
        )?;
        for (name, ty) in desc.members {
            charge(self.spent, add(name.len().min(field.len()), 1)?)?;
            if name == field {
                let ty = ty.ok_or(Failure::InternalFailure)?;
                if usage == Use::Move
                    && !symbolic_fact(self.resolver.originals, self.owner, &ty, self.spent)?.copy
                {
                    return Err(reject("original partial-field move state is unfinished"));
                }
                return Ok(ty);
            }
        }
        Err(self.error(span, "ZRYNA-M7006", "original field is absent from declaring struct"))
    }
    pub(super) fn index(
        &mut self,
        base: u32,
        at: u32,
        usage: Use,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let base = self.expression(base, Use::Observe, depth + 1)?;
        let index = self.expression(at, Use::Move, depth + 1)?;
        let expected = expressions::scalar(1, self.spent)?;
        self.expect(&index, &expected, span)?;
        let Closed::Stored(key) = base else {
            return Err(reject("original indexed loan observation is unfinished"));
        };
        let shape = key::shape(&key, self.spent)?;
        if !matches!(shape.tag, 0x20 | 0x21) {
            return Err(self.error(span, "ZRYNA-M7006", "original indexing requires array or Vec"));
        }
        let ty = Closed::Stored(copy_key(shape.children[0], self.spent)?);
        if usage == Use::Move
            && !symbolic_fact(self.resolver.originals, self.owner, &ty, self.spent)?.copy
        {
            return Err(reject("original noncopy indexed extraction is unfinished"));
        }
        Ok(ty)
    }
    pub(super) fn matching(
        &mut self,
        at: u32,
        arms: &'s [RawMatchArm],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let scrutinee = self.expression(at, Use::Move, depth + 1)?;
        let Closed::Stored(key) = &scrutinee else {
            return Err(reject("original loan match is unfinished"));
        };
        let shape = key::shape(key, self.spent)?;
        if !matches!(shape.tag, 0x11 | 0x13..=0x15) {
            return Err(self.error(span, "ZRYNA-M7006", "original match requires enum"));
        }
        charge(self.spent, shape.children.len())?;
        let mut args = reserve(shape.children.len())?;
        for key in &shape.children {
            args.push(copy_key(key, self.spent)?);
        }
        let before = self.snapshot()?;
        let mut joined = self.snapshot()?;
        let mut result = None;
        let mut expected_count = None;
        charge(self.spent, arms.len())?;
        let mut seen = reserve(arms.len())?;
        for arm in arms {
            self.restore(&before)?;
            let desc = describe(&self.resolver, &arm.type_name.text, &args, self.spent)?;
            self.expect(&desc.ty, &scrutinee, arm.span)?;
            if desc.structure {
                return Err(reject("original match names a struct"));
            }
            expected_count = Some(desc.members.len());
            let mut ordinal = None;
            for (index, (name, _)) in desc.members.iter().enumerate() {
                charge(self.spent, add(name.len().min(arm.variant.text.len()), 1)?)?;
                if *name == arm.variant.text {
                    ordinal = Some(index);
                    break;
                }
            }
            let ordinal = ordinal.ok_or_else(|| {
                self.error(arm.span, "ZRYNA-M7006", "original match variant is absent")
            })?;
            charge(self.spent, seen.len())?;
            if seen.contains(&ordinal) {
                return Err(self.error(
                    arm.span,
                    "ZRYNA-M7006",
                    "original match repeats a variant",
                ));
            }
            seen.push(ordinal);
            match (&arm.binding, &desc.members[ordinal].1) {
                (Some(binding), Some(ty)) => {
                    let ty = clone_type(ty, self.spent)?;
                    self.bind(&binding.text, ty)?;
                }
                (None, None) => {}
                _ => {
                    return Err(self.error(
                        arm.span,
                        "ZRYNA-M7006",
                        "original match binding arity differs",
                    ));
                }
            }
            let value = self.expression(arm.value, Use::Move, depth + 1)?;
            if let Some(expected) = &result {
                self.expect(&value, expected, arm.span)?;
            } else {
                result = Some(value);
            }
            self.locals.truncate(before.len());
            charge(self.spent, before.len())?;
            for (state, local) in joined.iter_mut().zip(&self.locals) {
                *state &= local.available;
            }
        }
        if expected_count != Some(arms.len()) {
            return Err(self.error(span, "ZRYNA-M7006", "original match must cover all variants"));
        }
        self.restore(&joined)?;
        result.ok_or_else(|| self.error(span, "ZRYNA-M7006", "original match has no arms"))
    }
}
