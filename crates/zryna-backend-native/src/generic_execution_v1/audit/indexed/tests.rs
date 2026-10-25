//! Actual SSA predicate controls issue no source, index-liveness or executable authority.
use super::*;
use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
#[test]
fn actual_bounds_role_rejects_signed_index_wrong_width_extension_or_status() {
    for mutation in 0..5 {
        let mut f = Function::new();
        f.signature = Signature::new(CallConv::SystemV);
        let index_ty = if mutation == 4 { types::I8 } else { types::I32 };
        f.signature.params.extend([AbiParam::new(index_ty), AbiParam::new(types::I64)]);
        let mut bc = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut f, &mut bc);
        let block = b.create_block();
        b.append_block_params_for_function_params(block);
        b.switch_to_block(block);
        let index = b.block_params(block)[0];
        let length = b.block_params(block)[1];
        let wide = if mutation == 3 {
            b.ins().sextend(types::I64, index)
        } else {
            b.ins().uextend(types::I64, index)
        };
        let condition = if mutation == 2 { IntCC::SignedLessThan } else { IntCC::UnsignedLessThan };
        let valid = b.ins().icmp(condition, wide, length);
        let zero = b.ins().iconst(types::I32, 0);
        let failure = b.ins().iconst(types::I32, if mutation == 1 { 2 } else { 1 });
        let status = b.ins().select(valid, zero, failure);
        assert_eq!(bounds(b.func, status).is_ok(), mutation == 0);
    }
}

mod freshness;
