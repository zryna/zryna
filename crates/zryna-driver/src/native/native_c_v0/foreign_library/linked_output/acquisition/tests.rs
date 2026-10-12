use super::*;
use std::{
    fs,
    os::unix::fs::symlink,
    sync::atomic::{AtomicUsize, Ordering},
};
mod origins;

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "zryna-materials-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("new private directory");
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("fixture cleanup");
    }
}
fn material(id: &str, bytes: usize) -> MaterialExpectation {
    MaterialExpectation {
        id: id.into(),
        role: Role::Static,
        path: id.into(),
        bytes,
        sha256: Sha256::digest(b"abc").into(),
        trace_path: Some(format!("/independent/{id}")),
        original_path: format!("/independent/{id}"),
    }
}
fn expectation(root: &std::path::Path) -> Expectation {
    Expectation {
        format: "zryna.private-link-inventory.v1".into(),
        target: "x86_64-unknown-linux-gnu".into(),
        root: root.into(),
        materials: vec![material("archive", 3)],
        linker_order: vec!["material:archive".into()],
        providers: closure::ProviderExpectation {
            interpreter: "/lib64/ld-linux-x86-64.so.2".into(),
            needed: vec![],
            providers: vec![],
            allow_weak: false,
            allow_ifunc: false,
            allow_tls: false,
        },
    }
}

#[test]
fn selected_reader_exact_length_first_extra_and_special_files() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("regular"), b"abc").expect("write exact independent regular fixture");
    let root = files::Root::open(&fixture.0).expect("open canonical private fixture root");
    assert_eq!(root.read("regular", 3).expect("read exact three-byte fixture"), b"abc");
    assert!(root.read("regular", 2).is_err());
    root.compare("regular", b"abc").expect("exact independent bytes compare");
    assert!(root.compare("regular", b"abd").is_err());
    assert!(root.compare("regular", b"ab").is_err());
    assert!(root.compare("regular", b"abcd").is_err());
    fs::write(fixture.0.join("empty"), []).expect("create zero-byte hostile leaf");
    assert!(root.read("empty", 3).is_err());
    symlink("regular", fixture.0.join("alias")).expect("create hostile leaf alias");
    assert!(root.read("alias", 3).is_err());
    fs::create_dir(fixture.0.join("directory")).expect("create nonregular directory leaf");
    assert!(root.read("directory", 3).is_err());
    // A bounded nonblocking control must reject without waiting for a writer.
    nix::unistd::mkfifo(
        &fixture.0.join("fifo"),
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )
    .expect("create bounded nonblocking FIFO control");
    assert!(root.read("fifo", 3).is_err());
    for path in ["", "../regular", "directory/../regular", "/regular", "./regular"] {
        assert!(root.read(path, 3).is_err());
    }
}

#[test]
fn selected_reader_rejects_symlink_and_replaced_ancestors() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join("parent")).expect("create actual selected parent directory");
    fs::write(fixture.0.join("parent/data"), b"abc")
        .expect("write independently fixed ancestor fixture");
    symlink("parent", fixture.0.join("alias")).expect("create hostile directory alias");
    assert!(files::Root::open(&fixture.0.join("alias")).is_err());
    let root = files::Root::open(&fixture.0).expect("open canonical private fixture root");
    assert!(root.read("alias/data", 3).is_err());
    let original = files::Root::open(&fixture.0.join("parent"))
        .expect("capture original selected directory identity");
    fs::rename(fixture.0.join("parent"), fixture.0.join("old"))
        .expect("replace original named ancestor");
    fs::create_dir(fixture.0.join("parent")).expect("create actual selected parent directory");
    fs::write(fixture.0.join("parent/data"), b"abc")
        .expect("write independently fixed ancestor fixture");
    assert!(original.read("data", 3).is_err());
    assert!(files::Root::open(&fixture.0.join("parent/.")).is_err());
}

#[test]
fn closed_manifest_exact_count_file_and_total_limits() {
    let fixture = Fixture::new();
    let mut expected = expectation(&fixture.0);
    expected.materials =
        (0..MAX_MATERIALS).map(|index| material(&format!("m{index}"), 1)).collect();
    expected.linker_order = vec!["material:m0".into()];
    inventory::validate(&expected).expect("independent exact-bound descriptor is valid");
    expected.materials.push(material("extra", 1));
    assert!(inventory::validate(&expected).is_err());
    expected = expectation(&fixture.0);
    expected.materials[0].role = Role::Tool;
    expected.materials[0].bytes = MAX_TOOL_BYTES;
    inventory::validate(&expected).expect("independent exact-bound descriptor is valid");
    expected.materials[0].bytes += 1;
    assert!(inventory::validate(&expected).is_err());
    expected.materials[0].role = Role::Static;
    expected.materials[0].bytes = crate::native::MAX_NATIVE_EXECUTABLE_BYTES;
    inventory::validate(&expected).expect("independent exact-bound descriptor is valid");
    expected.materials[0].bytes += 1;
    assert!(inventory::validate(&expected).is_err());
    expected.materials =
        (0..4).map(|index| material(&format!("m{index}"), MAX_TOTAL_BYTES / 4)).collect();
    expected.linker_order = vec!["material:m0".into()];
    inventory::validate(&expected).expect("independent exact-bound descriptor is valid");
    expected.materials.push(material("extra", 1));
    assert!(inventory::validate(&expected).is_err());
}

