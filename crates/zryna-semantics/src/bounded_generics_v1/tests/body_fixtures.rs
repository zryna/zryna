//! Independent fixture reader: complete source first, raw DTO second, production v5 verification
//! last. It has no semantic type/name resolution and cannot manufacture a verified snapshot.

use serde_json::{Value, json};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{RawProjectSyntaxSnapshot, verify_snapshot};

pub(in crate::bounded_generics_v1) struct Project {
    pub(in crate::bounded_generics_v1) sources: SourceMap,
    pub(in crate::bounded_generics_v1) syntax: zryna_syntax::v5::VerifiedProjectSyntaxV5,
}

#[path = "body_fixture_declarations.rs"]
mod declarations;
mod expressions;

pub(in crate::bounded_generics_v1) fn many_owners() -> Project {
    declarations::many_owners()
}

type Token = (String, u32, u32);

struct Reader {
    file: u32,
    tokens: Vec<Token>,
    cursor: usize,
    types: Vec<Value>,
    expressions: Vec<Value>,
    statements: Vec<Value>,
    blocks: Vec<Value>,
}

impl Reader {
    fn new(file: u32, source: &str) -> Self {
        let mut tokens = Vec::new();
        let mut index = 0;
        let bytes = source.as_bytes();
        while index < bytes.len() {
            if bytes[index].is_ascii_whitespace() {
                index += 1;
                continue;
            }
            let start = index;
            if bytes[index] == b'"' {
                index += 1;
                while bytes[index] != b'"' {
                    if bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += 1;
                }
                index += 1;
            } else if bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_' {
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
            } else {
                let operator = ["===", "!==", "=>", "<=", ">="]
                    .into_iter()
                    .find(|operator| source[index..].starts_with(operator));
                index += operator.map_or(1, str::len);
            }
            tokens.push((
                source[start..index].into(),
                u32::try_from(start).expect("fixture offset"),
                u32::try_from(index).expect("fixture index"),
            ));
        }
        Self {
            file,
            tokens,
            cursor: 0,
            types: vec![],
            expressions: vec![],
            statements: vec![],
            blocks: vec![],
        }
    }

    fn peek(&self) -> &str {
        self.tokens.get(self.cursor).map_or("", |token| token.0.as_str())
    }
    fn start(&self) -> u32 {
        self.tokens[self.cursor].1
    }
    fn end(&self) -> u32 {
        self.tokens[self.cursor - 1].2
    }
    fn span(&self, start: u32, end: u32) -> Value {
        json!({"file":self.file,"start":start,"end":end})
    }
    fn take(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }
    fn token(&mut self, expected: &str) -> Value {
        let (text, start, end) = self.take();
        assert_eq!(text, expected, "fixture token {start}");
        self.span(start, end)
    }
    fn identifier(&mut self) -> Value {
        let (text, start, end) = self.take();
        json!({"text":text,"span":self.span(start,end)})
    }
    fn emit(&mut self, start: u32, kind: Value) -> u32 {
        let index = u32::try_from(self.expressions.len()).expect("fixture arena");
        let mut expression = json!({"span":self.span(start,self.end())});
        expression["kind"] = kind;
        self.expressions.push(expression);
        index
    }
    fn block(&mut self) -> u32 {
        let index = u32::try_from(self.blocks.len()).expect("fixture arena");
        self.blocks.push(Value::Null);
        let start = self.start();
        let open_brace_span = self.token("{");
        let mut statements = vec![];
        while self.peek() != "}" {
            statements.push(self.statement());
        }
        let close_brace_span = self.token("}");
        self.blocks[index as usize] = json!({"span":self.span(start,self.end()),"open_brace_span":open_brace_span,"statements":statements,"close_brace_span":close_brace_span});
        index
    }
    fn statement(&mut self) -> u32 {
        let index = u32::try_from(self.statements.len()).expect("fixture arena");
        self.statements.push(Value::Null);
        let start = self.start();
        let kind = match self.peek() {
            "const" | "let" => {
                let mutable = self.peek() == "let";
                let keyword_span = self.token(if mutable { "let" } else { "const" });
                let name = self.identifier();
                let type_syntax = if self.peek() == ":" {
                    self.token(":");
                    self.ty()
                } else {
                    let index = u32::try_from(self.types.len()).expect("fixture arena");
                    self.types.push(json!({"span":self.span(self.start(),self.start()),"kind":{"kind":"missing"}}));
                    index
                };
                let equals_span = self.token("=");
                let initializer = self.expression(1);
                let semicolon_span = self.token(";");
                json!({"kind":"local-declaration","keyword_span":keyword_span,"mutable":mutable,"name":name,"type_syntax":type_syntax,"equals_span":equals_span,"initializer":initializer,"semicolon_span":semicolon_span})
            }
            "return" => {
                let keyword_span = self.token("return");
                let value = self.expression(1);
                let semicolon_span = self.token(";");
                json!({"kind":"return","keyword_span":keyword_span,"value":value,"semicolon_span":semicolon_span})
            }
            "{" => {
                let block = self.block();
                json!({"kind":"block","block":block})
            }
            "if" | "while" => {
                let looped = self.peek() == "while";
                let keyword_span = self.token(if looped { "while" } else { "if" });
                let open_paren_span = self.token("(");
                let condition = self.expression(1);
                let close_paren_span = self.token(")");
                let block = self.block();
                if looped {
                    json!({"kind":"while","keyword_span":keyword_span,"open_paren_span":open_paren_span,"condition":condition,"close_paren_span":close_paren_span,"body_block":block})
                } else {
                    let else_clause = if self.peek() == "else" {
                        let keyword_span = self.token("else");
                        let block = self.block();
                        json!({"keyword_span":keyword_span,"block":block})
                    } else {
                        Value::Null
                    };
                    json!({"kind":"if","keyword_span":keyword_span,"open_paren_span":open_paren_span,"condition":condition,"close_paren_span":close_paren_span,"then_block":block,"else_clause":else_clause})
                }
            }
            "upgradeWeak" => {
                let keyword_span = self.token("upgradeWeak");
                self.token("(");
                let weak = self.expression(1);
                self.token(",");
                self.token("(");
                let binding = self.identifier();
                self.token(")");
                let as_span = self.token("=>");
                let success_block = self.block();
                self.token(",");
                self.token("(");
                self.token(")");
                let else_span = self.token("=>");
                let failure_block = self.block();
                self.token(")");
                self.token(";");
                json!({"kind":"weak-upgrade","keyword_span":keyword_span,"weak":weak,"as_span":as_span,"binding":binding,"success_block":success_block,"else_span":else_span,"failure_block":failure_block})
            }
            _ => {
                let expression = self.expression(1);
                if self.peek() == "=" {
                    let equals_span = self.token("=");
                    let value = self.expression(1);
                    let semicolon_span = self.token(";");
                    json!({"kind":"assignment","target":expression,"equals_span":equals_span,"value":value,"semicolon_span":semicolon_span})
                } else {
                    let semicolon_span = self.token(";");
                    json!({"kind":"expression-statement","expression":expression,"semicolon_span":semicolon_span})
                }
            }
        };
        self.statements[index as usize] = json!({"span":self.span(start,self.end()),"kind":kind});
        index
    }
}

pub(in crate::bounded_generics_v1) fn project(files: &[(&str, &str)]) -> Project {
    let mut files = files.to_vec();
    files.sort_unstable_by_key(|(path, _)| *path);
    let sources = SourceMap::build(
        files
            .iter()
            .map(|(path, text)| SourceFileInput { path: (*path).into(), text: (*text).into() })
            .collect(),
    )
    .expect("original fixture invariant");
    // Consume one unit's temporary JSON before constructing the next. The large owner fixture
    // must retain its exact source counts without keeping a second whole-project syntax tree.
    let raw = snapshot(&files);
    let syntax =
        verify_snapshot(raw, &sources).expect("independent complete original v5 verification");
    Project { sources, syntax }
}

pub(in crate::bounded_generics_v1) fn snapshot(files: &[(&str, &str)]) -> RawProjectSyntaxSnapshot {
    RawProjectSyntaxSnapshot {
        schema_version: 5,
        diagnostics: vec![],
        files: files
            .iter()
            .enumerate()
            .map(|(id, (path, text))| {
                serde_json::from_value(
                    Reader::new(u32::try_from(id).expect("fixture file"), text).unit(path),
                )
                .expect("original fixture invariant")
            })
            .collect(),
    }
}
