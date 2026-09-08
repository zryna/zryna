use super::*;
use authority::{Authorities, InstanceAuthority, VerifiedLanguage};
use model::{Capability, Instance, Language, Reservation, Row, Selection};

use std::{fs, path::PathBuf};
use zryna_backend_webassembly::{WitSource, audit_pinned_wit_worlds};
use zryna_ir::{Expr, ExprId, ExprKind, Function, Program, Type};
use zryna_source::{SourceFileInput, SourceMap};

mod bounds;
mod quotas;
mod rejection;

fn node(id: &str) -> Instance {
    Instance {
        id: id.to_owned(),
        rows: BTreeSet::from([
            Row::UniversalJavaScript,
            Row::UniversalWebAssembly,
            Row::UniversalNative,
            Row::WitCommand,
            Row::WitServer,
            Row::JavaScriptBrowser,
            Row::JavaScriptNode,
        ]),
        requirements: BTreeSet::new(),
        restrictions: BTreeSet::from([
            Capability::Clock,
            Capability::Environment,
            Capability::Filesystem,
            Capability::Network,
            Capability::Randomness,
        ]),
        reservation: Reservation::default(),
    }
}

fn selection(row: Row) -> Selection {
    let world = match row {
        Row::WitCommand => Some("zryna:capability-profiles/command@0.1.0".to_owned()),
        Row::WitServer => Some("zryna:capability-profiles/server@0.1.0".to_owned()),
        _ => None,
    };
    Selection {
        row,
        policy_version: model::POLICY.to_owned(),
        world,
        approved: BTreeSet::new(),
        ceilings: policy::Policy::load().expect("pinned registry").limits(row),
    }
}

fn input(ids: &[&str], edges: &[(&str, &str)]) -> Input {
    Input {
        version: model::VERSION.to_owned(),
        root: ids[0].to_owned(),
        language: Language::I32V1,
        selections: vec![selection(Row::UniversalJavaScript)],
        instances: ids.iter().map(|id| node(id)).collect(),
        edges: edges.iter().map(|(from, to)| ((*from).to_owned(), (*to).to_owned())).collect(),
    }
}

fn verified_language(source: &str) -> VerifiedLanguage {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".to_owned(),
        text: source.to_owned(),
    }])
    .expect("source authority");
    let file = sources.verify_file_id(0).expect("source file");
    let span = sources.span(file, 0, u32::try_from(source.len()).expect("fixture")).expect("span");
    let program = zryna_ir::verify(
        Program {
            functions: vec![Function {
                name: "value".to_owned(),
                parameters: Vec::new(),
                return_type: Type::I32,
                expressions: vec![Expr { ty: Type::I32, span, kind: ExprKind::I32Literal(1) }],
                body: ExprId(0),
            }],
        },
        &sources,
    )
    .expect("verified program authority");
    VerifiedLanguage::I32V1 { program, sources }
}

