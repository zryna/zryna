use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};
use zryna_syntax::v4::{
    RawBlockSyntax, RawDataField, RawEnumVariant, RawIdentifierSyntax, RawImportBindingSyntax,
    RawImportSyntax, RawModuleSpecifierSyntax, RawStatementKind, RawStatementSyntax,
};
use zryna_syntax::v5::{
    RawDataDeclaration, RawDataDeclarationKind, RawExpressionKind, RawExpressionSyntax,
    RawFunctionBodySyntax, RawFunctionSyntax, RawParameterSyntax, RawProjectSyntaxSnapshot,
    RawSourceUnit, RawTypeParameter, RawTypeParameterList, RawTypeSyntax, RawTypeSyntaxKind,
    VerifiedProjectSyntaxV5, verify_snapshot,
};

pub(super) struct Project {
    pub(super) sources: SourceMap,
    pub(super) syntax: VerifiedProjectSyntaxV5,
}

/// Small source writer for complete independent declaration-context fixtures.
/// Every emitted DTO must pass the production source/arena verifier before semantics is invoked.
pub(super) struct Unit {
    text: String,
    raw: RawSourceUnit,
}

mod functions;

impl Unit {
    fn new(path: &str, id: usize) -> Self {
        Self {
            text: String::new(),
            raw: RawSourceUnit {
                id: u32::try_from(id).expect("fixture module"),
                path: path.into(),
                imports: vec![],
                type_syntax: vec![],
                data_declarations: vec![],
                functions: vec![],
            },
        }
    }

    fn span(&self, start: u32, end: u32) -> UntrustedSpan {
        UntrustedSpan { file: self.raw.id, start, end }
    }

    fn token(&mut self, text: &str) -> UntrustedSpan {
        let start = u32::try_from(self.text.len()).expect("fixture byte offset");
        self.text.push_str(text);
        self.span(start, u32::try_from(self.text.len()).expect("fixture byte offset"))
    }

    fn identifier(&mut self, text: &str) -> RawIdentifierSyntax {
        RawIdentifierSyntax { text: text.into(), span: self.token(text) }
    }

    fn annotation(&mut self, text: &str) -> u32 {
        let name = self.identifier(text);
        let id = u32::try_from(self.raw.type_syntax.len()).expect("fixture type index");
        self.raw
            .type_syntax
            .push(RawTypeSyntax { span: name.span, kind: RawTypeSyntaxKind::Named { name } });
        id
    }

    fn exported(&mut self, exported: bool) -> Option<UntrustedSpan> {
        exported.then(|| {
            let span = self.token("export");
            self.token(" ");
            span
        })
    }

    fn parameters(&mut self, names: &[&str]) -> Option<RawTypeParameterList> {
        if names.is_empty() {
            return None;
        }
        let less_than_span = self.token("<");
        let mut parameters = Vec::new();
        let mut comma_spans = Vec::new();
        for (index, text) in names.iter().enumerate() {
            if index > 0 {
                comma_spans.push(self.token(","));
                self.token(" ");
            }
            let name = self.identifier(text);
            self.token(" ");
            let extends_span = self.token("extends");
            self.token(" ");
            let bound = self.identifier("ZrynaValue");
            parameters.push(RawTypeParameter {
                span: self.span(name.span.start, bound.span.end),
                name,
                extends_span,
                bound,
            });
        }
        let greater_than_span = self.token(">");
        Some(RawTypeParameterList {
            span: self.span(less_than_span.start, greater_than_span.end),
            less_than_span,
            parameters,
            comma_spans,
            greater_than_span,
        })
    }

    pub(super) fn import(&mut self, path: &str, names: &[(&str, &str)]) {
        let import_span = self.token("import");
        self.token(" { ");
        let mut bindings = Vec::new();
        for (index, (original, alias)) in names.iter().enumerate() {
            if index > 0 {
                self.token(", ");
            }
            let imported = self.identifier(original);
            let (local, as_span) = if original == alias {
                (imported.clone(), None)
            } else {
                self.token(" ");
                let as_span = self.token("as");
                self.token(" ");
                (self.identifier(alias), Some(as_span))
            };
            bindings.push(RawImportBindingSyntax {
                span: self.span(imported.span.start, local.span.end),
                imported,
                local,
                as_span,
            });
        }
        self.token(" } ");
        let from_span = self.token("from");
        self.token(" \"");
        let value_span = self.token(path);
        let close = self.token("\"");
        let specifier = RawModuleSpecifierSyntax {
            text: path.into(),
            value_span,
            token_span: self.span(value_span.start - 1, close.end),
        };
        let semicolon_span = self.token(";");
        self.raw.imports.push(RawImportSyntax {
            span: self.span(import_span.start, semicolon_span.end),
            import_span,
            bindings,
            from_span,
            specifier,
            semicolon_span,
        });
        self.token("\n");
    }

