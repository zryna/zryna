//! Source token consumption for calls, ownership builtins, and literals.

use super::{
    Context, Cursor, DeclarationError, Kind, RawExpressionSyntax, RawFunctionBodySyntax,
    RawSourceUnit, Role, UntrustedSpan, arena, child,
};

pub(super) fn call(
    cursor: &mut Cursor<'_>,
    unit: &RawSourceUnit,
    body: &RawFunctionBodySyntax,
    kind: &Kind,
    context: Context,
) -> Result<(), DeclarationError> {
    let Kind::Call { callee, type_arguments, open_paren_span, arguments, close_paren_span } = kind
    else {
        return Err(arena::malformed());
    };
    if matches!(
        callee.text.as_str(),
        "match"
            | "upgradeWeak"
            | "clone"
            | "shared"
            | "downgrade"
            | "borrow"
            | "borrowMut"
            | "push"
            | "Vec"
            | "FixedArray"
    ) {
        return Err(arena::malformed());
    }
    context.identifier(cursor, callee, Role::Runtime)?;
    cursor.type_arguments(unit, type_arguments.as_ref())?;
    cursor.token(*open_paren_span, "(")?;
    for (index, id) in arguments.iter().enumerate() {
        if index != 0 {
            cursor.punctuation(",")?;
        }
        child(cursor, body, *id)?;
    }
    if !arguments.is_empty() {
        cursor.comma()?;
    }
    cursor.token(*close_paren_span, ")")?;
    Ok(())
}

pub(super) fn unary(
    cursor: &mut Cursor<'_>,
    body: &RawFunctionBodySyntax,
    kind: &Kind,
    keyword: UntrustedSpan,
    open: UntrustedSpan,
    value: u32,
    close: UntrustedSpan,
) -> Result<(), DeclarationError> {
    let spelling = match kind {
        Kind::Clone { .. } => "clone",
        Kind::Shared { .. } => "shared",
        Kind::Downgrade { .. } => "downgrade",
        Kind::Borrow { .. } => "borrow",
        _ => "borrowMut",
    };
    cursor.token(keyword, spelling)?;
    cursor.token(open, "(")?;
    child(cursor, body, value)?;
    cursor.token(close, ")")?;
    Ok(())
}

pub(super) fn literal(
    cursor: &mut Cursor<'_>,
    expression: &RawExpressionSyntax,
    spelling: &str,
) -> Result<(), DeclarationError> {
    cursor.token(expression.span, spelling)?;
    let kind = if matches!(expression.kind, Kind::I32Literal { .. }) {
        "i32-literal"
    } else {
        "string-literal"
    };
    if !super::super::wire::valid(&serde_json::json!({ "kind": kind, "spelling": spelling })) {
        return Err(arena::malformed());
    }
    Ok(())
}
