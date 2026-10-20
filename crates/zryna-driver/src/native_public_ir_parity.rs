//! Frozen public-provider qualification through the original syntax, semantic and IR owners.
//! Requires an actual built CLI, never substitutes an in-process native candidate.

#[path = "native_public_ir_parity/brands.rs"]
mod brands;
#[path = "native_public_ir_parity/observations.rs"]
mod observations;
#[path = "native_public_ir_parity/support.rs"]
mod support;

use observations::{Observation, lower_v2, lower_v3, lower_v4, pair};
use support::{Workers, registry, source_map};
use zryna_driver::{
    WorkspaceSourceRoot, discover_module_closure, discover_ownership_module_closure,
};
use zryna_source::NormalizedSourcePath;

#[test]
fn frozen_114_public_sources_have_exact_verified_syntax_diagnostics_and_ir() {
    let (root, registry) = registry();
    let workers = Workers::new(&root);
    for case in &registry.sources {
        let sources = source_map(&root, &case.source);
        let observation = match case.protocol {
            2 => pair(
                workers.native2.analyze_verified(&sources),
                workers.bootstrap2.analyze_verified(&sources),
                &case.id,
                |syntax| lower_v2(syntax, &sources),
            ),
            3 => pair(
                workers.native3.analyze_verified_v3(&sources),
                workers.bootstrap3.analyze_verified_v3(&sources),
                &case.id,
                |syntax| lower_v3(syntax, &sources, &case.source.path),
            ),
            4 => pair(
                workers.native4.analyze_verified_v4(&sources),
                workers.bootstrap4.analyze_verified_v4(&sources),
                &case.id,
                |syntax| lower_v4(syntax, &sources, &case.source.path),
            ),
            _ => panic!("unregistered protocol"),
        };
        if case.syntax_verification == "known-hostile-Y4002" {
            assert_eq!(observation.phase, "worker", "{}", case.id);
            assert_eq!(observation.codes(), ["ZRYNA-Y4002"], "{}", case.id);
            assert!(observation.ir.is_none(), "hostile candidate cannot acquire IR");
        }
    }
    assert_eq!(registry.sources.len(), 114);
}

#[test]
fn frozen_88_public_build_roots_have_exact_source_graph_verified_ir_and_diagnostics() {
    let (root, registry) = registry();
    let workers = Workers::new(&root);
    let workspace = WorkspaceSourceRoot::capture(&root).expect("retained repository source root");
    for case in &registry.cli_build_cases {
        let entry = NormalizedSourcePath::new(case.entrypoint.clone()).expect("frozen entry");
        let observation = match case.profile.as_str() {
            "i32-v1" => {
                let pin = &registry
                    .sources
                    .iter()
                    .find(|source| source.source.path == case.entrypoint)
                    .expect("pinned entry")
                    .source;
                let sources = source_map(&root, pin);
                pair(
                    workers.native2.analyze_verified(&sources),
                    workers.bootstrap2.analyze_verified(&sources),
                    &case.id,
                    |syntax| lower_v2(syntax, &sources),
                )
            }
            "control-flow-v1" => {
                let native = discover_module_closure(&workspace, entry.clone(), &workers.native3);
                let bootstrap = discover_module_closure(&workspace, entry, &workers.bootstrap3);
                match (native, bootstrap) {
                    (Ok(left), Ok(right)) => {
                        assert_eq!(left.modules(), right.modules(), "{} module hashes", case.id);
                        assert_eq!(
                            left.graph_sha256(),
                            right.graph_sha256(),
                            "{} graph hash",
                            case.id
                        );
                        assert_eq!(
                            brands::debug(left.edges(), left.sources()),
                            brands::debug(right.edges(), right.sources()),
                            "{} complete graph edges",
                            case.id
                        );
                        assert_eq!(
                            brands::debug(left.syntax(), left.sources()),
                            brands::debug(right.syntax(), right.sources()),
                            "{} complete final syntax",
                            case.id
                        );
                        let left = Observation::lowered(
                            left.lower_control_flow_v1(),
                            left.syntax().diagnostics(),
                            left.sources(),
                        );
                        let right = Observation::lowered(
                            right.lower_control_flow_v1(),
                            right.syntax().diagnostics(),
                            right.sources(),
                        );
                        assert_eq!(left, right, "{} complete verified IR and diagnostics", case.id);
                        left
                    }
                    (Err(left), Err(right)) => {
                        let left = Observation::closure(left);
                        assert_eq!(
                            left,
                            Observation::closure(right),
                            "{} graph rejection",
                            case.id
                        );
                        left
                    }
                    values => panic!("{} graph acceptance differs: {values:?}", case.id),
                }
            }
            "data-ownership-v1" => {
                let native =
                    discover_ownership_module_closure(&workspace, entry.clone(), &workers.native4);
                let bootstrap =
                    discover_ownership_module_closure(&workspace, entry, &workers.bootstrap4);
                match (native, bootstrap) {
                    (Ok(left), Ok(right)) => {
                        assert_eq!(left.modules(), right.modules(), "{} module hashes", case.id);
                        assert_eq!(
                            left.graph_sha256(),
                            right.graph_sha256(),
                            "{} graph hash",
                            case.id
                        );
                        assert_eq!(
                            brands::debug(left.edges(), left.sources()),
                            brands::debug(right.edges(), right.sources()),
                            "{} complete graph edges",
                            case.id
                        );
                        assert_eq!(
                            brands::debug(left.syntax(), left.sources()),
                            brands::debug(right.syntax(), right.sources()),
                            "{} complete final syntax",
                            case.id
                        );
                        let left = Observation::lowered(
                            left.lower_data_ownership_v1(),
                            left.syntax().diagnostics(),
                            left.sources(),
                        );
                        let right = Observation::lowered(
                            right.lower_data_ownership_v1(),
                            right.syntax().diagnostics(),
                            right.sources(),
                        );
                        assert_eq!(
                            left, right,
                            "{} complete verified ownership IR and runtime ABI",
                            case.id
                        );
                        left
                    }
                    (Err(left), Err(right)) => {
                        let left = Observation::closure(left);
                        assert_eq!(
                            left,
                            Observation::closure(right),
                            "{} graph rejection",
                            case.id
                        );
                        left
                    }
                    values => panic!("{} graph acceptance differs: {values:?}", case.id),
                }
            }
            _ => panic!("unregistered profile"),
        };
        assert_eq!(
            observation.ir.is_some(),
            case.disposition == "accepted",
            "{} independent frozen acceptance",
            case.id
        );
        assert_eq!(
            observation.codes(),
            case.diagnostic_codes,
            "{} independent frozen diagnostic codes",
            case.id
        );
    }
    assert_eq!(registry.cli_build_cases.len(), 88);
}
