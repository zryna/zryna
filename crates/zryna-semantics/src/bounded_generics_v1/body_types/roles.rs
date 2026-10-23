//! Opt-in original execution roles; symbolic source capabilities are never layout facts.
use zryna_source::UntrustedSpan;

use super::super::DeclarationIdentity;
use super::capabilities::Status;
use super::model::{Kind, Ty};
use super::{BodyTypeFailure, Checker, Mode, substitution};

pub(super) fn source_failure(failure: zryna_ir::generic_v1::Failure) -> BodyTypeFailure {
    match failure {
        zryna_ir::generic_v1::Failure::Diagnostics(errors) => BodyTypeFailure::Diagnostics(errors),
        zryna_ir::generic_v1::Failure::AllocationFailure => BodyTypeFailure::AllocationFailure,
        zryna_ir::generic_v1::Failure::InternalFailure => BodyTypeFailure::InternalFailure,
    }
}

/// Observe one referent without claiming its Copy capability or changing its loan identity.
/// The independent original checker owns availability, access, overlap and lexical lifetime.
pub(super) fn observe(
    checker: &Checker<'_, '_>,
    owner: DeclarationIdentity,
    ty: Option<Ty>,
) -> Result<Option<Ty>, BodyTypeFailure> {
    if checker.mode == Mode::Legacy {
        return Ok(ty);
    }
    let Some(ty) = ty else { return Ok(None) };
    let Some(head) = substitution::head(&checker.tables, owner, ty)? else { return Ok(None) };
    if matches!(head.kind, Kind::Borrow | Kind::BorrowMut) {
        Ok(Some(head.children[0].ok_or(BodyTypeFailure::InternalFailure)?))
    } else {
        Ok(Some(ty))
    }
}

/// A value read through a loan is an original universal Copy query, not a closed-type query.
pub(super) fn value(
    checker: &mut Checker<'_, '_>,
    owner: DeclarationIdentity,
    ty: Option<Ty>,
    at: UntrustedSpan,
) -> Result<Option<Ty>, BodyTypeFailure> {
    if checker.mode == Mode::Legacy {
        return Ok(ty);
    }
    let Some(original) = ty else { return Ok(None) };
    let referent =
        observe(checker, owner, Some(original))?.ok_or(BodyTypeFailure::InternalFailure)?;
    if referent == original {
        return Ok(Some(original));
    }
    match checker.capabilities.query(&checker.tables, owner, referent, false, false)? {
        Status::True => Ok(Some(referent)),
        Status::Unknown => Ok(None),
        Status::False => {
            let span = checker.span(at);
            checker.constraints.at(
                "ZRYNA-M7007",
                span,
                "an affine referent cannot be read as an implicit owned value",
                "observe the loan or explicitly clone a referent with original Clone capability",
            );
            Ok(None)
        }
    }
}

/// Borrow formals compare the loan wrapper exactly; stored formals consume value reads.
pub(super) fn argument(
    checker: &mut Checker<'_, '_>,
    owner: DeclarationIdentity,
    expected: Ty,
    actual: Option<Ty>,
    at: UntrustedSpan,
) -> Result<Option<Ty>, BodyTypeFailure> {
    if checker.mode == Mode::Legacy {
        return Ok(actual);
    }
    let Some(head) = substitution::head(&checker.tables, owner, expected)? else { return Ok(None) };
    if matches!(head.kind, Kind::Borrow | Kind::BorrowMut) {
        Ok(actual)
    } else {
        value(checker, owner, actual, at)
    }
}
