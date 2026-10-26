//! Private new-domain runtime material; legacy/default driver SOURCE remains separate.
use sha2::{Digest, Sha256};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;

pub(super) fn template(program: &VerifiedExecution<'_>) -> String {
    assert_eq!(
        program.runtime().declarations().native_header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h"),
        "the actual new execution must retain the exact17-operation ABI header"
    );
    let mut source = String::new();
    source.try_reserve_exact(18_363).expect("fixed bounded generic runtime template");
    for part in [
        include_str!("../../../../runtime/native/generic_execution_v1/allocation.h"),
        include_str!("../../../../runtime/native/generic_execution_v1/string_vec.h"),
        include_str!("../../../../runtime/native/generic_execution_v1/controls.h"),
    ] {
        source.push_str(part);
    }
    assert_eq!(source.len(), 18_363);
    assert_eq!(
        format!("{:x}", Sha256::digest(source.as_bytes())),
        "df9be4eee10a50225414335502f6b8153874b8d6d4b9ea0903932f505d77f980"
    );
    assert_eq!(source.matches("uint32_t zryna_rt_o1_").count(), 17);
    source
}
