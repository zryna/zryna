use super::*;
use zryna_diagnostics::Diagnostic;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawExpressionKind;

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1) enum TypeFailure {
    Nominal,
    Element,
    Context,
    Payload,
}

// Add one same-layout but distinct nominal declaration. No semantic/type inference here.
fn add_packet(source: &mut String, raw: &mut RawProjectSyntaxSnapshot) {
    let prefix_end = usize::try_from(raw.files[0].data_declarations[0].span.end).expect("span");
    let text = source[..prefix_end].replace("Parcel", "Packet");
    source.push('\n');
    let offset = u32::try_from(source.len()).expect("bounded source");
    let shifted = shift_snapshot(raw.clone(), 0, offset);
    let mut declaration = shifted.files[0].data_declarations[0].clone();
    let RawDataDeclarationKind::Struct { name, fields, .. } = &mut declaration.kind else {
        panic!("Struct")
    };
    name.text = "Packet".into();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].type_syntax, 0, "Vec<Parcel> fixture has String field");
    fields[0].type_syntax = u32::try_from(raw.files[0].type_syntax.len()).expect("type ID");
    raw.files[0].type_syntax.push(shifted.files[0].type_syntax[0].clone());
    raw.files[0].data_declarations.push(declaration);
    source.push_str(&text);
}

pub(in crate::data_ownership_v1) fn type_fixture(
    case: TypeFailure,
    invalid: bool,
) -> (String, RawProjectSyntaxSnapshot, UntrustedSpan) {
    match case {
        TypeFailure::Payload => {
            let (s, mut raw) = nested_enum_fixture();
            let mut source = s.0.to_owned();
            let expression = &mut raw.files[0].functions[0].body.expressions[2];
            let RawExpressionKind::EnumConstruction { variant, payload, .. } = &mut expression.kind
            else {
                panic!("selected Choice")
            };
            assert_eq!(variant.text, "some");
            assert_eq!(*payload, Some(1));
            if invalid {
                source
                    .replace_range(variant.span.start as usize..variant.span.end as usize, "none");
                variant.text = "none".into();
            }
            let bad = expression.span;
            (source, raw, bad)
        }
        TypeFailure::Nominal | TypeFailure::Context => {
            let (mut source, mut raw) = mixed_construction::mixed_fixture(true);
            add_packet(&mut source, &mut raw);
            let bad = if matches!(case, TypeFailure::Nominal) {
                let RawExpressionKind::StructConstruction { type_name, .. } =
                    &mut raw.files[0].functions[0].body.expressions[1].kind
                else {
                    panic!("Parcel constructor")
                };
                if invalid {
                    source.replace_range(
                        type_name.span.start as usize..type_name.span.end as usize,
                        "Packet",
                    );
                    type_name.text = "Packet".into();
                }
                type_name.span
            } else {
                // The outer return annotation becomes Vec<Packet>, while its initializer
                // remains Vec<Parcel>. Both element declarations are available and identical in shape.
                let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[1].kind
                else {
                    panic!("return element")
                };
                if invalid {
                    source
                        .replace_range(name.span.start as usize..name.span.end as usize, "Packet");
                    name.text = "Packet".into();
                }
                raw.files[0].functions[0].body.expressions[2].span
            };
            (source, raw, bad)
        }
        TypeFailure::Element => {
            let (s, mut raw) = nested_vec_fixture();
            let mut source = s.0.to_owned();
            if invalid {
                // Only the first inner constructor changes: Vec<i32>([7]) -> Vec<bool>([true]).
                let element = raw.files[0].type_syntax[6].span;
                source.replace_range(element.start as usize..element.end as usize, "bool");
                raw = shift_snapshot(raw, element.end, 1);
                let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[6].kind
                else {
                    panic!("inner element")
                };
                name.text = "bool".into();
                let literal = raw.files[0].functions[0].body.expressions[0].span;
                source.replace_range(literal.start as usize..literal.end as usize, "true");
                raw = shift_snapshot(raw, literal.end, 3);
                raw.files[0].functions[0].body.expressions[0].kind =
                    RawExpressionKind::BoolLiteral { value: true };
            }
            let bad = raw.files[0].functions[0].body.expressions[1].span;
            (source, raw, bad)
        }
    }
}

pub(in crate::data_ownership_v1) fn expected_type_failure(
    case: TypeFailure,
    sources: &SourceMap,
    at: UntrustedSpan,
) -> Diagnostic {
    let (code, message, help) = match case {
        TypeFailure::Payload => (
            "ZRYNA-M3005",
            "enum payload presence does not match the declared variant",
            "supply exactly one payload only for a payload variant",
        ),
        TypeFailure::Nominal => (
            "ZRYNA-M3016",
            "struct constructor type or ownership graph is outside the exact supported slice",
            "use an acyclic struct containing only bool, i32, String, or supported fixed arrays",
        ),
        TypeFailure::Element | TypeFailure::Context => (
            "ZRYNA-M3013",
            "Vec construction type differs from its contextual type",
            "construct the exact annotated Vec type",
        ),
    };
    Diagnostic::error_at(code, span(sources, at), message, help)
}

#[test]
fn mixed_source_rejects_distinct_nominal_nested_element_and_outer_context_exactly() {
    for case in
        [TypeFailure::Nominal, TypeFailure::Element, TypeFailure::Context, TypeFailure::Payload]
    {
        let (source, raw, _) = type_fixture(case, false);
        let sources = sources_for(&source);
        let syntax =
            verify_snapshot(raw, &sources).expect("valid source including distinct nominal");
        let mut previous = None;
        for _ in 0..2 {
            let program = lower(pair_input(&syntax, &sources)).expect("valid control full IR");
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let kinds = block
                .instructions()
                .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                .collect::<Vec<_>>();
            assert_eq!(
                kinds,
                if matches!(case, TypeFailure::Payload) {
                    vec![
                        VerifiedInstructionKind::StringFromUtf8,
                        VerifiedInstructionKind::VecConstruct,
                        VerifiedInstructionKind::EnumConstruct,
                        VerifiedInstructionKind::VecConstruct,
                    ]
                } else if matches!(case, TypeFailure::Element) {
                    vec![
                        VerifiedInstructionKind::I32Literal,
                        VerifiedInstructionKind::VecConstruct,
                        VerifiedInstructionKind::VecConstruct,
                        VerifiedInstructionKind::VecConstruct,
                    ]
                } else {
                    vec![
                        VerifiedInstructionKind::StringFromUtf8,
                        VerifiedInstructionKind::StructConstruct,
                        VerifiedInstructionKind::VecConstruct,
                    ]
                }
            );
            assert_eq!(block.terminator().derived_drop_actions().count(), 0);
            if let Some(previous) = &previous {
                assert_eq!(&kinds, previous);
            }
            previous = Some(kinds);
        }
        let (source, raw, bad) = type_fixture(case, true);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authentic exact-type rejection source");
        let expected = vec![expected_type_failure(case, &sources, bad)];
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("exact type rejected"),
                expected
            );
        }
    }
}