fn wit_audit() -> zryna_backend_webassembly::WitWorldAudit {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = crate_root.join("../../spec/wit/capability-profiles-v1/worlds.wit");
    let mut sources = vec![WitSource::new(
        "spec/wit/capability-profiles-v1/worlds.wit",
        fs::read(root).expect("root WIT source"),
    )];
    let dependencies =
        crate_root.join("../zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies");
    for package_name in ["cli", "clocks", "filesystem", "http", "io", "random", "sockets"] {
        let mut files = fs::read_dir(dependencies.join(package_name))
            .expect("WASI sources")
            .map(|entry| entry.expect("source entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "wit"))
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            let name = file.file_name().expect("file name").to_string_lossy();
            sources.push(WitSource::new(
                format!("wasi/{package_name}/{name}"),
                fs::read(file).expect("WASI source"),
            ));
        }
    }
    audit_pinned_wit_worlds(&sources).expect("sealed WIT world authority")
}

fn authorities(input: &Input) -> Authorities {
    Authorities {
        instances: input
            .instances
            .iter()
            .map(|instance| {
                (
                    instance.id.clone(),
                    InstanceAuthority { programs: vec![verified_language(&instance.id)] },
                )
            })
            .collect(),
        wit: input.selections.iter().any(|selection| selection.world.is_some()).then(wit_audit),
    }
}

fn pure_claim(input: &Input) -> Claim {
    let authorities = authorities(input);
    Claim {
        binding: graph::validate(input)
            .expect("valid test shape")
            .binding(
                &authorities
                    .binding(&input.instances.iter().map(|node| node.id.clone()).collect())
                    .expect("authorities"),
            )
            .expect("binding"),
        summaries: input
            .instances
            .iter()
            .map(|node| (node.id.clone(), Summary::default()))
            .collect(),
        witnesses: BTreeMap::new(),
    }
}

fn verify(input: &Input, claim: &Claim) -> Result<ValidatedComposition, Vec<Diagnostic>> {
    super::verify(input, &authorities(input), claim)
}

fn clock() -> Requirement {
    Requirement {
        capability: Capability::Clock,
        interface: "wasi:clocks/monotonic-clock@0.2.12".to_owned(),
    }
}

fn diamond() -> (Input, Claim) {
    let mut input = input(&["A", "B", "C", "D"], &[("A", "B"), ("A", "C"), ("B", "D"), ("C", "D")]);
    input.selections = vec![selection(Row::WitCommand)];
    input.selections[0].approved.insert(clock());
    input.instances[3].requirements.insert(clock());
    input.instances[3].reservation.subscriptions = 1;
    let mut claim = pure_claim(&input);
    for summary in claim.summaries.values_mut() {
        summary.requirements.insert(clock());
        summary.quota[0] = 1;
    }
    claim.witnesses.insert(clock(), vec!["A".to_owned(), "B".to_owned(), "D".to_owned()]);
    (input, claim)
}

fn rejected(input: &Input, claim: &Claim, code: &str) -> Vec<Diagnostic> {
    let first = verify(input, claim).expect_err("hostile input must reject");
    assert_eq!(first[0].code(), code, "{first:?}");
    assert_eq!(first, verify(input, claim).expect_err("repeat rejection"));
    let valid = self::input(&["recovery"], &[]);
    assert!(verify(&valid, &pure_claim(&valid)).is_ok());
    first
}

#[test]
fn pure_closure_is_canonical_across_all_outputs_and_input_permutations() {
    let mut input = input(&["A", "B", "C"], &[("A", "B"), ("B", "C")]);
    input.selections = vec![
        selection(Row::UniversalJavaScript),
        selection(Row::UniversalWebAssembly),
        selection(Row::UniversalNative),
    ];
    let claim = pure_claim(&input);
    let result = verify(&input, &claim).expect("pure closure");
    assert!(result.requirements().is_empty());
    input.instances.reverse();
    input.edges.reverse();
    input.selections.reverse();
    assert!(result.revalidate(&input, &authorities(&input)).is_ok());
    assert!(verify(&input, &claim).is_ok());
}

#[test]
fn diamond_has_one_reservation_and_shortest_bytewise_witness() {
    let (mut input, claim) = diamond();
    let result = verify(&input, &claim).expect("diamond");
    assert_eq!(result.requirements(), &BTreeSet::from([clock()]));
    assert_eq!(result.witnesses[&clock()], ["A", "B", "D"]);
    input.instances.reverse();
    input.edges.reverse();
    assert_eq!(verify(&input, &claim).expect("permuted").witnesses, result.witnesses);
    input.edges.push(("A".to_owned(), "D".to_owned()));
    let mut shorter = claim;
    shorter.binding = pure_claim(&input).binding;
    shorter.witnesses.insert(clock(), vec!["A".to_owned(), "D".to_owned()]);
    assert!(verify(&input, &shorter).is_ok());
}

#[test]
fn registry_projection_is_exact_and_substitution_fails_closed() {
    let bytes = include_bytes!("../../../../tests/wit-capability-profiles-v1.json");
    let registry: serde_json::Value = serde_json::from_slice(bytes).expect("registry");
    let authority = policy::Policy::from_bytes(bytes).expect("exact registry");
    for (index, row) in [Row::WitBrowser, Row::WitCommand, Row::WitServer].into_iter().enumerate() {
        let profile = &registry["profiles"][index];
        for capability in profile["capabilities"].as_array().expect("capabilities") {
            let kind = match capability["id"].as_str().expect("id") {
                "clock" => Capability::Clock,
                "environment" => Capability::Environment,
                "filesystem" => Capability::Filesystem,
                "network" => Capability::Network,
                "randomness" => Capability::Randomness,
                _ => panic!("unknown capability"),
            };
            for interface in capability["interfaces"].as_array().expect("interfaces") {
                assert!(authority.admits(
                    row,
                    &Requirement {
                        capability: kind,
                        interface: interface.as_str().expect("interface").to_owned()
                    }
                ));
            }
        }
    }
    let mut substituted = bytes.to_vec();
    substituted.push(b' ');
    assert!(policy::Policy::from_bytes(&substituted).is_err());
    assert!(policy::Policy::from_bytes(bytes).is_ok());
}
