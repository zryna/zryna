use super::*;
use zryna_ir::data_ownership_v1::ValueIdentity;
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

// A finite value of a recursive type: the Vec provides the layout indirection.
fn recursive_vec_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (source, mut raw) = mixed_construction::mixed_fixture(false);
    assert_eq!(source.matches("String").count(), 2);
    let mut source = source.replace("String", "Parcel");
    assert_eq!(raw.files[0].type_syntax.len(), 5);
    for index in [0, 3] {
        let ty = &mut raw.files[0].type_syntax[index];
        assert!(matches!(ty.kind, RawTypeSyntaxKind::String { .. }));
        ty.kind = RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "Parcel".into(), span: ty.span },
        };
    }
    let literal = source.find("\"a\"").expect("existing single Vec element");
    source.replace_range(literal..literal + 3, "");
    raw = shift_snapshot_signed(raw, u32::try_from(literal + 3).expect("offset"), -3);
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 3);
    let mut vector = body.expressions[1].clone();
    let RawExpressionKind::VecConstruction { elements, .. } = &mut vector.kind else {
        panic!("recursive Vec initializer");
    };
    assert_eq!(elements, &[0]);
    elements.clear();
    let mut structure = body.expressions[2].clone();
    let RawExpressionKind::StructConstruction { fields, .. } = &mut structure.kind else {
        panic!("Parcel constructor");
    };
    assert_eq!(fields.len(), 1);
    let RawFieldInitializerKind::Explicit { value, .. } = &mut fields[0].kind else {
        panic!("explicit recursive Vec field");
    };
    assert_eq!(*value, 1);
    *value = 0;
    body.expressions = vec![vector, structure];
    let RawStatementKind::Return { value, .. } = &mut body.statements[0].kind else {
        panic!("constructor return");
    };
    *value = 1;
    assert_eq!(
        source,
        "interface Parcel extends ZrynaStruct { value: Vec<Parcel>; }\nfunction make(): Parcel { return Parcel({ value: Vec<Parcel>([]) }); }"
    );
    (source, raw)
}

#[test]
fn mixed_recursive_vec_indirection_constructs_finite_empty_child_through_full_ir() {
    let (source, snapshot) = recursive_vec_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated recursive Vec source");
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("finite recursive full IR");
        let module = program.modules().next().expect("module");
        let functions = module.functions().collect::<Vec<_>>();
        assert_eq!(functions.len(), 1);
        let function = &functions[0];
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![VerifiedInstructionKind::VecConstruct, VerifiedInstructionKind::StructConstruct]
        );
        assert_eq!(function.places().count(), 2);
        assert_eq!(function.cleanup_plans().count(), 2);
        assert_eq!(instructions[0].value_operands().count(), 0);
        assert_eq!(
            instructions[1].value_operands().collect::<Vec<_>>(),
            vec![instructions[0].result().expect("empty recursive Vec")]
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            vec![instructions[1].result().expect("finite Parcel")]
        );
        assert!(instructions.iter().all(|i| i.derived_drop_actions().count() == 0));
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.kind(),
                    i.result().map(ValueIdentity::index),
                    i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}
