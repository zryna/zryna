use super::*;
use zryna_ir::data_ownership_v1::ValueIdentity;

#[path = "scalar_matrix_fixtures.rs"]
mod fixtures;

#[path = "scalar_matrix_negatives.rs"]
mod negatives;

use fixtures::{ARITHMETIC, BOOL_EQ, Binary, Expression, Node, Syntax, fixture};

// Every row spells its token width and right-operand source offset explicitly.
macro_rules! signed_row {
    ($text:literal, $syntax:ident, $opcode:ident, $width:literal, $right:literal) => {
        (
            Expression {
                text: $text,
                nodes: &[
                    Node { start: 1, end: 2, kind: Syntax::I32("1") },
                    Node { start: 0, end: 2, kind: Syntax::Neg { token: 0, operand: 0 } },
                    Node { start: $right, end: $right + 1, kind: Syntax::I32("0") },
                    Node {
                        start: 0,
                        end: $right + 1,
                        kind: Syntax::Binary {
                            token: 3,
                            width: $width,
                            kind: Binary::$syntax,
                            lhs: 1,
                            rhs: 2,
                        },
                    },
                ],
            },
            VerifiedInstructionKind::$opcode,
            true,
        )
    };
}

const COMPARISONS: [(Expression, VerifiedInstructionKind, bool); 8] = [
    (BOOL_EQ, VerifiedInstructionKind::Eq, false),
    (
        Expression {
            text: "true !== false",
            nodes: &[
                Node { start: 0, end: 4, kind: Syntax::Bool(true) },
                Node { start: 9, end: 14, kind: Syntax::Bool(false) },
                Node {
                    start: 0,
                    end: 14,
                    kind: Syntax::Binary { token: 5, width: 3, kind: Binary::Ne, lhs: 0, rhs: 1 },
                },
            ],
        },
        VerifiedInstructionKind::Ne,
        false,
    ),
    signed_row!("-1 === 0", Eq, Eq, 3, 7),
    signed_row!("-1 !== 0", Ne, Ne, 3, 7),
    signed_row!("-1 < 0", Lt, I32LtS, 1, 5),
    signed_row!("-1 <= 0", Le, I32LeS, 2, 6),
    signed_row!("-1 > 0", Gt, I32GtS, 1, 5),
    signed_row!("-1 >= 0", Ge, I32GeS, 2, 6),
];

fn expected_kinds(opcode: VerifiedInstructionKind, signed: bool) -> Vec<VerifiedInstructionKind> {
    let mut expected = vec![
        VerifiedInstructionKind::StringFromUtf8,
        VerifiedInstructionKind::VecConstruct,
        VerifiedInstructionKind::I32Literal,
        VerifiedInstructionKind::I32Literal,
        VerifiedInstructionKind::I32Literal,
        VerifiedInstructionKind::I32Mul,
        VerifiedInstructionKind::I32Sub,
        VerifiedInstructionKind::I32Literal,
        VerifiedInstructionKind::I32Neg,
        VerifiedInstructionKind::I32Add,
    ];
    expected.extend(if signed {
        vec![
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::I32Neg,
            VerifiedInstructionKind::I32Literal,
            opcode,
        ]
    } else {
        vec![VerifiedInstructionKind::BoolLiteral, VerifiedInstructionKind::BoolLiteral, opcode]
    });
    expected.push(VerifiedInstructionKind::StructConstruct);
    expected
}

