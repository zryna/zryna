//! Each mutation stays valid Wasm and carries matching supplemental source bodies.
use super::*;
use sha2::{Digest, Sha256};
pub(super) fn reject(case: &str, action: Rewrite) {
    for provider in super::super::source::PROVIDERS {
        program(case, provider, |program, wire| {
            for observed in [false, true] {
                let plan = memory::verify(
                    memory::derive(program, observed).expect("genuine geometry"),
                    program,
                )
                .expect("same-seal pointer cells");
                let m = model::Model::new(program, &plan).expect("retained model");
                let op = program
                    .operations()
                    .find(|op| {
                        if case == "copy-read-control" {
                            matches!(op.kind(), raw::Operation::IndexCopy { .. })
                        } else {
                            op.indexed_loan().is_some()
                        }
                    })
                    .expect("actual indexed operation");
                let mut code = code::build(&m).expect("actual source emission");
                let pristine = artifact::encode(&m, &code).expect("actual bytes");
                artifact::independent_audit(&pristine, &m)
                    .expect("pristine independently decoded owner");
                let target = usize::try_from(m.source_base).expect("bounded source base")
                    + op.site().function;
                let ops = operators(&pristine, target);
                let (before, after) = bounds(&ops);
                let mut replacement = Vec::new();
                let range = match action {
                    Rewrite::IndexBefore | Rewrite::IndexAfter => {
                        I::I32Const(2).encode(&mut replacement);
                        I::LocalSet(5).encode(&mut replacement);
                        let at =
                            if matches!(action, Rewrite::IndexBefore) { before } else { after };
                        at..at
                    }
                    Rewrite::ParameterSet(parameter) => {
                        I::I32Const(0).encode(&mut replacement);
                        I::LocalSet(parameter).encode(&mut replacement);
                        after..after
                    }
                    Rewrite::ParameterTee(parameter) => {
                        let(_,range)=ops.iter().find(|(op,range)|range.start>=after&&matches!(op,O::LocalGet{local_index}if *local_index==parameter)).expect("actual consumed source parameter");
                        I::I32Const(0).encode(&mut replacement);
                        I::LocalTee(parameter).encode(&mut replacement);
                        range.clone()
                    }
                    Rewrite::PcTee => {
                        let (_, range) = ops
                            .iter()
                            .find(|(op, _)| matches!(op, O::LocalGet { local_index: 2 }))
                            .expect("actual source dispatch");
                        I::I32Const(i32::MAX).encode(&mut replacement);
                        I::LocalTee(2).encode(&mut replacement);
                        range.clone()
                    }
                };
                code[target].body.bytes.splice(range, replacement);
                let wrong = artifact::encode(&m, &code).expect("matching supplemental body");
                assert_ne!(wrong, pristine);
                Validator::new_with_features(WasmFeatures::WASM1)
                    .validate_all(&wrong)
                    .expect("changed write remains structurally valid Wasm");
                let independent = artifact::independent_audit(&wrong, &m);
                let full = artifact::audit(&wrong, &m, &code);
                let directory = std::env::temp_dir().join(format!(
                    "m416-wasm-index-temporal-{}-{case}-{provider}-{observed}-{action:?}",
                    std::process::id()
                ));
                std::fs::create_dir(&directory).expect("new actual evidence");
                std::fs::write(directory.join("original.wasm"), &pristine)
                    .expect("retained original");
                std::fs::write(directory.join("hostile.wasm"), &wrong).expect("retained hostile");
                std::fs::write(directory.join("program.zir"), wire)
                    .expect("genuine same source seal");
                std::fs::write(directory.join("actual-before-assertions.json"),serde_json::to_vec_pretty(&serde_json::json!({"case":case,"provider":provider,"observed":observed,"mutation":format!("{action:?}"),"target":target,"original_sha256":format!("{:x}",Sha256::digest(&pristine)),"hostile_sha256":format!("{:x}",Sha256::digest(&wrong)),"independent_accepted":independent.is_ok(),"independent_error":independent.as_ref().err().map(ToString::to_string),"full_accepted":full.is_ok(),"full_error":full.as_ref().err().map(ToString::to_string),"structurally_valid":true,"matching_supplemental_bodies":true})).expect("receipt")).expect("retain both audit outcomes before assertion");
                assert!(
                    independent.is_err(),
                    "hidden source write {action:?} must fail independent actual-byte owner"
                );
                assert!(full.is_err(), "matching supplemental body cannot authorize {action:?}");
                let retry = code::build(&m).expect("pristine emission after hostile");
                artifact::audit(&pristine, &m, &retry)
                    .expect("failed hostile proof exposes no new authority");
            }
        });
    }
}
