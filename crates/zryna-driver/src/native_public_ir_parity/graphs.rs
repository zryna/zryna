use zryna_driver::{
    WorkspaceSourceRoot, discover_module_closure, discover_ownership_module_closure,
};
use zryna_source::NormalizedSourcePath;

use super::{
    brands,
    observations::Observation,
    support::{BuildCase, Workers},
};

pub fn control_flow(
    workspace: &WorkspaceSourceRoot,
    workers: &Workers,
    entry: NormalizedSourcePath,
    case: &BuildCase,
) -> Observation {
    let native = discover_module_closure(workspace, entry.clone(), &workers.native3);
    let bootstrap = discover_module_closure(workspace, entry, &workers.bootstrap3);
    match (native, bootstrap) {
        (Ok(left), Ok(right)) => {
            assert_eq!(left.modules(), right.modules(), "{} module hashes", case.id);
            assert_eq!(left.graph_sha256(), right.graph_sha256(), "{} graph hash", case.id);
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
            assert_eq!(left, Observation::closure(right), "{} graph rejection", case.id);
            left
        }
        values => panic!("{} graph acceptance differs: {values:?}", case.id),
    }
}

pub fn ownership(
    workspace: &WorkspaceSourceRoot,
    workers: &Workers,
    entry: NormalizedSourcePath,
    case: &BuildCase,
) -> Observation {
    let native = discover_ownership_module_closure(workspace, entry.clone(), &workers.native4);
    let bootstrap = discover_ownership_module_closure(workspace, entry, &workers.bootstrap4);
    match (native, bootstrap) {
        (Ok(left), Ok(right)) => {
            assert_eq!(left.modules(), right.modules(), "{} module hashes", case.id);
            assert_eq!(left.graph_sha256(), right.graph_sha256(), "{} graph hash", case.id);
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
            assert_eq!(left, right, "{} complete verified ownership IR and runtime ABI", case.id);
            left
        }
        (Err(left), Err(right)) => {
            let left = Observation::closure(left);
            assert_eq!(left, Observation::closure(right), "{} graph rejection", case.id);
            left
        }
        values => panic!("{} graph acceptance differs: {values:?}", case.id),
    }
}
