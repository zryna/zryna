//! Actual private filesystem fixtures exercise bytes, exact inventory and retained identities.

use super::*;
use cap_std::{ambient_authority, fs::Dir};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const DATA: &[u8] = b"audited material bytes";

struct Fixture {
    parent: PathBuf,
    path: PathBuf,
    root: Dir,
    uid: u32,
    gid: u32,
}
impl Fixture {
    fn new() -> Self {
        let parent = std::env::temp_dir().join(format!(
            "playground-material-audit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let path = parent.join("materials");
        fs::create_dir_all(path.join("bin")).expect("owned fixture directories");
        fs::write(path.join("bin/tool"), DATA).expect("real material bytes");
        fs::set_permissions(path.join("bin/tool"), fs::Permissions::from_mode(0o555))
            .expect("file mode");
        fs::set_permissions(path.join("bin"), fs::Permissions::from_mode(0o555))
            .expect("directory mode");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).expect("root mode");
        let metadata = fs::metadata(&path).expect("fixture owner");
        let root = Dir::open_ambient_dir(&path, ambient_authority()).expect("retained root");
        Self { parent, path, root, uid: metadata.uid(), gid: metadata.gid() }
    }
    fn bytes() -> Vec<u8> {
        format!(
            concat!(
                "{{\"format\":\"zryna.playground-materials.v1\",\"version\":1,",
                "\"role\":\"compiler\",\"directories\":[\"bin\"],\"files\":[{{",
                "\"path\":\"bin/tool\",\"bytes\":{},\"sha256\":\"{:x}\",\"mode\":365}}]}}"
            ),
            DATA.len(),
            Sha256::digest(DATA)
        )
        .into_bytes()
    }
    fn verify(&self) -> Result<Receipt, &'static str> {
        // Unit fixtures retain their real owner. Production entry always supplies exactly 0:0.
        audit(&self.root, &Self::bytes(), self.uid, self.gid)
    }
    fn writable(&self) {
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755))
            .expect("fixture root write");
        fs::set_permissions(self.path.join("bin"), fs::Permissions::from_mode(0o755))
            .expect("fixture dir write");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fn release(path: &Path) {
            for entry in fs::read_dir(path).expect("owned fixture inventory") {
                let entry = entry.expect("owned entry");
                if entry.file_type().expect("no-follow kind").is_dir() {
                    fs::set_permissions(entry.path(), fs::Permissions::from_mode(0o755))
                        .expect("release dir");
                    release(&entry.path());
                }
            }
        }
        release(&self.parent);
        fs::remove_dir_all(&self.parent).expect("remove only test-owned fixture");
    }
}

#[test]
fn scoped_published_provider_paths_receive_a_complete_filesystem_audit() {
    let fixture = Fixture::new();
    fixture.writable();
    let directories = ["node_modules", "node_modules/@typescript", "node_modules/@typescript/old"];
    fs::create_dir_all(fixture.path.join(directories[2])).expect("scoped provider directories");
    fs::rename(
        fixture.path.join("bin/tool"),
        fixture.path.join(directories[2]).join("typescript.js"),
    )
    .expect("actual scoped provider bytes");
    fs::remove_dir(fixture.path.join("bin")).expect("remove obsolete fixture directory");
    for path in directories {
        fs::set_permissions(fixture.path.join(path), fs::Permissions::from_mode(0o555))
            .expect("seal scoped directory");
    }
    fs::set_permissions(&fixture.path, fs::Permissions::from_mode(0o555)).expect("seal root");
    let text = String::from_utf8(Fixture::bytes())
        .expect("canonical fixture")
        .replace(
            "[\"bin\"]",
            "[\"node_modules\",\"node_modules/@typescript\",\"node_modules/@typescript/old\"]",
        )
        .replace("bin/tool", "node_modules/@typescript/old/typescript.js");
    let receipt = audit(&fixture.root, text.as_bytes(), fixture.uid, fixture.gid)
        .expect("authenticate scoped provider content, metadata and exact inventory");
    assert_eq!(receipt.files, 1);
    assert_eq!(receipt.bytes, DATA.len() as u64);
    for path in [
        "node_modules/@typescript/../old/typescript.js",
        "node_modules/@typescript//typescript.js",
        "node_modules/@typescript/old/../../typescript.js",
        "/node_modules/@typescript/old/typescript.js",
    ] {
        assert!(
            manifest::parse(
                text.replace("node_modules/@typescript/old/typescript.js", path).as_bytes()
            )
            .is_err()
        );
    }
}

#[test]
fn complete_actual_material_bytes_and_inventory_issue_bound_receipt() {
    let fixture = Fixture::new();
    let receipt = fixture.verify().expect("real ordinary material audit");
    assert_eq!(receipt.files, 1);
    assert_eq!(receipt.bytes, DATA.len() as u64);
    assert_eq!(receipt.root_sha256, format!("{:x}", Sha256::digest(Fixture::bytes())));
    if fixture.uid != 0 || fixture.gid != 0 {
        assert!(
            audit(&fixture.root, &Fixture::bytes(), 0, 0).is_err(),
            "production owner never inferred"
        );
    }
}

