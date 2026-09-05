use super::*;
use zryna_source::UntrustedSpan;

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum ReplacementRoot {
    Struct,
    Enum,
    Array,
    Vec,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum ReplacementCase {
    Constructor,
    Repeated,
    Move,
    Immutable,
    Moved,
    WrongType,
    SelfDirect,
    SelfNested,
    SelfCall,
    InvalidLater,
}

#[path = "mixed_root_replacement_builder.rs"]
mod builder;
use builder::{Fixture, at};

pub(in crate::data_ownership_v1) fn replacement_fixture(
    root: ReplacementRoot,
    case: ReplacementCase,
) -> (String, RawProjectSyntaxSnapshot) {
    let (prefix, declarations, types) = match root {
        ReplacementRoot::Struct => {
            let (source, raw) = mixed_construction::mixed_fixture(false);
            let file = &raw.files[0];
            let end = file.data_declarations[0].span.end as usize;
            (
                source[..end].to_owned(),
                file.data_declarations.clone(),
                file.type_syntax[..2].to_vec(),
            )
        }
        ReplacementRoot::Enum => {
            let (source, raw) = nested_enum_fixture();
            let file = &raw.files[0];
            let end = file.data_declarations[0].span.end as usize;
            (
                source.0[..end].to_owned(),
                file.data_declarations.clone(),
                file.type_syntax[..2].to_vec(),
            )
        }
        _ => (String::new(), Vec::new(), Vec::new()),
    };
    let mut f = Fixture { source: prefix, types, expressions: Vec::new(), statements: Vec::new() };
    f.text("\n");
    let function_start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("make");
    f.text("(): ");
    let result_type = f.ty(Some(root));
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    f.local(root, "item", !matches!(case, ReplacementCase::Immutable), false);
    if matches!(case, ReplacementCase::Move | ReplacementCase::Moved) {
        f.local(root, "next", false, matches!(case, ReplacementCase::Moved));
    }
    f.assignment(root, case, matches!(case, ReplacementCase::Repeated));
    if matches!(case, ReplacementCase::Repeated) {
        f.assignment(root, ReplacementCase::Constructor, false);
    }
    let return_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = f.reference("item");
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start as usize, f.source.len());
    let statements = (0..f.statements.len()).map(|id| id.try_into().expect("statement")).collect();
    let function = RawFunctionSyntax {
        span: at(function_start, f.source.len()),
        export_span: None,
        function_span,
        name,
        parameters: Vec::new(),
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                statements,
                close_brace_span,
            }],
            statements: f.statements,
            expressions: f.expressions,
        },
    };
    let mut raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".into(),
            imports: Vec::new(),
            type_syntax: f.types,
            data_declarations: declarations,
            functions: vec![function],
        }],
        diagnostics: Vec::new(),
    };
    if matches!(case, ReplacementCase::SelfCall) {
        append_identity(&mut f.source, &mut raw, root);
    }
    (f.source, raw)
}

fn append_identity(source: &mut String, raw: &mut RawProjectSyntaxSnapshot, root: ReplacementRoot) {
    let mut f = Fixture {
        source: source.clone(),
        types: raw.files[0].type_syntax.clone(),
        expressions: Vec::new(),
        statements: Vec::new(),
    };
    f.text(" ");
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("identity");
    f.text("(");
    let parameter_start = f.source.len();
    let parameter_name = f.name("value");
    f.text(": ");
    let type_syntax = f.ty(Some(root));
    let parameter = RawParameterSyntax {
        span: at(parameter_start, f.source.len()),
        name: parameter_name,
        type_syntax,
    };
    f.text("): ");
    let result_type = f.ty(Some(root));
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let return_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = f.reference("value");
    let semicolon_span = f.text(";");
    let statement = RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    };
    f.text(" ");
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start as usize, f.source.len());
    raw.files[0].functions.push(RawFunctionSyntax {
        span: at(start, f.source.len()),
        export_span: None,
        function_span,
        name,
        parameters: vec![parameter],
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                statements: vec![0],
                close_brace_span,
            }],
            statements: vec![statement],
            expressions: f.expressions,
        },
    });
    raw.files[0].type_syntax = f.types;
    *source = f.source;
}
