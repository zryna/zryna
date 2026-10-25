//! Untyped instrumentation checks cleanup through the actual invocation transport.
use crate::generic_execution_v1::{accounting_tests, emit, emit_invocation, tests::with_program};
use sha2::{Digest, Sha256};
use std::{path::Path, process::Command};
use zryna_abi::Invocation;

#[test]
fn five_authentic_identity_routes_clean_nonempty_owners_and_retry_same_instance() {
    let oracle = include_str!("cleanup/EXPECTED.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle.as_bytes())),
        "df511427cbaa9b2d51fd412f36c382d1611bfa60b1e09f9a0cbb06d08eb01303"
    );
    for provider in ["native", "typescript"] {
        with_program("nominal-enum-clone", provider, |program| {
            let ordinary = emit(program).expect("ordinary preimage from genuine NEW seal");
            let carrier = emit_invocation(program, Invocation::new("entry".into(), vec![]))
                .expect("same source scalar invocation");
            carrier.audit_source(carrier.as_source()).expect("pristine typed carrier");
            let private = instrument(carrier.as_source());
            assert!(
                carrier.audit_source(&private).is_err(),
                "instrumented text grants no authority"
            );
            carrier.audit_source(carrier.as_source()).expect("pristine retry after rejection");
            assert_eq!(ordinary, emit(program).expect("ordinary byte replay"));
            execute(provider, carrier.as_source(), &private, oracle);
        });
    }
}

fn once(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "exact real private call-role hook");
    text.replacen(old, new, 1)
}

fn instrument(source: &str) -> String {
    let text = once(
        source,
        "function $stringClone($source,$ctx){\n",
        "function $stringClone($source,$ctx){\n$testClonePreflights++;if($testCode!==0){const $code=$testCode;$testCode=0;throw $sourceTraps[$code];}\n",
    );
    let text = once(
        &text,
        "$r.live=true;$live++;return $result;",
        "$r.live=true;$live++;$testAcquired.push({id:$id+1,bytes:$size,alignment:$align});return $result;",
    );
    let text = once(
        &text,
        "function $stringFree($h){if(!$string($h))return 255;return $release($h.pointer,$h.capacity,1);}",
        "function $stringFree($h){const $before=$string($h);if(!$before)return 255;const $id=$h.pointer.id+1,$bytes=Array.from($before.data);const $status=$release($h.pointer,$h.capacity,1);$testReleased.push({id:$id,bytes:$bytes,status:$status});return $status;}",
    );
    let text = once(
        &text,
        "$runtimeExit=function(){if($live!==0)$abi();};",
        "$runtimeExit=function(){if($live!==0)$abi();$testKernelExit={live:$live,attempts:$attempts,steps:$steps};};",
    );
    let text = once(
        &text,
        "function $accountExit($ctx){$accountCheck($ctx);if($ctx.pending||$ctx.L!==0||$ctx.H!==0)$accountAbi();$accountActive=null;}",
        "function $accountExit($ctx){$accountCheck($ctx);if($ctx.pending||$ctx.L!==0||$ctx.H!==0)$accountAbi();$testAccountExit={A:$ctx.A,L:$ctx.L,S:$ctx.S,H:$ctx.H};$accountActive=null;}",
    );
    format!("{text}\n{}", include_str!("cleanup/instrument.mjs"))
}

fn execute(provider: &str, pristine: &str, private: &str, oracle: &str) {
    let directory = std::env::temp_dir()
        .join(format!("m416-js-carrier-cleanup-{}-{provider}", std::process::id()));
    std::fs::create_dir(&directory).expect("new retained evidence directory");
    for (name, text) in
        [("pristine.mjs", pristine), ("private.mjs", private), ("EXPECTED.json", oracle)]
    {
        std::fs::write(directory.join(name), text).expect("retain exact pre-execution material");
    }
    let mut command = Command::new("node");
    command
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/generic_execution_v1/invocation/tests/cleanup/execute.mjs"),
        )
        .arg(&directory)
        .arg(provider);
    let result = accounting_tests::process::run(command);
    std::fs::write(directory.join("stdout.jsonl"), &result.output.stdout).expect("retain stdout");
    std::fs::write(directory.join("stderr.txt"), &result.output.stderr).expect("retain stderr");
    let status = serde_json::json!({
        "provider":provider,"status":result.output.status.to_string(),
        "exit_code":result.output.status.code(),"containment":result.containment,
        "deadline_seconds":60,"stream_bytes":65536,"instrumented_artifact_authority":false
    });
    std::fs::write(
        directory.join("status.json"),
        serde_json::to_vec_pretty(&status).expect("process receipt"),
    )
    .expect("retain status before comparisons");
    assert!(result.containment.is_none(), "bounded host failure at {directory:?}");
    assert!(
        result.output.status.success(),
        "{}; retained {directory:?}",
        String::from_utf8_lossy(&result.output.stderr)
    );
    assert!(result.output.stderr.is_empty(), "unexpected stderr retained at {directory:?}");
    let rows = String::from_utf8(result.output.stdout).expect("bounded actual JSONL");
    let rows = rows
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("actual observation"))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 16, "fifteen actual invocations plus terminal summary");
    assert_eq!(rows[15]["calls"], 15);
    assert_eq!(rows[15]["same_instance"], true);
    assert_eq!(rows[15]["typed_artifact"], false);
}
