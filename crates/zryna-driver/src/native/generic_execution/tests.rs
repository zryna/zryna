//! Genuine same-owner production runtime, link and private-provider staging controls.

mod inputs;
mod runtime;
use crate::generic_recursive_source as source;
mod staging;

use super::*;
use crate::native::{ArtifactOutputRoot, LinuxX8664LinkToolchain, NativeProcessLimits};
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
use zryna_abi::{Invocation, ScalarOutcome, ScalarValue};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn with_host(run: impl FnOnce(&ArtifactOutputRoot, &LinuxX8664LinkToolchain)) {
    let path = std::env::temp_dir().join(format!(
        "zryna-generic-production-native-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(path.join(".zryna/out")).expect("new private test output workspace");
    let root = ArtifactOutputRoot::for_workspace(&path).expect("ordinary held output capability");
    let tools = crate::native::discover_linux_native_toolchain(NativeProcessLimits::default())
        .expect("documented production GNU capability");
    run(&root, &tools);
    assert_eq!(fs::read_dir(root.path()).expect("output directory after cleanup").count(), 0);
    fs::remove_dir_all(path).expect("remove owned empty test workspace");
}

#[test]
fn genuine_erased_recursive_clone_and_all_nine_categories_prepare_and_run() {
    with_host(|root, tools| {
        for (case, arguments, expected) in
            [("erased", vec![], 7), ("clone", vec![ScalarValue::I32(2)], 7), ("mixed", vec![], 97)]
        {
            source::with_program(case, |program| {
                exercise(root, tools, program, &arguments, expected);
            });
        }
    });
}

fn exercise(
    root: &ArtifactOutputRoot,
    tools: &LinuxX8664LinkToolchain,
    program: &zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>,
    arguments: &[ScalarValue],
    expected: i32,
) {
    let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("NEW native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("ordinary object target");
    let ordinary = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("ordinary object with independent audit");
    let carrier = zryna_backend_native::generic_execution_v1::emit_invocation(
        &mir,
        Invocation::new("entry".into(), arguments.to_vec()),
        target,
    )
    .expect("source-owned invocation artifact");
    let material = crate::generic_runtime_v1::RuntimeSource::issue(program, &mir, &ordinary)
        .expect("same owner and ordinary object material");
    let prepared = prepare(material, &carrier, root, tools, NativeProcessLimits::default())
        .expect("actual production runtime compile/link/audit");
    assert!(std::ptr::eq(prepared.source().program(), program));
    bindings(&prepared, carrier.bytes(), tools, arguments);
    for _ in 0..2 {
        assert_eq!(
            run(&prepared, root, NativeProcessLimits::default())
                .expect("private staged providers and exact frame"),
            ScalarOutcome::Returned { value: ScalarValue::I32(expected) }
        );
    }
    assert_eq!(
        ordinary,
        zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
            .expect("ordinary object bytes remain unchanged after invocation emission")
    );
}

fn bindings(
    prepared: &PreparedGenericExecutable<'_, '_, '_>,
    carrier: &[u8],
    tools: &LinuxX8664LinkToolchain,
    arguments: &[ScalarValue],
) {
    let identity = prepared.identity();
    assert_eq!(identity.arguments(), arguments);
    assert_eq!(identity.logical_export(), "entry");
    assert_eq!(identity.entry_symbol(), "zryna_gexecution_invoke_v1");
    assert_eq!(*identity.invocation_object_sha256(), <[u8; 32]>::from(Sha256::digest(carrier)));
    assert_eq!(
        *identity.harness_sha256(),
        <[u8; 32]>::from(Sha256::digest(
            harness::render(identity.result_type()).expect("exact harness")
        ))
    );
    assert_eq!(
        *identity.image().executable_sha256(),
        <[u8; 32]>::from(Sha256::digest(prepared.bytes()))
    );
    assert_eq!(
        identity.image().required_tls_bytes(),
        prepared.source().identity().required_tls_bytes()
    );
    assert_eq!(identity.material().material(), prepared.source().identity());
    assert_ne!(identity.material().runtime_object_sha256(), &[0; 32]);
    assert_eq!(identity.image().closure().providers.len(), 2);
    let receipt = serde_json::to_value(identity.dependencies()).expect("owned provider evidence");
    assert_eq!(receipt["gcc_version"], tools.gcc_version());
    assert_eq!(receipt["linker_version"], tools.linker_version());
}