#[test]
fn actual_content_mode_and_hardlink_substitutions_reject() {
    let fixture = Fixture::new();
    fs::set_permissions(fixture.path.join("bin/tool"), fs::Permissions::from_mode(0o644))
        .expect("wrong mode");
    assert!(fixture.verify().is_err());
    fs::write(fixture.path.join("bin/tool"), vec![b'x'; DATA.len()])
        .expect("same-size changed bytes");
    fs::set_permissions(fixture.path.join("bin/tool"), fs::Permissions::from_mode(0o555))
        .expect("restore mode");
    assert!(fixture.verify().is_err());
    fs::set_permissions(fixture.path.join("bin/tool"), fs::Permissions::from_mode(0o755))
        .expect("fixture write");
    fs::write(fixture.path.join("bin/tool"), DATA).expect("restore actual bytes");
    fs::set_permissions(fixture.path.join("bin/tool"), fs::Permissions::from_mode(0o555))
        .expect("restore mode");
    fs::hard_link(fixture.path.join("bin/tool"), fixture.parent.join("alias"))
        .expect("actual hardlink");
    assert!(fixture.verify().is_err(), "undeclared second link refuses executable identity");
}

#[test]
fn extra_empty_directory_symlink_and_named_root_substitution_fail_closed() {
    let fixture = Fixture::new();
    fixture.writable();
    fs::create_dir(fixture.path.join("extra")).expect("unknown empty directory");
    fs::set_permissions(&fixture.path, fs::Permissions::from_mode(0o555)).expect("root seal");
    assert!(fixture.verify().is_err());
    fixture.writable();
    fs::remove_dir(fixture.path.join("extra")).expect("remove test-owned extra");
    fs::remove_file(fixture.path.join("bin/tool")).expect("replace actual leaf");
    fs::write(fixture.parent.join("elsewhere"), DATA).expect("link target fixture");
    std::os::unix::fs::symlink(fixture.parent.join("elsewhere"), fixture.path.join("bin/tool"))
        .expect("leaf symlink");
    fs::set_permissions(fixture.path.join("bin"), fs::Permissions::from_mode(0o555))
        .expect("reseal dir");
    fs::set_permissions(&fixture.path, fs::Permissions::from_mode(0o555)).expect("reseal root");
    assert!(fixture.verify().is_err());
    fs::rename(&fixture.path, fixture.parent.join("retained-original"))
        .expect("replace named root");
    fs::create_dir(&fixture.path).expect("foreign replacement");
    let replacement =
        Dir::open_ambient_dir(&fixture.path, ambient_authority()).expect("foreign root handle");
    assert!(filesystem::same_directory(&fixture.root, &replacement).is_err());
}

#[test]
fn canonical_manifest_rejects_extra_fields_paths_role_limits_and_aliases() {
    let bytes = Fixture::bytes();
    assert!(manifest::parse(&[bytes.as_slice(), b"\n"].concat()).is_err());
    let text = String::from_utf8(bytes).expect("canonical fixture");
    for mutant in [
        text.replace("\"version\":1", "\"version\":1,\"version\":1"),
        text.replace("\"role\":\"compiler\"", "\"role\":\"unknown\""),
        text.replace("\"path\":\"bin/tool\"", "\"path\":\"../tool\""),
        text.replace("\"mode\":365", "\"mode\":493"),
        text.replace(&format!("\"bytes\":{}", DATA.len()), "\"bytes\":134217729"),
        text.replace("\"directories\":[\"bin\"]", "\"directories\":[\"bin\",\"bin\"]"),
        text.replace("\"version\":1", "\"version\":1,\"extra\":true"),
    ] {
        assert!(manifest::parse(mutant.as_bytes()).is_err(), "{mutant}");
    }
}

#[test]
fn streaming_authenticates_bytes_beyond_two_complete_read_buffers() {
    use std::io::{Seek, SeekFrom, Write};
    let fixture = Fixture::new();
    let path = fixture.path.join("bin/tool");
    let data = vec![0x5a; 2 * 65_536 + 1];
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("fixture file write");
    fs::write(&path, &data).expect("multiple real read blocks");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).expect("seal actual bytes");
    let bytes = String::from_utf8(Fixture::bytes())
        .expect("canonical manifest")
        .replace(&format!("\"bytes\":{}", DATA.len()), &format!("\"bytes\":{}", data.len()))
        .replace(&format!("{:x}", Sha256::digest(DATA)), &format!("{:x}", Sha256::digest(&data)))
        .into_bytes();
    let receipt = audit(&fixture.root, &bytes, fixture.uid, fixture.gid)
        .expect("all material chunks authenticate");
    assert_eq!(receipt.bytes, data.len() as u64);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("fixture file write");
    let mut file = fs::OpenOptions::new().write(true).open(&path).expect("fixture writer");
    file.seek(SeekFrom::Start(65_536 + 5)).expect("late-block mutation");
    file.write_all(&[0x5b]).expect("change beyond first buffer");
    drop(file);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).expect("restore exact mode");
    assert!(audit(&fixture.root, &bytes, fixture.uid, fixture.gid).is_err());
}
