//! Authenticated source context and lexical identifier role restrictions.

use super::{Cursor, DeclarationError, RawIdentifierSyntax, RawSourceUnit, SourceMap, arena};

#[derive(Clone, Copy)]
pub(in crate::v5) enum Role {
    FunctionBinding,
    DataBinding,
    FunctionTypeParameter,
    DataTypeParameter,
    ValueBinding,
    LocalBinding,
    Runtime,
    Assignment,
    TypeHead,
    ApplicationHead,
}

#[derive(Clone, Copy)]
pub(in crate::v5) struct Context {
    external_module: bool,
    in_function: bool,
}

fn reserved_runtime(text: &str) -> bool {
    matches!(
        text,
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
    )
}

fn strict_word(text: &str) -> bool {
    matches!(
        text,
        "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
    )
}

fn type_diversion(text: &str) -> bool {
    matches!(
        text,
        "any"
            | "unknown"
            | "string"
            | "number"
            | "bigint"
            | "symbol"
            | "boolean"
            | "undefined"
            | "never"
            | "object"
            | "true"
            | "false"
            | "null"
            | "void"
            | "this"
            | "typeof"
            | "import"
            | "new"
            | "keyof"
            | "unique"
            | "readonly"
            | "infer"
    )
}

impl Context {
    /// Prefix facts authenticate module evidence; complete inventory/EOF coverage is still required.
    pub(in crate::v5) fn authenticate(
        sources: &SourceMap,
        unit: &RawSourceUnit,
    ) -> Result<Self, DeclarationError> {
        let mut external_module = false;
        for import in &unit.imports {
            if import.span.file != unit.id || import.span.start != import.import_span.start {
                return Err(arena::malformed());
            }
            Cursor::new(sources, import.span)?.token(import.import_span, "import")?;
            external_module = true;
        }
        for (span, export) in unit
            .data_declarations
            .iter()
            .map(|data| (data.span, data.export_span))
            .chain(unit.functions.iter().map(|function| (function.span, function.export_span)))
        {
            if let Some(export) = export {
                if span.file != unit.id || span.start != export.start {
                    return Err(arena::malformed());
                }
                Cursor::new(sources, span)?.token(export, "export")?;
                external_module = true;
            }
        }
        Ok(Self { external_module, in_function: false })
    }

    pub(in crate::v5) const fn function(self) -> Self {
        Self { in_function: true, ..self }
    }

    pub(in crate::v5) fn allows(self, text: &str, role: Role) -> bool {
        if matches!(role, Role::TypeHead | Role::ApplicationHead) {
            return !(type_diversion(text)
                || matches!(role, Role::ApplicationHead) && text == "function"
                || self.external_module
                    && (strict_word(text) || !self.in_function && text == "await"));
        }
        if reserved_runtime(text) || (self.external_module && strict_word(text)) {
            return false;
        }
        match role {
            Role::FunctionBinding => {
                !(self.external_module && matches!(text, "await" | "eval" | "arguments"))
            }
            Role::DataBinding | Role::DataTypeParameter => {
                !(self.external_module && text == "await")
            }
            Role::FunctionTypeParameter | Role::Runtime => true,
            Role::ValueBinding | Role::Assignment => {
                !(self.external_module && matches!(text, "eval" | "arguments"))
            }
            Role::LocalBinding => {
                text != "let" && !(self.external_module && matches!(text, "eval" | "arguments"))
            }
            Role::TypeHead | Role::ApplicationHead => unreachable!(),
        }
    }

    pub(in crate::v5) fn identifier(
        self,
        cursor: &mut Cursor<'_>,
        name: &RawIdentifierSyntax,
        role: Role,
    ) -> Result<(), DeclarationError> {
        cursor.identifier(name)?;
        if !self.allows(&name.text, role) {
            return Err(DeclarationError::malformed(Some(cursor.bound(name.span)?)));
        }
        Ok(())
    }
}
