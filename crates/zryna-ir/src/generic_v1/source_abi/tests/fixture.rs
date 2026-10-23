//! Literal source/DTO pair for identity functions; no frontend or body producer supplies syntax.
use crate::generic_v1::{Failure, copy_v1, owned_v2};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};
use zryna_syntax::{v4, v5};

pub(in crate::generic_v1) struct Fixture {
    pub sources: SourceMap,
    pub syntax: v5::VerifiedProjectSyntaxV5,
    pub linear: layout::VerifiedLayouts,
    pub linux: layout::VerifiedLayouts,
}

fn span(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: start.try_into().expect("small source"),
        end: end.try_into().expect("small source"),
    }
}

fn ident(text: &str, start: usize) -> v4::RawIdentifierSyntax {
    v4::RawIdentifierSyntax { text: text.into(), span: span(start, start + text.len()) }
}

fn ty(nodes: &mut Vec<v5::RawTypeSyntax>, spelling: &str, start: usize) -> u32 {
    let kind = match spelling {
        "String" => v5::RawTypeSyntaxKind::String { keyword_span: span(start, start + 6) },
        "Option<i32>" => {
            let argument = ty(nodes, "i32", start + 7);
            v5::RawTypeSyntaxKind::Application {
                name: ident("Option", start),
                type_arguments: v5::RawTypeArgumentList {
                    span: span(start + 6, start + 11),
                    less_than_span: span(start + 6, start + 7),
                    arguments: vec![argument],
                    comma_spans: vec![],
                    greater_than_span: span(start + 10, start + 11),
                },
            }
        }
        _ => v5::RawTypeSyntaxKind::Named { name: ident(spelling, start) },
    };
    let id = nodes.len().try_into().expect("small type arena");
    nodes.push(v5::RawTypeSyntax { span: span(start, start + spelling.len()), kind });
    id
}

fn layouts(
    sources: &SourceMap,
    parameter: &str,
    result: &str,
) -> (layout::VerifiedLayouts, layout::VerifiedLayouts) {
    let mut nodes = [
        zryna_layout::raw::TypeKind::Bool,
        zryna_layout::raw::TypeKind::I32,
        zryna_layout::raw::TypeKind::String,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, kind)| layout::raw::TypeNode {
        id: layout::raw::NodeId(i.try_into().expect("primitive index")),
        span: None,
        kind: layout::raw::TypeKind::Base(kind),
    })
    .collect::<Vec<_>>();
    if parameter == "Option<i32>" || result == "Option<i32>" {
        nodes.push(layout::raw::TypeNode {
            id: layout::raw::NodeId(3),
            span: None,
            kind: layout::raw::TypeKind::Option { argument: layout::raw::NodeId(1) },
        });
    }
    let graph = layout::raw::Graph {
        modules: vec![layout::raw::Module {
            id: layout::raw::ModuleId(0),
            source_file: sources.verify_file_id(0).expect("literal file"),
            data_declarations: 0,
        }],
        declarations: vec![],
        program_roots: nodes.iter().map(|node| node.id).collect(),
        types: nodes,
    };
    let linear =
        layout::verify(&graph, sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux =
        layout::verify(&graph, sources, StorageTarget::LinuxX8664V1).expect("linux layouts");
    (linear, linux)
}

impl Fixture {
    pub fn new(parameter: &str, result: &str) -> Self {
        let value = if parameter == result {
            "input"
        } else if result == "String" {
            "\"x\""
        } else {
            "1"
        };
        let source =
            format!("export function value(input:{parameter}):{result}{{return {value};}}");
        let parameter_start = 28;
        let result_start = parameter_start + parameter.len() + 2;
        let open = source.find('{').expect("literal brace");
        let returned = open + 8;
        let semicolon = returned + value.len();
        let end = source.len();
        let mut types = vec![];
        let parameter_type = ty(&mut types, parameter, parameter_start);
        let result_type = ty(&mut types, result, result_start);
        let expression = if value == "input" {
            v5::RawExpressionKind::Reference { name: ident(value, returned) }
        } else if result == "String" {
            v5::RawExpressionKind::StringLiteral { spelling: value.into() }
        } else {
            v5::RawExpressionKind::I32Literal { spelling: value.into() }
        };
        let body_span = span(open, end);
        let function = v5::RawFunctionSyntax {
            span: span(0, end),
            export_span: Some(span(0, 6)),
            function_span: span(7, 15),
            name: ident("value", 16),
            type_parameters: None,
            parameters: vec![v5::RawParameterSyntax {
                span: span(22, parameter_start + parameter.len()),
                name: ident("input", 22),
                type_syntax: parameter_type,
            }],
            result_type,
            body: v5::RawFunctionBodySyntax {
                span: body_span,
                root_block: 0,
                blocks: vec![v4::RawBlockSyntax {
                    span: body_span,
                    open_brace_span: span(open, open + 1),
                    statements: vec![0],
                    close_brace_span: span(end - 1, end),
                }],
                statements: vec![v4::RawStatementSyntax {
                    span: span(open + 1, semicolon + 1),
                    kind: v4::RawStatementKind::Return {
                        keyword_span: span(open + 1, open + 7),
                        value: 0,
                        semicolon_span: span(semicolon, semicolon + 1),
                    },
                }],
                expressions: vec![v5::RawExpressionSyntax {
                    span: span(returned, semicolon),
                    kind: expression,
                }],
            },
        };
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
                .expect("literal source");
        let syntax = v5::verify_snapshot(
            v5::RawProjectSyntaxSnapshot {
                schema_version: 5,
                diagnostics: vec![],
                files: vec![v5::RawSourceUnit {
                    id: 0,
                    path: "main.zry".into(),
                    imports: vec![],
                    type_syntax: types,
                    data_declarations: vec![],
                    functions: vec![function],
                }],
            },
            &sources,
        )
        .expect("independent complete literal syntax");
        let (linear, linux) = layouts(&sources, parameter, result);
        Self { sources, syntax, linear, linux }
    }

    pub fn owned(&self) -> Result<(), Failure> {
        let claim =
            owned_v2::produce_claim(&self.syntax, &self.sources, &self.linear, &self.linux, &[])?;
        let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
            zryna_ownership_runtime_abi::generic_v1::raw_v1(&self.linear, &self.linux)
                .expect("runtime claims"),
            &self.linear,
            &self.linux,
        )
        .expect("runtime authority");
        owned_v2::verify(
            owned_v2::wire::decode(&owned_v2::wire::encode(&claim)?)?,
            &self.syntax,
            &self.sources,
            self.sources.verify_file_id(0).expect("entry"),
            &self.linear,
            &self.linux,
            &runtime,
        )
        .map(|_| ())
    }

    pub fn copy(&self) -> Result<(), Failure> {
        let claim =
            copy_v1::produce_claim(&self.syntax, &self.sources, &self.linear, &self.linux, &[])?;
        let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
            zryna_ownership_runtime_abi::generic_v1::raw_v1(&self.linear, &self.linux)
                .expect("runtime claims"),
            &self.linear,
            &self.linux,
        )
        .expect("runtime authority");
        copy_v1::verify(
            crate::generic_v1::wire::decode(&crate::generic_v1::wire::encode(&claim)?)?,
            &self.syntax,
            &self.sources,
            self.sources.verify_file_id(0).expect("entry"),
            &self.linear,
            &self.linux,
            &runtime,
        )
        .map(|_| ())
    }
}
