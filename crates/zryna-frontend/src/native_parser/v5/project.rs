//! Module inventory accounting and bounded source nesting for protocol v5.

use super::{
    FileParser, Keyword, ParseError, Token, TokenKind, Totals, resource, syntax, unsupported,
};

pub(super) fn parse_module(
    parser: &mut FileParser<'_>,
    path: &str,
    totals: &mut Totals,
) -> Result<syntax::RawSourceUnit, ParseError> {
    let mut imports = Vec::new();
    let mut declarations = Vec::new();
    let mut functions = Vec::new();
    while let Some(token) = parser.current() {
        if token.kind() == TokenKind::Keyword(Keyword::Import)
            && declarations.is_empty()
            && functions.is_empty()
        {
            if imports.len() >= syntax::MAX_IMPORTS_PER_MODULE
                || totals.imports + imports.len() >= syntax::MAX_IMPORTS_PER_PROJECT
            {
                return Err(resource(if imports.len() >= syntax::MAX_IMPORTS_PER_MODULE {
                    "module exceeds the import-declaration limit"
                } else {
                    "project exceeds the import-declaration limit"
                }));
            }
            let import = parser.import()?;
            parser.previous_bindings += import.bindings.len();
            imports.push(import);
            continue;
        }
        if !matches!(
            token.kind(),
            TokenKind::Keyword(Keyword::Interface | Keyword::Export | Keyword::Function)
        ) {
            return Err(unsupported(Some(token), "unsupported protocol-v5 top-level syntax"));
        }
        if token.kind() == TokenKind::Keyword(Keyword::Function)
            || (token.kind() == TokenKind::Keyword(Keyword::Export)
                && parser
                    .next()
                    .is_some_and(|next| next.kind() == TokenKind::Keyword(Keyword::Function)))
        {
            if functions.len() >= syntax::MAX_FUNCTIONS_PER_MODULE
                || totals.functions + functions.len() >= syntax::MAX_FUNCTIONS_PER_PROJECT
            {
                return Err(resource(if functions.len() >= syntax::MAX_FUNCTIONS_PER_MODULE {
                    "module exceeds the function limit"
                } else {
                    "project exceeds the function limit"
                }));
            }
            let function = parser.function()?;
            parser.previous_parameters += function.parameters.len();
            parser.previous_blocks += function.body.blocks.len();
            parser.previous_statements += function.body.statements.len();
            parser.previous_expressions += function.body.expressions.len();
            let locals = function
                .body
                .statements
                .iter()
                .filter(|statement| {
                    matches!(statement.kind, syntax::RawStatementKind::LocalDeclaration { .. })
                })
                .count();
            parser.previous_locals += locals;
            totals.parameters += function.parameters.len();
            totals.blocks += function.body.blocks.len();
            totals.statements += function.body.statements.len();
            totals.expressions += function.body.expressions.len();
            totals.locals += locals;
            functions.push(function);
            continue;
        }
        if declarations.len() >= syntax::MAX_DATA_DECLARATIONS_PER_MODULE {
            return Err(resource("module exceeds the nominal-declaration limit"));
        }
        let declaration = parser.data_declaration(totals.declarations + declarations.len())?;
        totals.members += match &declaration.kind {
            syntax::RawDataDeclarationKind::Struct { fields, .. } => fields.len(),
            syntax::RawDataDeclarationKind::Enum { variants, .. } => variants.len(),
        };
        parser.previous_members = totals.members;
        declarations.push(declaration);
    }
    totals.declarations += declarations.len();
    totals.functions += functions.len();
    totals.types += parser.types.len();
    totals.imports += imports.len();
    totals.bindings = parser.previous_bindings;
    totals.aggregate_operands = parser.aggregate_operands;
    totals.match_arms = parser.match_arms;
    Ok(syntax::RawSourceUnit {
        id: parser.file,
        path: path.to_owned(),
        imports,
        type_syntax: std::mem::take(&mut parser.types),
        data_declarations: declarations,
        functions,
    })
}

pub(super) fn enforce_source_nesting(tokens: &[Token]) -> Result<(), ParseError> {
    let mut depth = 0_u32;
    for token in tokens {
        match token.kind() {
            TokenKind::OpenBrace | TokenKind::OpenBracket | TokenKind::OpenParen => {
                depth += 1;
                if depth > syntax::MAX_NESTING_DEPTH {
                    return Err(resource("source exceeds the nesting limit"));
                }
            }
            TokenKind::CloseBrace | TokenKind::CloseBracket | TokenKind::CloseParen => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    Ok(())
}
