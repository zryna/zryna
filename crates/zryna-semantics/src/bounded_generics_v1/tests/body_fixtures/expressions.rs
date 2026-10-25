//! Independent expression DTO construction from the original fixture token stream.

use super::{Reader, Value, json};

impl Reader {
    pub(super) fn expression(&mut self, minimum: u8) -> u32 {
        let start = self.start();
        let mut left = self.primary();
        loop {
            let (priority, tag) = match self.peek() {
                "===" => (1, "equal"),
                "!==" => (1, "not-equal"),
                "<" => (2, "less-than"),
                "<=" => (2, "less-equal"),
                ">" => (2, "greater-than"),
                ">=" => (2, "greater-equal"),
                "+" => (3, "addition"),
                "-" => (3, "subtraction"),
                "*" => (4, "multiplication"),
                _ => (0, ""),
            };
            if priority == 0 || priority < minimum {
                break;
            }
            let (_, begin, end) = self.take();
            let operator_span = self.span(begin, end);
            let right = self.expression(priority + 1);
            left = self.emit(
                start,
                json!({"kind":tag,"operator_span":operator_span,"lhs":left,"rhs":right}),
            );
        }
        left
    }
    pub(super) fn primary(&mut self) -> u32 {
        let start = self.start();
        let value = if self.peek() == "-" {
            let operator_span = self.token("-");
            let operand = self.expression(5);
            self.emit(
                start,
                json!({"kind":"negation","operator_span":operator_span,"operand":operand}),
            )
        } else if self.peek() == "match" {
            self.match_expression()
        } else if matches!(self.peek(), "Vec" | "FixedArray") {
            let fixed = self.peek() == "FixedArray";
            let type_syntax = self.ty();
            let open_paren_span = self.token("(");
            let open_bracket_span = self.token("[");
            let elements = self.values("]");
            let close_bracket_span = self.token("]");
            let close_paren_span = self.token(")");
            self.emit(start,json!({"kind":if fixed {"fixed-array-construction"} else {"vec-construction"},"type_syntax":type_syntax,"open_paren_span":open_paren_span,"open_bracket_span":open_bracket_span,"elements":elements,"close_bracket_span":close_bracket_span,"close_paren_span":close_paren_span}))
        } else {
            let first = self.identifier();
            let text = first["text"].as_str().expect("original fixture invariant").to_owned();
            if text == "true" || text == "false" {
                self.emit(start, json!({"kind":"bool-literal","value":text=="true"}))
            } else if text.as_bytes()[0].is_ascii_digit() {
                self.emit(start, json!({"kind":"i32-literal","spelling":text}))
            } else if text.starts_with('"') {
                self.emit(start, json!({"kind":"string-literal","spelling":text}))
            } else {
                let enum_call = self.peek() == "."
                    && matches!(
                        self.tokens.get(self.cursor + 2).map(|token| token.0.as_str()),
                        Some("(" | "<")
                    );
                let member = if enum_call {
                    let dot = self.token(".");
                    Some((dot, self.identifier()))
                } else {
                    None
                };
                let type_arguments =
                    if self.application_call() { self.arguments() } else { Value::Null };
                if self.peek() == "(" {
                    let open_paren_span = self.token("(");
                    if self.peek() == "{" {
                        let open_brace_span = self.token("{");
                        let mut fields = vec![];
                        while self.peek() != "}" {
                            let begin = self.start();
                            let name = self.identifier();
                            let kind = if self.peek() == ":" {
                                let colon_span = self.token(":");
                                let value = self.expression(1);
                                json!({"kind":"explicit","name":name,"colon_span":colon_span,"value":value})
                            } else {
                                let value =
                                    self.emit(begin, json!({"kind":"reference","name":name}));
                                json!({"kind":"shorthand","name":name,"value":value})
                            };
                            fields.push(json!({"span":self.span(begin,self.end()),"kind":kind}));
                            if self.peek() != "," {
                                break;
                            }
                            self.token(",");
                        }
                        let close_brace_span = self.token("}");
                        let close_paren_span = self.token(")");
                        self.emit(start,json!({"kind":"struct-construction","type_name":first,"type_arguments":type_arguments,"open_paren_span":open_paren_span,"open_brace_span":open_brace_span,"fields":fields,"close_brace_span":close_brace_span,"close_paren_span":close_paren_span}))
                    } else {
                        let values = self.values(")");
                        let close_paren_span = self.token(")");
                        let kind = if let Some((dot_span, variant)) = member {
                            assert!(values.len() <= 1);
                            json!({"kind":"enum-construction","type_name":first,"dot_span":dot_span,"variant":variant,"type_arguments":type_arguments,"open_paren_span":open_paren_span,"payload":values.first(),"close_paren_span":close_paren_span})
                        } else if matches!(
                            text.as_str(),
                            "clone" | "shared" | "downgrade" | "borrow" | "borrowMut" | "push"
                        ) {
                            assert!(type_arguments.is_null());
                            if text == "push" {
                                assert_eq!(values.len(), 2);
                                json!({"kind":"vec-push","keyword_span":first["span"],"open_paren_span":open_paren_span,"vector":values[0],"comma_span":self.separator(values[0],values[1]),"value":values[1],"close_paren_span":close_paren_span})
                            } else {
                                assert_eq!(values.len(), 1);
                                json!({"kind":if text=="borrowMut" {"borrow-mut"} else {text.as_str()},"keyword_span":first["span"],"open_paren_span":open_paren_span,"value":values[0],"close_paren_span":close_paren_span})
                            }
                        } else {
                            json!({"kind":"call","callee":first,"type_arguments":type_arguments,"open_paren_span":open_paren_span,"arguments":values,"close_paren_span":close_paren_span})
                        };
                        self.emit(start, kind)
                    }
                } else {
                    assert!(type_arguments.is_null());
                    self.emit(start, json!({"kind":"reference","name":first}))
                }
            }
        };
        self.projection(start, value)
    }

