//! Genuine compact source -> v5 seal -> independent backend amplification gates.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_struct_clone_compact_dag/backends.rs"]
mod backends;
#[path = "generic_struct_clone_compact_dag/source.rs"]
mod source;

#[test]
fn compact_nominal_enum_dags_seal_before_independent_native_and_web_budget_observations() {
    for case in ["compact-dag-n04", "compact-dag-n12", "compact-dag-n16"] {
        source::with_program(case, |program, wire, expected, evidence| {
            let directory = backends::freeze(case, wire, expected, evidence);
            let observations = backends::observe(program, expected, &directory);
            std::fs::write(
                directory.join("backend-observations.json"),
                serde_json::to_vec_pretty(&observations).expect("backend observations JSON"),
            )
            .expect("retain actual independent backend outcomes");
            for target in ["native", "javascript", "webassembly"] {
                let required = expected[target]["outcome"].as_str().expect("frozen backend gate");
                let actual = &observations[target];
                if required == "independent-observation-required" {
                    assert!(actual["outcome"] == "accept" || actual["outcome"] == "budget");
                    if actual["outcome"] == "budget" {
                        assert_eq!(
                            actual["diagnostic"],
                            if target == "javascript" { "ZRYNA-J2003" } else { "ZRYNA-W4001" }
                        );
                    }
                } else {
                    assert_eq!(actual["outcome"], required, "{case}/{target}: {actual}");
                    if required == "budget" {
                        assert_eq!(actual["diagnostic"], expected[target]["diagnostic"]);
                    }
                }
            }
            println!("Genuine compact Clone source/seal/backend evidence: {}", directory.display());
        });
    }
}
