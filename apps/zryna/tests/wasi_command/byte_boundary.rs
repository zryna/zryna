//! Real pinned frontend, public H1 factory and execution at the component byte boundary.

use super::support::{Case, guard, read_json};
use sha2::{Digest as _, Sha256};
use std::{fs, path::PathBuf};

const LIMIT: usize = 1_048_576;

struct Sources {
    directory: PathBuf,
    exact: String,
    extra: String,
    text: String,
}

impl Sources {
    fn new(case: &Case) -> Self {
        let relative = format!(".zryna/cache/{}-component-size", case.stem);
        let directory = case.root.join(&relative);
        fs::create_dir_all(directory.parent().expect("cache parent")).expect("fixture cache");
        fs::create_dir(&directory).expect("create-only unique source fixture");
        let source = |wide| {
            format!(
                "function retained(): bool {{ const payload: String = \"{}{}\"; return true; }}\n\
             export function main(): bool {{ return true; }}\n",
                "a".repeat(wide),
                "!".repeat(87_044 - wide)
            )
        };
        let text = source(4);
        let extra_text = source(5);
        assert_eq!(text.len(), 87_161);
        assert_eq!(extra_text.len(), text.len());
        assert_eq!(text.bytes().zip(extra_text.bytes()).filter(|(a, b)| a != b).count(), 1);
        fs::write(directory.join("exact.zry"), &text).expect("ordinary authored exact source");
        fs::write(directory.join("extra.zry"), extra_text).expect("ordinary first-extra source");
        Self {
            directory,
            exact: format!("{relative}/exact.zry"),
            extra: format!("{relative}/extra.zry"),
            text,
        }
    }
}

impl Drop for Sources {
    fn drop(&mut self) {
        // The fixture created only this exact unique directory and these two source files.
        let _ = fs::remove_file(self.directory.join("exact.zry"));
        let _ = fs::remove_file(self.directory.join("extra.zry"));
        let _ = fs::remove_dir(&self.directory);
    }
}

#[test]
fn actual_cli_h1_accepts1048576_and_rejects_source_emitted1048577_before_execution() {
    let _guard = guard();
    let accepted = Case::new();
    let sources = Sources::new(&accepted);
    let output = accepted.run(&sources.exact, None, &[]);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let response = read_json(&output.stdout);
    assert_eq!(response["execution"]["outcome"]["runReturn"], "ok");
    assert_eq!(response["execution"]["teardown"], "confirmed");
    let manifest_bytes = accepted.manifest();
    zryna_driver::decode_command_h1_manifest(&manifest_bytes).expect("complete closed H1 record");
    let record = read_json(&manifest_bytes);
    let component =
        fs::read(accepted.bundle.join("wasi-command").join(format!("{}.wasm", accepted.stem)))
            .expect("published actual factory component");
    assert_eq!(component.len(), LIMIT);
    assert_eq!(record["limits"]["maxComponentBytes"], LIMIT);
    assert_eq!(record["source"]["path"], sources.exact);
    assert_eq!(
        record["source"]["sha256"],
        format!("{:x}", Sha256::digest(sources.text.as_bytes()))
    );
    assert_eq!(record["component"]["sha256"], format!("{:x}", Sha256::digest(&component)));
    assert_eq!(record["source"]["programBinding"].as_str().expect("program binding").len(), 64);
    assert_eq!(record["composition"]["binding"].as_str().expect("composition binding").len(), 64);
    assert_eq!(record["component"]["world"], "zryna:capability-profiles/command@0.1.0");
    assert_eq!(record["component"]["wasiVersion"], "0.2.12");
    assert_eq!(record["component"]["witFileCount"], 34);
    assert_eq!(record["grants"]["requested"], serde_json::json!([]));
    assert_eq!(record["grants"]["effective"], serde_json::json!([]));
    assert_eq!(record["grants"]["staticQuota"], serde_json::json!(vec![0; 10]));
    assert_eq!(record["input"]["kind"], "none");
    assert_eq!(record["execution"]["runReturn"], "ok");
    assert_eq!(record["teardown"], "confirmed");

    let rejected = Case::new();
    let output = rejected.run(&sources.extra, None, &[]);
    assert_eq!(output.status.code(), Some(3));
    let response = read_json(&output.stdout);
    assert_eq!(response["ok"], false);
    assert_eq!(response["diagnostics"][0]["code"], "ZRYNA-W4102");
    assert!(response["manifest"].is_null());
    assert!(response["execution"].is_null(), "component rejection precedes engine execution");
    assert!(!rejected.bundle.exists(), "first extra publishes no record or component");
    let output = rejected.run("examples/wasi-command/pure.zry", None, &[]);
    assert!(output.status.success(), "next ordinary source recovers after size rejection");
    let recovery = read_json(&rejected.manifest());
    assert_eq!(recovery["execution"]["runReturn"], "ok");
    assert_eq!(recovery["teardown"], "confirmed");
}
