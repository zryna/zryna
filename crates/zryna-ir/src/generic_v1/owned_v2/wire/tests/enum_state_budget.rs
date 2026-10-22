//! Genuine source amplification: lexical state copied by match must remain bounded.
use crate::generic_v1::{Failure, owned_v2};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};
use zryna_syntax::{v4, v5};

fn at(base: usize, start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: u32::try_from(base + start).expect("small source"),
        end: u32::try_from(base + end).expect("small source"),
    }
}

fn token(base: usize, text: &str, spelling: &str, occurrence: usize) -> UntrustedSpan {
    let start = text.match_indices(spelling).nth(occurrence).expect("literal source token").0;
    at(base, start, start + spelling.len())
}

fn name(text: &str, span: UntrustedSpan) -> v4::RawIdentifierSyntax {
    v4::RawIdentifierSyntax { text: text.into(), span }
}

fn scalar(types: &mut Vec<v5::RawTypeSyntax>, span: UntrustedSpan) -> u32 {
    let id = u32::try_from(types.len()).expect("type count");
    types.push(v5::RawTypeSyntax {
        span,
        kind: v5::RawTypeSyntaxKind::Named { name: name("i32", span) },
    });
    id
}

fn expression(
    expressions: &mut Vec<v5::RawExpressionSyntax>,
    span: UntrustedSpan,
    kind: v5::RawExpressionKind,
) -> u32 {
    let id = u32::try_from(expressions.len()).expect("expression count");
    expressions.push(v5::RawExpressionSyntax { span, kind });
    id
}

fn body(
    open: UntrustedSpan,
    close: UntrustedSpan,
    statements: Vec<v4::RawStatementSyntax>,
    expressions: Vec<v5::RawExpressionSyntax>,
) -> v5::RawFunctionBodySyntax {
    let span = UntrustedSpan { end: close.end, ..open };
    let ids = (0..u32::try_from(statements.len()).expect("statement count")).collect();
    v5::RawFunctionBodySyntax {
        span,
        root_block: 0,
        blocks: vec![v4::RawBlockSyntax {
            span,
            open_brace_span: open,
            statements: ids,
            close_brace_span: close,
        }],
        statements,
        expressions,
    }
}

