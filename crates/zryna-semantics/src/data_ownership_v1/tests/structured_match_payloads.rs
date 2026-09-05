use super::*;
use zryna_syntax::v4::RawDataField;

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Payload {
    I32,
    String,
    Array(u32),
    Vec,
    Nested,
    Struct,
    Enum,
}

impl Builder {
    pub(in crate::data_ownership_v1::tests) fn payload_type(&mut self, payload: Payload) -> u32 {
        let start = self.text.len();
        let kind = match payload {
            Payload::I32 => return self.named_type("i32"),
            Payload::String => return self.ty(true),
            Payload::Struct => return self.named_type("Envelope"),
            Payload::Enum => return self.named_type("Inner"),
            Payload::Array(length) => {
                let keyword_span = self.text("FixedArray");
                let less_than_span = self.text("<");
                let element = self.ty(true);
                let comma_span = self.text(",");
                self.text(" ");
                let length_spelling = length.to_string();
                let length_span = self.text(&length_spelling);
                let greater_than_span = self.text(">");
                RawTypeSyntaxKind::FixedArray {
                    keyword_span,
                    less_than_span,
                    element,
                    comma_span,
                    length_span,
                    length_spelling,
                    length,
                    greater_than_span,
                }
            }
            Payload::Vec | Payload::Nested => {
                let keyword_span = self.text("Vec");
                let less_than_span = self.text("<");
                let argument = if matches!(payload, Payload::Nested) {
                    self.payload_type(Payload::Array(2))
                } else {
                    self.ty(true)
                };
                let greater_than_span = self.text(">");
                RawTypeSyntaxKind::Vec { keyword_span, less_than_span, argument, greater_than_span }
            }
        };
        let id = u32::try_from(self.types.len()).expect("type count");
        self.types.push(RawTypeSyntax { span: self.span(start), kind });
        id
    }

    pub(in crate::data_ownership_v1::tests) fn payload_declaration(
        &mut self,
        payload: Payload,
    ) -> Option<RawDataDeclaration> {
        if !matches!(payload, Payload::Struct | Payload::Enum) {
            return None;
        }
        let start = self.text.len();
        let interface_span = self.text("interface");
        self.text(" ");
        let name = self.name(if matches!(payload, Payload::Struct) { "Envelope" } else { "Inner" });
        self.text(" ");
        let extends_span = self.text("extends");
        self.text(" ");
        let marker_span =
            self.text(if matches!(payload, Payload::Struct) { "ZrynaStruct" } else { "ZrynaEnum" });
        self.text(" ");
        let open_brace_span = self.text("{");
        self.text(" ");
        let member_start = self.text.len();
        let member = self.name("items");
        let colon_span = self.text(":");
        self.text(" ");
        let type_syntax = self.payload_type(Payload::Nested);
        let semicolon_span = self.text(";");
        let member_span = self.span(member_start);
        let kind = if matches!(payload, Payload::Struct) {
            self.text(" ");
            let close_brace_span = self.text("}");
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
                    type_syntax,
                    semicolon_span,
                }],
            }
        } else {
            let first = RawEnumVariant {
                span: member_span,
                name: member,
                colon_span,
                payload_type: Some(type_syntax),
                none_span: None,
                semicolon_span,
            };
            self.text(" ");
            let member_start = self.text.len();
            let member = self.name("empty");
            let colon_span = self.text(":");
            self.text(" ");
            let none_span = Some(self.text("ZrynaNone"));
            let semicolon_span = self.text(";");
            let second = RawEnumVariant {
                span: self.span(member_start),
                name: member,
                colon_span,
                payload_type: None,
                none_span,
                semicolon_span,
            };
            self.text(" ");
            let close_brace_span = self.text("}");
            RawDataDeclarationKind::Enum {
                interface_span,
                name,
                extends_span,
                marker_span,
                open_brace_span,
                close_brace_span,
                variants: vec![first, second],
            }
        };
        Some(RawDataDeclaration { span: self.span(start), export_span: None, kind })
    }
}