    pub(super) fn projection(&mut self, start: u32, mut value: u32) -> u32 {
        while matches!(self.peek(), "." | "[") {
            let kind = if self.peek() == "." {
                let dot_span = self.token(".");
                let field = self.identifier();
                json!({"kind":"field-access","base":value,"dot_span":dot_span,"field":field})
            } else {
                let open_bracket_span = self.token("[");
                let index = self.expression(1);
                let close_bracket_span = self.token("]");
                json!({"kind":"index","base":value,"open_bracket_span":open_bracket_span,"index":index,"close_bracket_span":close_bracket_span})
            };
            value = self.emit(start, kind);
        }
        value
    }
    pub(super) fn separator(&self, left: u32, right: u32) -> Value {
        let end = self.expressions[left as usize]["span"]["end"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .expect("original fixture invariant");
        let start = self.expressions[right as usize]["span"]["start"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .expect("original fixture invariant");
        let token = self
            .tokens
            .iter()
            .find(|token| token.0 == "," && token.1 >= end && token.2 <= start)
            .expect("original fixture invariant");
        self.span(token.1, token.2)
    }
    pub(super) fn application_call(&self) -> bool {
        if self.peek() != "<" {
            return false;
        }
        let mut depth = 0;
        for index in self.cursor..self.tokens.len() {
            match self.tokens[index].0.as_str() {
                "<" => depth += 1,
                ">" => {
                    depth -= 1;
                    if depth == 0 {
                        return self.tokens.get(index + 1).is_some_and(|token| token.0 == "(");
                    }
                }
                ";" | "=" | "{" | "}" => return false,
                _ => {}
            }
        }
        false
    }
    pub(super) fn values(&mut self, close: &str) -> Vec<u32> {
        let mut values = vec![];
        while self.peek() != close {
            values.push(self.expression(1));
            if self.peek() != "," {
                break;
            }
            self.token(",");
        }
        values
    }
    pub(super) fn match_expression(&mut self) -> u32 {
        let start = self.start();
        let keyword_span = self.token("match");
        let open_paren_span = self.token("(");
        let scrutinee = self.expression(1);
        self.token(",");
        let open_brace_span = self.token("{");
        let mut arms = vec![];
        while self.peek() != "}" {
            let (key, begin, end) = self.take();
            let (family, variant) =
                key.trim_matches('"').split_once('.').expect("original fixture invariant");
            let dot = begin + 1 + u32::try_from(family.len()).expect("fixture family length");
            let type_name = json!({"text":family,"span":self.span(begin+1,dot)});
            let variant = json!({"text":variant,"span":self.span(dot+1,end-1)});
            self.token(":");
            self.token("(");
            let binding = if self.peek() == ")" { Value::Null } else { self.identifier() };
            self.token(")");
            let arrow_span = self.token("=>");
            let value = self.expression(1);
            arms.push(json!({"span":self.span(begin,self.end()),"type_name":type_name,"dot_span":self.span(dot,dot+1),"variant":variant,"binding":binding,"arrow_span":arrow_span,"value":value}));
            if self.peek() != "," {
                break;
            }
            self.token(",");
        }
        let close_brace_span = self.token("}");
        let close_paren_span = self.token(")");
        self.emit(start,json!({"kind":"match","keyword_span":keyword_span,"open_paren_span":open_paren_span,"scrutinee":scrutinee,"close_paren_span":close_paren_span,"open_brace_span":open_brace_span,"arms":arms,"close_brace_span":close_brace_span}))
    }
}