#[test]
fn mixed_copy_all_ten_operators_keep_nested_arithmetic_and_signed_comparison_order() {
    for (comparison, opcode, signed) in COMPARISONS {
        let (source, snapshot) = fixture(ARITHMETIC, comparison);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(snapshot, &sources).expect("fixed operator DTO authenticates");
        let mut previous = None;
        for _ in 0..2 {
            let program = lower(pair_input(&syntax, &sources)).expect("all-operator mixed full IR");
            let module = program.modules().next().expect("module");
            let function = module.functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instructions = block.instructions().collect::<Vec<_>>();
            assert_eq!(
                instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
                expected_kinds(opcode, signed),
                "{}",
                comparison.text
            );
            for (index, literal) in [(2, 7), (3, 3), (4, 2), (7, 1)] {
                assert_eq!(instructions[index].i32_literal(), Some(literal));
            }
            for (index, operands) in
                [(1, vec![0]), (5, vec![3, 4]), (6, vec![2, 5]), (8, vec![7]), (9, vec![6, 8])]
            {
                assert_eq!(
                    instructions[index]
                        .value_operands()
                        .map(ValueIdentity::index)
                        .collect::<Vec<_>>(),
                    operands
                );
            }
            let (comparison_result, root) = if signed { (13, 14) } else { (12, 13) };
            if signed {
                assert_eq!(instructions[10].i32_literal(), Some(1));
                assert_eq!(instructions[12].i32_literal(), Some(0));
                assert_eq!(
                    instructions[11].value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                    [10]
                );
                assert_eq!(
                    instructions[13].value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                    [11, 12]
                );
            } else {
                assert_eq!(instructions[10].bool_literal(), Some(true));
                assert_eq!(instructions[11].bool_literal(), Some(false));
                assert_eq!(
                    instructions[12].value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                    [10, 11]
                );
            }
            assert_eq!(
                instructions[root].value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                [1, 9, u32::try_from(comparison_result).expect("small ID")]
            );
            assert_eq!(
                block.terminator().value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                [u32::try_from(root).expect("small ID")]
            );
            assert_eq!(function.places().count(), 3, "Copy values create no owned places");
            assert_eq!(function.cleanup_plans().count(), 3);
            assert_eq!(block.terminator().derived_drop_actions().count(), 0);
            let observed = instructions
                .iter()
                .map(|i| {
                    (
                        i.kind(),
                        i.result().map(ValueIdentity::index),
                        i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                        i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(observed[1].3, [0]);
            assert!(
                observed.iter().enumerate().all(|(index, item)| index == 1 || item.3.is_empty())
            );
            if let Some(previous) = &previous {
                assert_eq!(&observed, previous);
            }
            previous = Some(observed);
        }
    }
}

#[test]
fn mixed_i32_wrapping_boundary_retains_add_opcode_without_arithmetic_trap() {
    const WRAP: Expression = Expression {
        text: "2147483647 + 1",
        nodes: &[
            Node { start: 0, end: 10, kind: Syntax::I32("2147483647") },
            Node { start: 13, end: 14, kind: Syntax::I32("1") },
            Node {
                start: 0,
                end: 14,
                kind: Syntax::Binary { token: 11, width: 1, kind: Binary::Add, lhs: 0, rhs: 1 },
            },
        ],
    };
    let (source, snapshot) = fixture(WRAP, BOOL_EQ);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated wrapping boundary");
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("wrapping boundary full IR");
        let module = program.modules().next().expect("module");
        let function = module.functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            [
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::I32Literal,
                VerifiedInstructionKind::I32Literal,
                VerifiedInstructionKind::I32Add,
                VerifiedInstructionKind::BoolLiteral,
                VerifiedInstructionKind::BoolLiteral,
                VerifiedInstructionKind::Eq,
                VerifiedInstructionKind::StructConstruct,
            ]
        );
        assert_eq!(instructions[2].i32_literal(), Some(i32::MAX));
        assert_eq!(instructions[3].i32_literal(), Some(1));
        assert_eq!(
            instructions[4].value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            [2, 3]
        );
        assert_eq!(
            instructions[8].value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            [1, 4, 7]
        );
        assert_eq!(instructions[4].cleanup(), None);
        assert_eq!(instructions[4].derived_drop_actions().count(), 0);
        assert_eq!(function.places().count(), 3);
        assert_eq!(function.cleanup_plans().count(), 3);
        assert_eq!(
            block.terminator().value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            [8]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        // Verifies the frozen wrapping opcode/inputs, not a target runtime result.
    }
}
pub(in crate::data_ownership_v1) fn nested_scalar_fixture() -> (String, RawProjectSyntaxSnapshot) {
    fixture(ARITHMETIC, BOOL_EQ)
}
