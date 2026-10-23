//! Rejection-only match diagnostics retain exact borrowed payload provenance.
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawMatchArm;
use zryna_syntax::v5::RawExpressionKind;

use super::super::model::Origin;
use super::super::{equality, substitution};
use super::{BodyTypeFailure, Checker, DeclarationIdentity, Kind, Ty, resources};

pub(super) fn quoted_key(arm: &RawMatchArm) -> Result<UntrustedSpan, BodyTypeFailure> {
    Ok(UntrustedSpan {
        file: arm.type_name.span.file,
        start: arm.type_name.span.start.checked_sub(1).ok_or(BodyTypeFailure::InternalFailure)?,
        end: arm.variant.span.end.checked_add(1).ok_or(BodyTypeFailure::InternalFailure)?,
    })
}

pub(super) fn borrowed_payload_conversion(
    checker: &mut Checker<'_, '_>,
    owner: DeclarationIdentity,
    expected: Option<Ty>,
    actual: Option<Ty>,
) -> Result<bool, BodyTypeFailure> {
    let (Some(expected), Some(actual)) = (expected, actual) else { return Ok(false) };
    for (borrowed, peer) in [(expected, actual), (actual, expected)] {
        let Some(borrowed) = substitution::normalize(&checker.tables, owner, borrowed)? else {
            continue;
        };
        let Origin::ArmBorrow { function, expression, arm } = borrowed.origin else { continue };
        if function != owner {
            return Err(BodyTypeFailure::InternalFailure);
        }
        let Some(head) = substitution::head(&checker.tables, owner, borrowed)? else { continue };
        if !matches!(head.kind, Kind::Borrow | Kind::BorrowMut) {
            continue;
        }
        let payload = head.children[0].ok_or(BodyTypeFailure::InternalFailure)?;
        if !equality::equal(&checker.tables, owner, payload, peer, &mut checker.equality)? {
            continue;
        }
        let original = resources::raw_function(checker.context, owner);
        let RawExpressionKind::Match { arms, .. } =
            &original.body.expressions[expression as usize].kind
        else {
            return Err(BodyTypeFailure::InternalFailure);
        };
        let arm = arms.get(arm as usize).ok_or(BodyTypeFailure::InternalFailure)?;
        let at = checker.span(original.body.expressions[arm.value as usize].span);
        checker.constraints.at(
            "ZRYNA-M7007",
            at,
            "borrowed match payload cannot become an owned arm result",
            "observe the borrowed payload or explicitly clone it where permitted",
        );
        return Ok(true);
    }
    Ok(false)
}
