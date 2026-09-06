use super::*;
use zryna_syntax::v4::{RawDataDeclaration, RawDataDeclarationKind, RawDataField, RawEnumVariant};

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Payload {
    String,
    Struct,
    Enum,
    FixedArray,
    Vec,
    NestedShared,
    NestedWeak,
    RecursiveVec,
}

impl Payload {
    pub(super) fn ty(self) -> Ty {
        match self {
            Self::String => Ty::String,
            Self::Struct => Ty::Named("PayloadStruct"),
            Self::Enum => Ty::Named("PayloadEnum"),
            Self::FixedArray => Ty::Array(Box::new(Ty::String), 1),
            Self::Vec => Ty::Vec(Box::new(Ty::String)),
            Self::NestedShared => Ty::Shared(Box::new(Ty::String)),
            Self::NestedWeak => Ty::Weak(Box::new(Ty::String)),
            Self::RecursiveVec => Ty::Named("RecursivePayload"),
        }
    }
}

pub(super) fn for_payload(f: &mut Builder, payload: Payload) -> Vec<RawDataDeclaration> {
    match payload {
        Payload::Struct => vec![nominal(f, false, false)],
        Payload::Enum => vec![nominal(f, true, false)],
        Payload::RecursiveVec => vec![nominal(f, false, true)],
        _ => Vec::new(),
    }
}

fn nominal(f: &mut Builder, enumeration: bool, recursive: bool) -> RawDataDeclaration {
    let start = f.source.len();
    let interface_span = f.text("interface");
    f.text(" ");
    let name = f.name(if recursive {
        "RecursivePayload"
    } else if enumeration {
        "PayloadEnum"
    } else {
        "PayloadStruct"
    });
    f.text(" ");
    let extends_span = f.text("extends");
    f.text(" ");
    let marker_span = f.text(if enumeration { "ZrynaEnum" } else { "ZrynaStruct" });
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    let member_start = f.source.len();
    let member = f.name("value");
    let colon_span = f.text(":");
    f.text(" ");
    let member_ty =
        if recursive { Ty::Vec(Box::new(Ty::Named("RecursivePayload"))) } else { Ty::String };
    let type_syntax = f.ty(&member_ty);
    let semicolon_span = f.text(";");
    f.text(" ");
    let close_brace_span = f.text("}");
    f.text("\n");
    let kind = if enumeration {
        RawDataDeclarationKind::Enum {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            variants: vec![RawEnumVariant {
                span: at(member_start, semicolon_span.end as usize),
                name: member,
                colon_span,
                payload_type: Some(type_syntax),
                none_span: None,
                semicolon_span,
            }],
            close_brace_span,
        }
    } else {
        RawDataDeclarationKind::Struct {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            fields: vec![RawDataField {
                span: at(member_start, semicolon_span.end as usize),
                name: member,
                colon_span,
                type_syntax,
                semicolon_span,
            }],
            close_brace_span,
        }
    };
    RawDataDeclaration { span: at(start, close_brace_span.end as usize), export_span: None, kind }
}