// This fixture writes the original program and its literal token claims together. The public
// v5 verifier still authenticates every source byte, type occurrence and expression/statement
// arena. No internal Builder, Budget, synthetic SourceMap or fabricated IR body is used.
#[allow(
    clippy::too_many_lines,
    reason = "The independent literal DTO fixture retains complete token spans and canonical arena order."
)]
fn authenticated(matches: usize) -> (SourceMap, v5::VerifiedProjectSyntaxV5) {
    let header = "function churn(value:Option<i32>):i32 {\n";
    let mut source = header.to_owned();
    let mut types = Vec::new();
    let argument = scalar(&mut types, token(0, header, "i32", 0));
    let option = token(0, header, "Option<i32>", 0);
    types.push(v5::RawTypeSyntax {
        span: option,
        kind: v5::RawTypeSyntaxKind::Application {
            name: name("Option", token(0, header, "Option", 0)),
            type_arguments: v5::RawTypeArgumentList {
                span: token(0, header, "<i32>", 0),
                less_than_span: token(0, header, "<", 0),
                arguments: vec![argument],
                comma_spans: vec![],
                greater_than_span: token(0, header, ">", 0),
            },
        },
    });
    let result_type = scalar(&mut types, token(0, header, "i32", 1));
    let mut statements = Vec::new();
    let mut expressions = Vec::new();
    for index in 0..400 {
        let local = format!("local{index}");
        let text = format!("const {local}:i32=0;\n");
        let base = source.len();
        source.push_str(&text);
        let ty = scalar(&mut types, token(base, &text, "i32", 0));
        let value_span = token(base, &text, "=0", 0);
        let initializer = expression(
            &mut expressions,
            UntrustedSpan { start: value_span.start + 1, ..value_span },
            v5::RawExpressionKind::I32Literal { spelling: "0".into() },
        );
        statements.push(v4::RawStatementSyntax {
            span: at(base, 0, text.len() - 1),
            kind: v4::RawStatementKind::LocalDeclaration {
                keyword_span: token(base, &text, "const", 0),
                mutable: false,
                name: name(&local, token(base, &text, &local, 0)),
                type_syntax: ty,
                equals_span: token(base, &text, "=", 0),
                initializer,
                semicolon_span: token(base, &text, ";", 0),
            },
        });
    }
    for _ in 0..matches {
        let text = "match(value,{\"Option.none\":()=>0,\"Option.some\":(item)=>item});\n";
        let base = source.len();
        source.push_str(text);
        let scrutinee_span = token(base, text, "value", 0);
        let scrutinee = expression(
            &mut expressions,
            scrutinee_span,
            v5::RawExpressionKind::Reference { name: name("value", scrutinee_span) },
        );
        let none_span = token(base, text, "0", 0);
        let none = expression(
            &mut expressions,
            none_span,
            v5::RawExpressionKind::I32Literal { spelling: "0".into() },
        );
        let some_span = token(base, text, "item", 1);
        let some = expression(
            &mut expressions,
            some_span,
            v5::RawExpressionKind::Reference { name: name("item", some_span) },
        );
        let arms = [("none", none, none_span), ("some", some, some_span)]
            .into_iter()
            .enumerate()
            .map(|(ordinal, (variant, value, end))| {
                let family = token(base, text, "Option", ordinal);
                v4::RawMatchArm {
                    span: UntrustedSpan { start: family.start - 1, end: end.end, ..family },
                    type_name: name("Option", family),
                    dot_span: token(base, text, ".", ordinal),
                    variant: name(variant, token(base, text, variant, 0)),
                    binding: (ordinal == 1).then(|| name("item", token(base, text, "item", 0))),
                    arrow_span: token(base, text, "=>", ordinal),
                    value,
                }
            })
            .collect();
        let close = token(base, text, "});", 0);
        let close_paren_span =
            UntrustedSpan { start: close.start + 1, end: close.start + 2, ..close };
        let selection = expression(
            &mut expressions,
            at(base, 0, text.len() - 2),
            v5::RawExpressionKind::Match {
                keyword_span: token(base, text, "match", 0),
                open_paren_span: token(base, text, "(", 0),
                scrutinee,
                close_paren_span,
                open_brace_span: token(base, text, "{", 0),
                arms,
                close_brace_span: token(base, text, "}", 0),
            },
        );
        statements.push(v4::RawStatementSyntax {
            span: at(base, 0, text.len() - 1),
            kind: v4::RawStatementKind::ExpressionStatement {
                expression: selection,
                semicolon_span: token(base, text, ";", 0),
            },
        });
    }
    let tail = "return 0;\n}\n";
    let base = source.len();
    source.push_str(tail);
    let value = expression(
        &mut expressions,
        token(base, tail, "0", 0),
        v5::RawExpressionKind::I32Literal { spelling: "0".into() },
    );
    statements.push(v4::RawStatementSyntax {
        span: at(base, 0, "return 0;".len()),
        kind: v4::RawStatementKind::Return {
            keyword_span: token(base, tail, "return", 0),
            value,
            semicolon_span: token(base, tail, ";", 0),
        },
    });
    let close = token(base, tail, "}", 0);
    let churn = v5::RawFunctionSyntax {
        span: at(0, 0, close.end as usize),
        export_span: None,
        function_span: token(0, header, "function", 0),
        name: name("churn", token(0, header, "churn", 0)),
        type_parameters: None,
        parameters: vec![v5::RawParameterSyntax {
            span: UntrustedSpan {
                start: token(0, header, "value", 0).start,
                end: option.end,
                ..option
            },
            name: name("value", token(0, header, "value", 0)),
            type_syntax: 1,
        }],
        result_type,
        body: body(token(0, header, "{", 0), close, statements, expressions),
    };
    let text = "export function root(input:i32):i32 {return input;}\n";
    let base = source.len();
    source.push_str(text);
    let parameter_type = scalar(&mut types, token(base, text, "i32", 0));
    let result_type = scalar(&mut types, token(base, text, "i32", 1));
    let input = token(base, text, "input", 1);
    let statements = vec![v4::RawStatementSyntax {
        span: at(
            base,
            text.find("return").expect("return"),
            text.find(';').expect("semicolon") + 1,
        ),
        kind: v4::RawStatementKind::Return {
            keyword_span: token(base, text, "return", 0),
            value: 0,
            semicolon_span: token(base, text, ";", 0),
        },
    }];
    let root = v5::RawFunctionSyntax {
        span: at(base, 0, text.len() - 1),
        export_span: Some(token(base, text, "export", 0)),
        function_span: token(base, text, "function", 0),
        name: name("root", token(base, text, "root", 0)),
        type_parameters: None,
        parameters: vec![v5::RawParameterSyntax {
            span: token(base, text, "input:i32", 0),
            name: name("input", token(base, text, "input", 0)),
            type_syntax: parameter_type,
        }],
        result_type,
        body: body(
            token(base, text, "{", 0),
            token(base, text, "}", 0),
            statements,
            vec![v5::RawExpressionSyntax {
                span: input,
                kind: v5::RawExpressionKind::Reference { name: name("input", input) },
            }],
        ),
    };
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
        .expect("complete original source");
    let syntax = v5::verify_snapshot(
        v5::RawProjectSyntaxSnapshot {
            schema_version: 5,
            files: vec![v5::RawSourceUnit {
                id: 0,
                path: "main.zry".into(),
                imports: vec![],
                type_syntax: types,
                data_declarations: vec![],
                functions: vec![churn, root],
            }],
            diagnostics: vec![],
        },
        &sources,
    )
    .expect("independent complete v5 source authentication");
    (sources, syntax)
}