    pub(super) fn data(
        &mut self,
        text: &str,
        parameters: &[&str],
        enumeration: bool,
        exported: bool,
    ) {
        let export_span = self.exported(exported);
        let interface_span = self.token("interface");
        self.token(" ");
        let name = self.identifier(text);
        let type_parameters = self.parameters(parameters);
        self.token(" ");
        let extends_span = self.token("extends");
        self.token(" ");
        let marker_span = self.token(if enumeration { "ZrynaEnum" } else { "ZrynaStruct" });
        self.token(" ");
        let open_brace_span = self.token("{");
        self.token(" ");
        let member = self.identifier(if enumeration { "ok" } else { "value" });
        let colon_span = self.token(":");
        self.token(" ");
        let payload = self.annotation(parameters.first().copied().unwrap_or("i32"));
        let semicolon_span = self.token(";");
        let member_span = self.span(member.span.start, semicolon_span.end);
        self.token(" ");
        let close_brace_span = self.token("}");
        let kind = if enumeration {
            RawDataDeclarationKind::Enum {
                interface_span,
                name,
                extends_span,
                marker_span,
                open_brace_span,
                close_brace_span,
                variants: vec![RawEnumVariant {
                    span: member_span,
                    name: member,
                    colon_span,
                    payload_type: Some(payload),
                    none_span: None,
                    semicolon_span,
                }],
            }
        } else {
            RawDataDeclarationKind::Struct {
                interface_span,
                name,
                extends_span,
                marker_span,
                open_brace_span,
                close_brace_span,
                fields: vec![RawDataField {
                    span: member_span,
                    name: member,
                    colon_span,
                    type_syntax: payload,
                    semicolon_span,
                }],
            }
        };
        self.raw.data_declarations.push(RawDataDeclaration {
            span: self.span(export_span.unwrap_or(interface_span).start, close_brace_span.end),
            export_span,
            type_parameters,
            kind,
        });
        self.token("\n");
    }
}

pub(super) fn raw_project(
    paths: &[&str],
    mut write: impl FnMut(&str, &mut Unit),
) -> (SourceMap, RawProjectSyntaxSnapshot) {
    let mut paths = paths.to_vec();
    paths.sort_unstable();
    let mut files = Vec::new();
    let mut sources = Vec::new();
    for (id, path) in paths.into_iter().enumerate() {
        let mut unit = Unit::new(path, id);
        write(path, &mut unit);
        sources.push(SourceFileInput { path: path.into(), text: unit.text });
        files.push(unit.raw);
    }
    (
        SourceMap::build(sources).expect("complete original source fixture"),
        RawProjectSyntaxSnapshot { schema_version: 5, files, diagnostics: vec![] },
    )
}

pub(super) fn project(paths: &[&str], write: impl FnMut(&str, &mut Unit)) -> Project {
    let (sources, raw) = raw_project(paths, write);
    let syntax =
        verify_snapshot(raw, &sources).expect("independent complete source/arena verification");
    Project { sources, syntax }
}

pub(super) fn scalar() -> Project {
    project(&["main.zry"], |_, unit| unit.function("score", &[], true))
}

pub(super) fn reference() -> Project {
    let sources = SourceMap::build(vec![
        SourceFileInput {
            path: "main.zry".into(),
            text: include_str!("../../../../../tests/m7-syntax-fixtures/main.zry").into(),
        },
        SourceFileInput {
            path: "values.zry".into(),
            text: include_str!("../../../../../tests/m7-syntax-fixtures/values.zry").into(),
        },
    ])
    .expect("original independent fixtures");
    let raw = zryna_syntax::v5::decode_snapshot(include_bytes!(
        "../../../../../tests/m7-syntax-fixtures/reference.json"
    ))
    .expect("independent v5 fixture");
    let syntax = verify_snapshot(raw, &sources).expect("complete original syntax");
    Project { sources, syntax }
}
