//! Closed provisional-symbol spelling and portable collision verification.

use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;

use super::{VerificationErrors, raw};

pub(super) fn verify(module: &raw::Module, errors: &mut VerificationErrors) {
    let mut exact = BTreeMap::<String, usize>::new();
    let mut portable = BTreeMap::<String, usize>::new();
    for (function_index, function) in module.functions.iter().enumerate() {
        if !valid_symbol(&function.symbol) {
            errors.push(Diagnostic::error(
                "ZRYNA-N1002",
                None,
                format!("native function #{function_index} has an invalid provisional symbol"),
                "use bounded ASCII [A-Za-z_][A-Za-z0-9_]* without reserved bindings",
            ));
            continue;
        }
        if let Some(previous) = exact.get(&function.symbol).copied() {
            errors.push(Diagnostic::error(
                "ZRYNA-N1003",
                None,
                format!(
                    "native function #{function_index} duplicates the symbol of function #{previous}"
                ),
                "give every native function one exact unique provisional symbol",
            ));
            continue;
        }
        exact.insert(function.symbol.clone(), function_index);
        let identity = function.symbol.to_ascii_lowercase();
        if let Some(previous) = portable.get(&identity).copied() {
            errors.push(Diagnostic::error(
                "ZRYNA-N1003",
                None,
                format!(
                    "native function #{function_index} collides with function #{previous} under the portable symbol identity"
                ),
                "use symbols that remain unique when ASCII case is ignored",
            ));
        } else {
            portable.insert(identity, function_index);
        }
    }
}

fn valid_symbol(symbol: &str) -> bool {
    let mut bytes = symbol.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !reserved_symbol(symbol)
}

fn reserved_symbol(symbol: &str) -> bool {
    matches!(
        symbol,
        "arguments"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "constructor"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "eval"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "instanceof"
            | "interface"
            | "let"
            | "new"
            | "null"
            | "package"
            | "private"
            | "protected"
            | "prototype"
            | "public"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "then"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
            | "__proto__"
    )
}