fn layouts(sources: &SourceMap) -> (layout::VerifiedLayouts, layout::VerifiedLayouts) {
    use layout::raw::{Graph, Module, ModuleId, NodeId, TypeKind, TypeNode};
    let graph = Graph {
        modules: vec![Module {
            id: ModuleId(0),
            source_file: sources.verify_file_id(0).expect("file"),
            data_declarations: 0,
        }],
        declarations: vec![],
        types: vec![
            TypeNode {
                id: NodeId(0),
                span: None,
                kind: TypeKind::Base(zryna_layout::raw::TypeKind::Bool),
            },
            TypeNode {
                id: NodeId(1),
                span: None,
                kind: TypeKind::Base(zryna_layout::raw::TypeKind::I32),
            },
            TypeNode {
                id: NodeId(2),
                span: None,
                kind: TypeKind::Base(zryna_layout::raw::TypeKind::String),
            },
            TypeNode { id: NodeId(3), span: None, kind: TypeKind::Option { argument: NodeId(1) } },
        ],
        program_roots: vec![NodeId(3)],
    };
    (
        layout::verify(&graph, sources, StorageTarget::Linear32V1)
            .expect("independent Linear32 layout"),
        layout::verify(&graph, sources, StorageTarget::LinuxX8664V1)
            .expect("independent Linux layout"),
    )
}

fn accepted_source_wire() -> Vec<u8> {
    let (sources, syntax) = authenticated(1);
    let (linear, linux) = layouts(&sources);
    let claim = owned_v2::produce_claim(&syntax, &sources, &linear, &linux, &[])
        .expect("small source crosses the public producer");
    let bytes = owned_v2::wire::encode(&claim).expect("bounded public wire");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("runtime claims"),
        &linear,
        &linux,
    )
    .expect("runtime declaration authority");
    owned_v2::verify(
        owned_v2::wire::decode(&bytes).expect("public hostile-wire decoder"),
        &syntax,
        &sources,
        sources.verify_file_id(0).expect("entry"),
        &linear,
        &linux,
        &runtime,
    )
    .expect("small source independently seals its complete type/owner/cleanup claim");
    bytes
}

#[test]
fn repeated_match_binding_copies_exceed_source_state_budget_before_claim_sealing() {
    let pristine = accepted_source_wire();
    // Each match saves 400 scalar bindings and restores them into both arms. Counting only
    // those three copies, each binding and its one-byte type key already costs 2 units:
    // 440 * 3 * 400 * 2 = 1,056,000 > the unchanged 1,048,576-state-unit ceiling.
    // Availability vectors, the enum parameter, loans and merge state add further work.
    // These 440 matches remain below independent source/IR value, block and wire ceilings.
    for _ in 0..2 {
        let (sources, syntax) = authenticated(440);
        let (linear, linux) = layouts(&sources);
        let Err(failure) = owned_v2::produce_claim(&syntax, &sources, &linear, &linux, &[]) else {
            panic!("match snapshots published an over-budget raw claim");
        };
        let Failure::Diagnostics(errors) = failure else {
            panic!("expected deterministic source-state budget rejection");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-I3201");
        assert_eq!(errors[0].message(), "owned source specialization aggregate ceiling");
    }
    assert_eq!(accepted_source_wire(), pristine, "rejection must leave pristine replay unchanged");
}
