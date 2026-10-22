//! Public accepted-size reachability proof, not an exact 1 MiB acceptance fixture.
//! The driver separately binds public preparation to one no-arg i32 scalar export.

use super::{ServerOperation, check, component};
use zryna_ir::{Expr, ExprId, ExprKind, Function, Program, Type, VerifiedProgram};
use zryna_source::{SourceFileInput, SourceMap, Span};

fn maximum_public_shape() -> (String, VerifiedProgram) {
    fn tree(expressions: &mut Vec<Expr>, span: Span, start: usize, leaves: usize) -> ExprId {
        let kind = if leaves == 1 {
            // Alternating MAX/MIN leaves sum to -4096; the final adjustment makes
            // the whole 8192-leaf expression return 200. Every leaf still needs
            // five signed LEB bytes, so this retains maximum literal width.
            ExprKind::I32Literal(if start == 8191 {
                i32::MIN + 4296
            } else if start.is_multiple_of(2) {
                i32::MAX
            } else {
                i32::MIN
            })
        } else {
            let half = leaves / 2;
            let lhs = tree(expressions, span, start, half);
            let rhs = tree(expressions, span, start + half, half);
            ExprKind::I32Add { lhs, rhs }
        };
        let id = ExprId(u32::try_from(expressions.len()).expect("bounded expression identity"));
        expressions.push(Expr { ty: Type::I32, span, kind });
        id
    }
    let name = "s".repeat(zryna_ir::MAX_IR_EXPORT_NAME_BYTES);
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "status.zry".into(), text: name.clone() }])
            .expect("private verified-IR source map");
    let file = sources.verify_file_id(0).expect("source identity");
    let span = sources.span(file, 0, 128).expect("source-backed test span");
    let mut expressions = Vec::new();
    let body = tree(&mut expressions, span, 0, 8192);
    assert_eq!(expressions.len(), 16_383);
    let program = zryna_ir::verify(
        Program {
            functions: vec![Function {
                name: name.clone(),
                parameters: vec![],
                return_type: Type::I32,
                expressions,
                body,
            }],
        },
        &sources,
    )
    .expect("maximum sole-function no-arg i32 public shape");
    (name, program)
}

#[test]
fn public_single_scalar_size_upper_bound_is_strictly_below_component_ceiling() {
    let expressions = zryna_ir::MAX_IR_EXPRESSIONS_PER_FUNCTION;
    let names = zryna_ir::MAX_IR_EXPORT_NAME_BYTES;
    assert_eq!((expressions, names), (16_384, 128));
    // A no-arg I32V1 single-owner tree contains only i32.const and i32.add.
    // With L leaves it has L-1 adds and 2L-1 nodes. The largest admitted tree
    // therefore has 8192 leaves/8191 adds. Each signed-i32 literal encodes in
    // at most 6 bytes (one opcode plus five signed LEB bytes); add takes one.
    let leaves = expressions.div_ceil(2);
    let opcode_bytes = leaves * 6 + leaves - 1;
    assert_eq!(opcode_bytes, 57_343);
    let (name, program) = maximum_public_shape();
    for operation in [ServerOperation::Reply, ServerOperation::ClockRead] {
        let baseline = component(operation);
        check(&baseline, baseline.bytes()).expect("actual canonical baseline audits");
        // Sole scalar signature/count, authenticated WIT declarations, allocator,
        // operation bridge and topology are fixed by these source boundaries.
        // Changing its expression tree contributes at most opcode_bytes.
        // A selected export name occurs once in scalar exports and once in the
        // bridge import, each at most 128 bytes. Variable ULEB framing is bounded
        // conservatively by 256 bytes (far above all relevant length prefixes).
        // Counting the complete baseline again deliberately overestimates every
        // fixed byte and its baseline expressions/names; it never subtracts them.
        let upper_bound = baseline.bytes().len() + opcode_bytes + 2 * names + 256;
        let maximum = super::super::emit_server_response(
            &program,
            &crate::pinned_wit_sources(),
            &name,
            operation,
        )
        .expect("actual maximum public-shaped canonical component");
        maximum.revalidate(&program).expect("matching verified program and canonical replay");
        assert!(maximum.bytes().len() <= upper_bound, "actual maximal shape obeys symbolic bound");
        assert!(
            upper_bound < super::super::shell::MAX_BYTES,
            "public single-scalar bound {upper_bound} must remain strictly below 1 MiB"
        );
        println!(
            "[m4-public-server-size] {}",
            serde_json::json!({
                "schema": "zryna.m4-public-server-size.v1",
                "operation": match operation {
                    ServerOperation::Reply => "reply",
                    ServerOperation::ClockRead => "clock_read",
                },
                "baselineBytes": baseline.bytes().len(),
                "maximumShapeBytes": maximum.bytes().len(),
                "expressionNodeLimit": expressions,
                "maximumTreeNodes": 2 * leaves - 1,
                "leafCount": leaves,
                "addCount": leaves - 1,
                "exportNameBytes": names,
                "exportNameOccurrences": 2,
                "opcodeByteBound": opcode_bytes,
                "framingByteBound": 256,
                "upperBoundBytes": upper_bound,
                "componentByteLimit": super::super::shell::MAX_BYTES,
            })
        );
    }
}
