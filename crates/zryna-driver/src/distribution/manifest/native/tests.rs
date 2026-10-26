use super::super::{Distribution, FileRecord, Recipe, Source, Target};
use super::{FORMAT, RECIPE_FORMAT, RECIPE_SHA256, required_paths};
use std::collections::BTreeMap;

fn tuple(path: &str) -> FileRecord {
    FileRecord {
        path: path.to_owned(),
        size: 1,
        sha256: "a".repeat(64),
        role: super::role(path).expect("native role").to_owned(),
        mode: 0o644,
        material: "source".to_owned(),
        licenses: vec!["LICENSE".to_owned()],
    }
}

fn native() -> Distribution {
    let (triple, archive, baseline): (&str, &str, &[(&str, &str)]) = if cfg!(windows) {
        (
            "x86_64-pc-windows-msvc",
            "zip",
            &[
                ("architecture", "x86_64"),
                ("os", "windows"),
                ("product", "windows-server"),
                ("runtime", "operating-system-ucrt"),
                ("version", "2022"),
            ],
        )
    } else {
        (
            "x86_64-unknown-linux-gnu",
            "tar-gzip",
            &[
                ("architecture", "x86_64"),
                ("distribution", "ubuntu"),
                ("os", "linux"),
                ("version", "24.04"),
            ],
        )
    };
    let mut files = required_paths().iter().map(|path| tuple(path)).collect::<Vec<_>>();
    files.push(tuple("licenses/rust/example-1.0.0/LICENSE"));
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Distribution {
        format: FORMAT.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        source: Source {
            repository: "https://github.com/zryna/zryna".to_owned(),
            r#ref: "refs/heads/main".to_owned(),
            commit: "1".repeat(40),
            tree: "2".repeat(40),
            epoch: 1,
        },
        target: Target {
            triple: triple.to_owned(),
            archive_format: archive.to_owned(),
            platform_baseline: baseline
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect::<BTreeMap<_, _>>(),
        },
        recipe: Recipe { format: RECIPE_FORMAT.to_owned(), sha256: RECIPE_SHA256.to_owned() },
        files,
    }
}

#[test]
fn native_candidate_has_no_bootstrap_or_node_capability() {
    let value = native();
    value.validate().expect("bounded native candidate");
    assert!(value.is_native_provider());
    assert!(!value.requires_bootstrap());
    assert!(value.provider_paths().expect("native provider paths").is_empty());
    assert!(value.node_path().is_err());
    let encoded = super::super::super::wire::canonical(&value).expect("canonical native record");
    Distribution::parse(&encoded).expect("ordinary native wire grammar");
}

#[test]
fn native_installation_rejects_runtime_bootstrap_and_unknown_payloads() {
    for path in [
        "runtime/node/bin/node",
        "lib/zryna/bootstrap/worker.mjs",
        "licenses/node-LICENSE",
        "licenses/typescript-LICENSE.txt",
        "bin/helper",
        "metadata/extra.json",
    ] {
        let mut value = native();
        let mut extra = tuple("LICENSE");
        extra.path = path.to_owned();
        value.files.push(extra);
        value.files.sort_by(|left, right| left.path.cmp(&right.path));
        assert!(value.validate().is_err(), "extra path {path}");
    }
}

#[test]
fn native_installation_rejects_missing_notices_and_provenance() {
    for path in required_paths().iter().copied().chain(["licenses/rust/example-1.0.0/LICENSE"]) {
        let mut value = native();
        value.files.retain(|file| file.path != path);
        assert!(value.validate().is_err(), "missing path {path}");
    }
}

#[test]
fn native_candidate_cannot_inherit_legacy_recipe_or_tag_authority() {
    let mut value = native();
    value.recipe.format = "zryna.distribution-recipe.v1".to_owned();
    assert!(value.validate().is_err());
    let mut value = native();
    value.recipe.sha256 = "f".repeat(64);
    assert!(value.validate().is_err());
    let mut value = native();
    value.source.r#ref = format!("refs/tags/v{}", env!("CARGO_PKG_VERSION"));
    assert!(value.validate().is_err());
    let mut value = native();
    value.format = "zryna.distribution.v1".to_owned();
    assert!(value.validate().is_err());
}

#[test]
fn native_candidate_retains_original_file_identity_and_alias_guards() {
    let mutations: [fn(&mut Vec<FileRecord>); 5] = [
        |files: &mut Vec<FileRecord>| files[0].sha256 = "A".repeat(64),
        |files: &mut Vec<FileRecord>| files[0].size = 0,
        |files: &mut Vec<FileRecord>| files[0].mode = 0o755,
        |files: &mut Vec<FileRecord>| files[0].path = "../LICENSE".to_owned(),
        |files: &mut Vec<FileRecord>| files.push(files[0].clone()),
    ];
    for mutate in mutations {
        let mut value = native();
        mutate(&mut value.files);
        assert!(value.validate().is_err());
    }
}

#[test]
fn native_source_ref_policy_is_closed_and_does_not_change_v1() {
    for source_ref in [
        "refs/heads/main",
        "refs/heads/codex/native-install",
        "refs/heads/codex/m414/v2.1",
        "refs/heads/fix/bounded-generics-profile",
        "refs/heads/fix/m416/v2.1",
    ] {
        let mut value = native();
        value.source.r#ref = source_ref.to_owned();
        value.validate().expect("actual candidate branch");
    }
    let mut invalid = vec![
        "refs/heads/codex/".to_owned(),
        "refs/heads/codex/a//b".to_owned(),
        "refs/heads/codex/../main".to_owned(),
        "refs/heads/codex/a..b".to_owned(),
        "refs/heads/codex/a.lock".to_owned(),
        "refs/heads/codex/a.".to_owned(),
        "refs/heads/codex/.hidden".to_owned(),
        "refs/heads/codex/UPPER".to_owned(),
        "refs/heads/codex/a@b".to_owned(),
        "refs/heads/codex/a\\b".to_owned(),
        "refs/pull/414/merge".to_owned(),
        "refs/heads/feature/native".to_owned(),
        "refs/tags/v0.2.3".to_owned(),
        "refs/heads/main/extra".to_owned(),
    ];
    invalid.push(format!("refs/heads/codex/{}", "a".repeat(65)));
    invalid.push(format!("refs/heads/codex/{}", ["a"; 10].join("/")));
    invalid.push(format!("refs/heads/codex/{0}/{0}/{0}", "a".repeat(64)));
    let fix_invalid = invalid
        .iter()
        .filter(|source_ref| source_ref.starts_with("refs/heads/codex/"))
        .map(|source_ref| source_ref.replacen("refs/heads/codex/", "refs/heads/fix/", 1))
        .collect::<Vec<_>>();
    invalid.extend(fix_invalid);
    for source_ref in invalid {
        let mut value = native();
        value.source.r#ref = source_ref;
        assert!(value.validate().is_err(), "unsafe source ref {}", value.source.r#ref);
    }
    assert!(super::super::source_ref_matches_version("refs/heads/main", "0.2.3"));
    assert!(super::super::source_ref_matches_version("refs/tags/v0.2.3", "0.2.3"));
    assert!(!super::super::source_ref_matches_version("refs/heads/codex/native", "0.2.3"));
    assert!(!super::super::source_ref_matches_version(
        "refs/heads/fix/bounded-generics-profile",
        "0.2.3"
    ));
}
