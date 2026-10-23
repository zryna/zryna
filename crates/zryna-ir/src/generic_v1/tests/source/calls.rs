//! Closed calls authenticate original substitution, source ranges and source arguments.

use super::*;

fn claimed_call(fixture: &mut Authenticated) {
    let span = fixture.syntax.files()[0].functions[0]
        .body
        .expressions
        .iter()
        .find(|expression| {
            matches!(&expression.kind, zryna_syntax::v5::RawExpressionKind::Call { callee, .. }
            if callee.text == "identity")
        })
        .expect("authenticated original identity<i32> call")
        .span;
    let block = &mut fixture.fixture.program.functions[3].blocks[0];
    block.instructions.push(raw::Instruction {
        result: definition(1, raw::Type::Stored(1)),
        span,
        operation: raw::Operation::ClosedGenericCall { instance: 1, arguments: vec![0] },
    });
    block.terminator = raw::Terminator::Return(1);
}

#[test]
fn closed_call_keys_are_original_caller_substitution_not_merely_matching_signatures() {
    let mut fixture = Authenticated::new();
    claimed_call(&mut fixture);
    fixture.check().expect("authenticated imported original call target and explicit i32 argument");
    let block = &mut fixture.fixture.program.functions[3].blocks[0];
    block.instructions[0].result.ty = raw::Type::Stored(0);
    block.instructions[0].operation = raw::Operation::BoolLiteral(true);
    block.instructions[1].result.ty = raw::Type::Stored(0);
    block.instructions[1].operation =
        raw::Operation::ClosedGenericCall { instance: 0, arguments: vec![0] };
    block.instructions.push(raw::Instruction {
        result: definition(2, raw::Type::Stored(1)),
        span: block.span,
        operation: raw::Operation::I32Literal(7),
    });
    block.terminator = raw::Terminator::Return(2);
    fixture.fixture.check().expect("forged bool call is structurally and signature consistent");
    rejection(fixture.check(), "ZRYNA-I7001");
}

#[test]
fn call_with_an_invented_source_range_or_wrong_source_argument_count_rejects() {
    for range in [false, true] {
        let mut fixture = Authenticated::new();
        claimed_call(&mut fixture);
        let block = &mut fixture.fixture.program.functions[3].blocks[0];
        if range {
            block.instructions[1].span = block.span;
        } else {
            block.instructions[1].operation =
                raw::Operation::ClosedGenericCall { instance: 1, arguments: vec![] };
        }
        rejection(fixture.check(), "ZRYNA-I7001");
    }
    let mut pristine = Authenticated::new();
    claimed_call(&mut pristine);
    pristine.check().expect("pristine source-bound recovery");
}
