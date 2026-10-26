//! Mutate the actual SSA role graph; these pure checks issue no executable authority.
use super::*;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlagsData, Signature};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
#[test]
fn actual_request_and_publication_roles_reject_independent_drift() {
    for mutation in 0..5 {
        let mut f = Function::new();
        f.signature = Signature::new(CallConv::SystemV);
        f.signature.params.extend([AbiParam::new(types::I64), AbiParam::new(types::I64)]);
        let mut bc = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut f, &mut bc);
        let block = b.create_block();
        b.append_block_params_for_function_params(block);
        b.switch_to_block(block);
        let target = b.block_params(block)[0];
        let out = b.block_params(block)[1];
        let length = b.ins().load(types::I64, MemFlagsData::new(), target, 8);
        let actual = b.ins().iadd_imm_u(length, 1);
        let capacity = b.ins().load(
            types::I64,
            MemFlagsData::new(),
            target,
            if mutation == 2 { 8 } else { 16 },
        );
        let four = b.ins().iconst(types::I64, if mutation == 1 { 2 } else { 4 });
        let small = b.ins().icmp(IntCC::UnsignedLessThan, actual, four);
        let minimum = b.ins().select(small, four, actual);
        let condition = if mutation == 3 { IntCC::NotEqual } else { IntCC::Equal };
        let empty = b.ins().icmp_imm_u(condition, capacity, 0);
        let request = b.ins().select(empty, minimum, actual);
        let written = if mutation == 4 { request } else { actual };
        let published = b.ins().store(MemFlagsData::new(), written, out, 8);
        assert_eq!(shape(b.func, target, request, out, published).is_ok(), mutation == 0);
    }
}
