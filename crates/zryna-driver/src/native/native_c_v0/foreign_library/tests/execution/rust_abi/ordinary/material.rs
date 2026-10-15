//! Independently pinned ordinary measurement data; no compiler capability is constructed.
use super::*;
use std::path::{Path, PathBuf};

pub(super) const OBJECT_SHA: &str =
    "6a77f351de6feba50f743838545794a14b833112952455614e31d8be410d1014";
const SOURCE_SHA: &str = "26c4f32993bd1aabb118b8a3dfe66902bc623852564f562776218c366758826b";
pub(super) fn check_source(source: &[u8]) -> Result<(), Diagnostic> {
    check(source, 1696, SOURCE_SHA)
}
pub(super) fn capture_data(
    requirements: &HandleLinkRequirements,
    object: &[u8],
    source: &[u8],
) -> Result<CapturedRustProfileLibrary, Diagnostic> {
    check_source(source)?;
    if object.len() > zryna_backend_native::MAX_NATIVE_OBJECT_BYTES {
        return Err(rejected());
    }
    targets::compare_functions(reference::ORIGINAL_RUST_OBJECT, object)?;
    let object_sha = sha(object);
    let source_sha = sha(source);
    capture_rust_profile_library(
        requirements,
        &RustProfileLibraryInput {
            library: ForeignLibraryInput {
                library_id: capture::LIBRARY_ID,
                header_bytes: capture::HEADER,
                policy_bytes: capture::POLICY,
                object_bytes: object,
                object_size: object.len(),
                object_sha256: &object_sha,
                dependency_symbols: &[],
            },
            source_bytes: source,
            source_size: source.len(),
            source_sha256: &source_sha,
        },
    )
}
fn check(bytes: &[u8], length: usize, expected: &str) -> Result<(), Diagnostic> {
    if bytes.len() != length || digest(bytes) != expected {
        return Err(rejected());
    }
    Ok(())
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ToolPin {
    path: PathBuf,
    length: usize,
    identity: String,
    digest: String,
}
impl ToolPin {
    fn observe(path: PathBuf) -> Result<Self, Diagnostic> {
        Self::observe_bounded(path, 64 * 1024 * 1024)
    }
    fn observe_bounded(path: PathBuf, limit: usize) -> Result<Self, Diagnostic> {
        let metadata = std::fs::symlink_metadata(&path).map_err(|_| rejected())?;
        let length = usize::try_from(metadata.len()).map_err(|_| rejected())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || length > limit {
            return Err(rejected());
        }
        let observed_identity = identity(&metadata);
        let digest = hash(&path, length)?;
        if observed_identity != identity(&std::fs::symlink_metadata(&path).map_err(|_| rejected())?)
        {
            return Err(rejected());
        }
        Ok(Self { path, length, identity: observed_identity, digest })
    }
    fn check(&self) -> Result<(), Diagnostic> {
        let current = Self::observe_bounded(self.path.clone(), self.length)?;
        if current.length != self.length
            || current.identity != self.identity
            || current.digest != self.digest
        {
            return Err(rejected());
        }
        Ok(())
    }
}
fn identity(m: &std::fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt as _;
    format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        m.dev(),
        m.ino(),
        m.len(),
        m.mode(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec()
    )
}
#[derive(Clone, Debug)]
pub(super) struct OrdinaryTools {
    pub(super) native: native::LinuxX8664LinkToolchain,
    pub(super) objdump: PathBuf,
    pins: Vec<ToolPin>,
}
impl OrdinaryTools {
    pub(super) fn acquire() -> Result<Self, Diagnostic> {
        let native =
            native::discover_linux_native_toolchain(native::NativeProcessLimits::default())?;
        let objdump = std::fs::canonicalize("/usr/bin/objdump").map_err(|_| rejected())?;
        let pins = [native.driver.clone(), native.linker.clone(), objdump.clone()]
            .into_iter()
            .map(ToolPin::observe)
            .collect::<Result<Vec<_>, _>>()?;
        let value = Self { native, objdump, pins };
        value.check()?;
        Ok(value)
    }
    pub(super) fn check(&self) -> Result<(), Diagnostic> {
        if self.pins.len() != 3
            || self.pins[0].path != self.native.driver
            || self.pins[1].path != self.native.linker
            || self.pins[2].path != self.objdump
        {
            return Err(rejected());
        }
        native::revalidate_tool(&self.native.driver, &self.native.driver_identity)?;
        native::revalidate_tool(&self.native.linker, &self.native.linker_identity)?;
        for pin in &self.pins {
            pin.check()?;
        }
        Ok(())
    }
    pub(super) fn report(&self) -> serde_json::Value {
        serde_json::json!({"scope":"ordinary host selected tool observations, not upstream/provider qualification","complete_input_use":false,"pins":self.pins.iter().map(|p|serde_json::json!({"path":p.path,"bytes":p.length,"identity":p.identity,"sha256":p.digest})).collect::<Vec<_>>()})
    }
    pub(super) fn substituted_control(&self) -> Self {
        let mut value = self.clone();
        let replacement = if value.pins[0].digest.starts_with('0') { "1" } else { "0" };
        value.pins[0].digest.replace_range(..1, replacement);
        value
    }
}
pub(super) use super::rust_measurement::OrdinaryRustMeasurement;

#[derive(Clone)]
pub(super) struct RustTools {
    pub(super) root: PathBuf,
    pub(super) rustc: PathBuf,
    pins: Vec<ToolPin>,
}
impl RustTools {
    pub(super) fn acquire() -> Result<Self, Diagnostic> {
        let selected = std::env::var_os("RUSTUP_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|v| PathBuf::from(v).join(".rustup")))
            .ok_or_else(rejected)?
            .join("toolchains/1.97.1-x86_64-unknown-linux-gnu");
        let root = std::fs::canonicalize(selected).map_err(|_| rejected())?;
        let rustc = root.join("bin/rustc");
        let pins = [
            (rustc.clone(), 645_000),
            (root.join("lib/librustc_driver-832cf6cfb1386559.so"), 157_802_496),
        ]
        .into_iter()
        .map(|(p, n)| ToolPin::observe_bounded(p, n))
        .collect::<Result<Vec<_>, _>>()?;
        let value = Self { root, rustc, pins };
        value.check()?;
        Ok(value)
    }
    pub(super) fn check(&self) -> Result<(), Diagnostic> {
        let expected = [
            (
                self.root.join("bin/rustc"),
                645_000,
                "d3a664c970a9fd8361b64194861bebc1ae37b9054e5ee3400dc1c9e691797eea",
            ),
            (
                self.root.join("lib/librustc_driver-832cf6cfb1386559.so"),
                157_802_496,
                "b0726217c199be52842c1318a340f0fc68a2c2ca0e3fa4886191dbf6ee192a58",
            ),
        ];
        if self.rustc != expected[0].0 || self.pins.len() != 2 {
            return Err(rejected());
        }
        for (pin, (path, length, digest)) in self.pins.iter().zip(expected) {
            if pin.path != path || pin.length != length || pin.digest != digest {
                return Err(rejected());
            }
            pin.check()?;
        }
        Ok(())
    }
    pub(super) fn report(&self) -> serde_json::Value {
        serde_json::json!({"scope":"ordinary selected official-content Rust observations; outside private tool inventory caps","selected_sysroot":self.root,"compiler":self.rustc,"complete_input_use":false,"private_recipe_qualified":false,"pins":self.pins.iter().map(|p|serde_json::json!({"path":p.path,"bytes":p.length,"identity":p.identity,"sha256":p.digest})).collect::<Vec<_>>()})
    }
}
pub(super) fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Diagnostic> {
    export::read(path, limit)
}
fn hash(path: &Path, limit: usize) -> Result<String, Diagnostic> {
    export::Directory::open(path.parent().ok_or_else(rejected)?)?
        .hash(path.file_name().and_then(|s| s.to_str()).ok_or_else(rejected)?, limit)
}
pub(super) fn evidence_parent() -> Result<PathBuf, Diagnostic> {
    let parent = std::env::var_os("ZRYNA_RUST_ORDINARY_EVIDENCE_PARENT")
        .map_or_else(std::env::temp_dir, PathBuf::from);
    let source = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).map_err(|_| rejected())?;
    if parent.starts_with(source.parent().and_then(Path::parent).ok_or_else(rejected)?) {
        return Err(rejected());
    }
    export::Directory::open(&parent)?.revalidate()?;
    Ok(parent)
}