#[test]
fn closed_manifest_case_alias_parent_and_unknown_order_reject() {
    let fixture = Fixture::new();
    let original = expectation(&fixture.0);
    for paths in [("dir/x", "Dir/y"), ("dir", "dir/y"), ("dir", "DIR"), ("../escape", "other")] {
        let mut expected = original.clone();
        expected.materials[0].path = paths.0.into();
        let mut other = material("other", 3);
        other.path = paths.1.into();
        expected.materials.push(other);
        assert!(inventory::validate(&expected).is_err());
    }
    for slot in ["unknown", "material:absent", "member:archive:bad(member)", "member:absent:x"] {
        let mut expected = original.clone();
        expected.linker_order = vec![slot.into()];
        assert!(inventory::validate(&expected).is_err());
    }
    let mut expected = original;
    expected.materials[0].path = "dir/x".into();
    let mut other = material("other", 3);
    other.path = "dir/y".into();
    expected.materials.push(other);
    inventory::validate(&expected).expect("independent exact-bound descriptor is valid");
}

#[test]
fn ordered_occurrences_and_archive_members_are_not_sets() {
    let fixture = Fixture::new();
    let mut expected = expectation(&fixture.0);
    expected.linker_order = vec![
        "material:archive".into(),
        "client".into(),
        "material:archive".into(),
        "member:archive:unit.o".into(),
    ];
    let objects = [("client", "/original/client.o".into())];
    let actual = [
        "/independent/archive",
        "/original/client.o",
        "/independent/archive",
        "(/independent/archive)unit.o",
    ]
    .map(String::from);
    inventory::check_order(&expected, &actual, &objects)
        .expect("exact repeated ordered trace matches");
    assert!(inventory::check_order(&expected, &actual[..3], &objects).is_err());
    let mut changed = actual.clone();
    changed.swap(0, 1);
    assert!(inventory::check_order(&expected, &changed, &objects).is_err());
}

#[test]
fn expectation_raw_pin_unknown_fields_and_file_substitution_reject() {
    let fixture = Fixture::new();
    let manifest = fixture.0.join("expectation");
    let bytes = b"{}";
    fs::write(&manifest, bytes).expect("write independently authored manifest bytes");
    assert!(read_expectation(&manifest, &Sha256::digest(bytes).into()).is_err());
    assert!(read_expectation(&manifest, &Sha256::digest(b"different").into()).is_err());
    let mut value = serde_json::json!({"format":"zryna.private-link-inventory.v1", "target":"x86_64-unknown-linux-gnu", "root":fixture.0,
        "materials":[{"id":"archive","role":"static","path":"archive","bytes":3,"sha256":<[u8;32]>::from(Sha256::digest(b"abc")),"trace_path":"/independent/archive","original_path":"/independent/archive"}],
        "linker_order":["material:archive"],"providers":{"interpreter":"/lib64/ld-linux-x86-64.so.2","needed":[],"providers":[],"allow_weak":false,"allow_ifunc":false,"allow_tls":false}});
    let bytes = serde_json::to_vec(&value).expect("encode independent closed manifest fixture");
    fs::write(&manifest, &bytes).expect("retain exact authored manifest wire");
    read_expectation(&manifest, &Sha256::digest(&bytes).into())
        .expect("closed independently pinned manifest parses");
    value["approval"] = true.into();
    let bytes = serde_json::to_vec(&value).expect("encode independent closed manifest fixture");
    fs::write(&manifest, &bytes).expect("retain exact authored manifest wire");
    assert!(read_expectation(&manifest, &Sha256::digest(&bytes).into()).is_err());
}

#[test]
fn declared_tool_cardinality_exact_eight_and_first_extra_rejects() {
    let fixture = Fixture::new();
    let mut expected = expectation(&fixture.0);
    expected.materials = (0..MAX_TOOL_MATERIALS)
        .map(|index| {
            let mut item = material(&format!("tool{index}"), 1);
            item.role = Role::Tool;
            item
        })
        .collect();
    expected.linker_order = vec!["material:tool0".into()];
    inventory::validate(&expected).expect("independent exact-bound descriptor is valid");
    let mut extra = material("extra", 1);
    extra.role = Role::Tool;
    expected.materials.push(extra);
    assert!(inventory::validate(&expected).is_err());
}
