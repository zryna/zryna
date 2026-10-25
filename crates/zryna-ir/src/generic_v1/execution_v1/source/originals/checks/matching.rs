//! Independent symbolic enum alternatives preserve borrowed parent and selected arm authority.
use super::{
    Checker, Closed, Failure, UntrustedSpan, Use, add, charge, clone_type, copy_key, describe, key,
    reject, reserve,
};
use zryna_syntax::v4::RawMatchArm;
mod borrowed;
impl<'s> Checker<'_, 's, '_> {
    pub(super) fn matching(
        &mut self,
        at: u32,
        arms: &'s [RawMatchArm],
        usage: Use,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let outer = self.locals.len();
        let (scrutinee, parent) = self.match_source(at, depth)?;
        let Closed::Stored(key) = &scrutinee else {
            return Err(self.error(
                span,
                "ZRYNA-M7006",
                "original match requires a stored enum referent",
            ));
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
        let mut joined: Option<Vec<super::partial::LocalState>> = None;
        let mut result = None;
        let mut expected_count = None;
        charge(self.spent, arms.len())?;
        let mut seen = reserve(arms.len())?;
        for arm in arms {
            self.restore(&before)?;
            let (count, ordinal, payload) = self.match_variant(arm, &args, &scrutinee)?;
            expected_count = Some(count);
            charge(self.spent, seen.len())?;
            if seen.contains(&ordinal) {
                return Err(self.error(
                    arm.span,
                    "ZRYNA-M7006",
                    "original match repeats a variant",
                ));
            }
            seen.push(ordinal);
            match (&arm.binding, payload.as_ref()) {
                (Some(binding), Some(ty)) => {
                    if let Some(parent) = parent {
                        self.match_payload(&binding.text, ty, parent, ordinal)?;
                    } else {
                        let ty = clone_type(ty, self.spent)?;
                        self.bind(&binding.text, ty)?;
                    }
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
            let arm_usage =
                if parent.is_some() && usage == Use::Loan { Use::Loan } else { Use::Move };
            let value = self.expression(arm.value, arm_usage, depth + 1)?;
            if matches!(value, Closed::Borrow(..)) {
                return Err(self.error(
                    self.original.body.expressions[arm.value as usize].span,
                    "ZRYNA-M7007",
                    "a selected arm loan cannot escape as a match result",
                ));
            }
            if let Some(expected) = &result {
                self.expect(&value, expected, arm.span)?;
            } else {
                result = Some(value);
            }
            self.locals.truncate(before.len());
            let selected = self.snapshot()?;
            if let Some(previous) = &joined {
                self.same_original(previous, &selected, arm.span)?;
            }
            joined = Some(selected);
        }
        if expected_count != Some(arms.len()) {
            return Err(self.error(span, "ZRYNA-M7006", "original match must cover all variants"));
        }
        self.restore(&joined.ok_or(Failure::InternalFailure)?)?;
        self.locals.truncate(outer);
        result.ok_or_else(|| self.error(span, "ZRYNA-M7006", "original match has no arms"))
    }
    fn match_variant(
        &mut self,
        arm: &RawMatchArm,
        args: &[Vec<u8>],
        scrutinee: &Closed,
    ) -> Result<(usize, usize, Option<Closed>), Failure> {
        let mut desc = describe(&self.resolver, &arm.type_name.text, args, self.spent)?;
        self.expect(&desc.ty, scrutinee, arm.span)?;
        if desc.structure {
            return Err(reject("original match names a struct"));
        }
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
        charge(self.spent, 1)?;
        let count = desc.members.len();
        Ok((count, ordinal, desc.members.swap_remove(ordinal).1))
    }
}
