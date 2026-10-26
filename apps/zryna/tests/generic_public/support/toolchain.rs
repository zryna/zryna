//! Resolve actual pinned Rust metadata tools before the fixture removes executable search.
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

pub fn without_node_or_linker(command: &mut Command) {
    let root = super::repo();
    let selected = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let mut probe = Command::new(selected);
    probe.current_dir(&root).args(["--print", "sysroot"]);
    let sysroot = PathBuf::from(stdout(&mut probe)).canonicalize().expect("actual Rust sysroot");
    let rustc = executable(&sysroot, "rustc");
    let cargo = executable(&sysroot, "cargo");
    let pinned = include_str!("../../../../../rust-toolchain.toml")
        .lines()
        .find_map(|line| line.trim().strip_prefix("channel = "))
        .expect("checked-in exact toolchain channel")
        .trim_matches('"');
    for (name, executable) in [("rustc", &rustc), ("cargo", &cargo)] {
        let version = stdout(Command::new(executable).current_dir(&root).arg("--version"));
        let mut fields = version.split_whitespace();
        assert_eq!(fields.next(), Some(name));
        assert_eq!(
            fields.next(),
            Some(pinned),
            "actual direct metadata tool must match pinned toolchain"
        );
    }
    let actual = stdout(Command::new(&rustc).current_dir(&root).args(["--print", "sysroot"]));
    assert_eq!(PathBuf::from(actual).canonicalize().expect("direct tool sysroot"), sysroot);
    command
        .env("CARGO", cargo)
        .env("RUSTC", rustc)
        .env("PATH", "")
        .env_remove("NODE")
        .env_remove("ZRYNA_TEST_NODE");
    for name in ["CARGO", "RUSTC"] {
        let value = command
            .get_envs()
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value)
            .expect("explicit real metadata tool path");
        assert!(Path::new(value).is_absolute() && Path::new(value).is_file());
    }
    let path = command
        .get_envs()
        .find(|(key, _)| *key == "PATH")
        .and_then(|(_, value)| value)
        .expect("the test must explicitly remove executable search");
    assert!(path.is_empty());
}
fn executable(sysroot: &Path, name: &str) -> PathBuf {
    let name = if cfg!(windows) { format!("{name}.exe") } else { name.into() };
    let path = sysroot
        .join("bin")
        .join(name)
        .canonicalize()
        .expect("actual installed toolchain executable");
    assert!(path.is_absolute() && path.is_file());
    path
}
fn stdout(command: &mut Command) -> String {
    let observed = command.output().expect("actual installed Rust tool probe");
    assert!(
        observed.status.success(),
        "{:?}: {}",
        command,
        String::from_utf8_lossy(&observed.stderr)
    );
    assert!(
        observed.stderr.is_empty(),
        "{:?}: {}",
        command,
        String::from_utf8_lossy(&observed.stderr)
    );
    String::from_utf8(observed.stdout).expect("Rust tool output UTF8").trim().to_owned()
}
