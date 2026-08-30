//! JavaScript-only source compilation and create-only artifact publication.

use std::{
    error::Error,
    fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
};

use zryna_backend_javascript::JavaScriptArtifact;
use zryna_diagnostics::Diagnostic;
use zryna_frontend::VerifiedFrontendProvider;
use zryna_source::SourceMap;

use crate::{SourceToIrError, compile_to_verified_ir};

/// File extension used for directly importable ECMAScript modules.
pub const JAVASCRIPT_ARTIFACT_EXTENSION: &str = "mjs";
/// Maximum portable artifact stem bytes accepted by the JavaScript publisher.
pub const MAX_JAVASCRIPT_ARTIFACT_STEM_BYTES: usize = 128;

const MAX_TEMPORARY_NAME_ATTEMPTS: u64 = 64;
const JAVASCRIPT_OUTPUT_RELATIVE_ROOT: &str = ".zryna/out";
static NEXT_TEMPORARY_NAME: AtomicU64 = AtomicU64::new(0);

/// Validated capability for the workspace's declared JavaScript output root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JavaScriptOutputRoot {
    path: PathBuf,
}

impl JavaScriptOutputRoot {
    /// Derives and validates the exact `.zryna/out` root of one absolute workspace path.
    ///
    /// The output root and every persistent ancestor must already be real directories rather
    /// than symbolic links, junctions, or other reparse points.
    ///
    /// # Errors
    ///
    /// Returns a stable diagnostic when the workspace path is relative or the declared output
    /// chain is unavailable, non-directory, or link-like.
    pub fn for_workspace(workspace_root: &Path) -> Result<Self, Diagnostic> {
        if !workspace_root.is_absolute() {
            return Err(invalid_output_root_error(
                "workspace root must be absolute before deriving the JavaScript output root",
            ));
        }
        let path = workspace_root.join(JAVASCRIPT_OUTPUT_RELATIVE_ROOT);
        validate_real_directory_chain(&path)?;
        Ok(Self { path })
    }

    /// Returns the exact validated `.zryna/out` path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn revalidate(&self) -> Result<(), Diagnostic> {
        validate_real_directory_chain(&self.path)
    }
}

/// One JavaScript module that has been atomically published at a new destination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedJavaScriptArtifact {
    path: PathBuf,
    diagnostics: Vec<Diagnostic>,
}

impl PublishedJavaScriptArtifact {
    /// Returns the exact published module path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns non-fatal publication diagnostics, such as a temporary-name cleanup warning.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Successful source-to-JavaScript compilation.
#[derive(Clone, Debug)]
pub struct JavaScriptBuildSuccess {
    artifact: PublishedJavaScriptArtifact,
    diagnostics: Vec<Diagnostic>,
}

impl JavaScriptBuildSuccess {
    /// Returns the atomically published ECMAScript module.
    #[must_use]
    pub const fn artifact(&self) -> &PublishedJavaScriptArtifact {
        &self.artifact
    }

    /// Returns deterministic non-fatal frontend and publication diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Failure before a new JavaScript artifact can be reported.
#[derive(Debug)]
pub enum JavaScriptBuildError {
    /// Source analysis, semantics, or IR verification failed.
    Source(SourceToIrError),
    /// The JavaScript backend rejected a verified program.
    Backend(Diagnostic),
    /// The complete module could not be published atomically.
    Publication(Diagnostic),
}

impl JavaScriptBuildError {
    /// Returns deterministic compiler diagnostics carried by this failure.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Source(error) => error.diagnostics(),
            Self::Backend(diagnostic) | Self::Publication(diagnostic) => {
                std::slice::from_ref(diagnostic)
            }
        }
    }
}

impl fmt::Display for JavaScriptBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(formatter),
            Self::Backend(diagnostic) => {
                write!(formatter, "JavaScript emission failed: {diagnostic}")
            }
            Self::Publication(diagnostic) => {
                write!(formatter, "JavaScript publication failed: {diagnostic}")
            }
        }
    }
}

impl Error for JavaScriptBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Backend(_) | Self::Publication(_) => None,
        }
    }
}

/// Compiles real source through verified IR and publishes one new ECMAScript module.
///
/// The output capability must identify an existing validated `.zryna/out` directory.
/// `artifact_stem` is one portable ASCII filename stem, not a path. Publication is create-only:
/// an existing destination is never replaced.
///
/// # Errors
///
/// Returns a phase-specific failure and never reports a new artifact unless complete bytes were
/// synchronized and atomically linked to the absent destination.
pub fn compile_javascript<Provider: VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
    output_root: &JavaScriptOutputRoot,
    artifact_stem: &str,
) -> Result<JavaScriptBuildSuccess, JavaScriptBuildError> {
    let compiled =
        compile_to_verified_ir(frontend, sources).map_err(JavaScriptBuildError::Source)?;
    let artifact = zryna_backend_javascript::emit(compiled.program())
        .map_err(JavaScriptBuildError::Backend)?;
    let published = publish_javascript(&artifact, output_root, artifact_stem)
        .map_err(JavaScriptBuildError::Publication)?;
    let mut diagnostics = compiled.diagnostics().to_vec();
    diagnostics.extend_from_slice(published.diagnostics());
    Ok(JavaScriptBuildSuccess { artifact: published, diagnostics })
}

/// Atomically publishes a complete JavaScript artifact at one absent destination.
///
/// The capability is revalidated immediately before use. The complete module is written and
/// synchronized through a create-new sibling temporary file, then published with a create-only
/// hard link. Existing files, directories, or links are never replaced.
///
/// # Errors
///
/// Returns a stable diagnostic for an unsafe name, invalid output root, existing destination, or
/// filesystem failure. A failed publication never creates or modifies the destination.
pub fn publish_javascript(
    artifact: &JavaScriptArtifact,
    output_root: &JavaScriptOutputRoot,
    artifact_stem: &str,
) -> Result<PublishedJavaScriptArtifact, Diagnostic> {
    validate_artifact_stem(artifact_stem)?;
    output_root.revalidate()?;
    let output_path = output_root.path();
    let destination = output_path.join(format!("{artifact_stem}.{JAVASCRIPT_ARTIFACT_EXTENSION}"));
    match fs::symlink_metadata(&destination) {
        Ok(_) => return Err(destination_exists_error(artifact_stem)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(publication_error("ZRYNA-D2003", artifact_stem, "inspect", &error));
        }
    }

    let (temporary_path, mut temporary) = create_temporary(output_path, artifact_stem)?;
    if let Err(error) = temporary.write_all(artifact.source.as_bytes()).and_then(|()| {
        temporary.flush()?;
        temporary.sync_all()
    }) {
        drop(temporary);
        let _ = fs::remove_file(&temporary_path);
        return Err(publication_error("ZRYNA-D2004", artifact_stem, "write", &error));
    }
    drop(temporary);

    if let Err(error) = fs::hard_link(&temporary_path, &destination) {
        let _ = fs::remove_file(&temporary_path);
        if error.kind() == io::ErrorKind::AlreadyExists {
            return Err(destination_exists_error(artifact_stem));
        }
        return Err(publication_error("ZRYNA-D2005", artifact_stem, "commit", &error));
    }

    let diagnostics = fs::remove_file(&temporary_path).err().map_or_else(Vec::new, |error| {
        vec![Diagnostic::warning(
            "ZRYNA-D2006",
            None,
            format!(
                "published JavaScript artifact '{artifact_stem}.{JAVASCRIPT_ARTIFACT_EXTENSION}' but could not remove its temporary name: {error}"
            ),
            "remove the sibling .zryna temporary file after confirming the published module",
        )]
    });
    Ok(PublishedJavaScriptArtifact { path: destination, diagnostics })
}

fn validate_artifact_stem(stem: &str) -> Result<(), Diagnostic> {
    let bytes = stem.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= MAX_JAVASCRIPT_ARTIFACT_STEM_BYTES
        && matches!(bytes[0], b'A'..=b'Z' | b'a'..=b'z' | b'_')
        && bytes[1..]
            .iter()
            .all(|byte| matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
        && !is_windows_device_stem(stem);
    if valid {
        Ok(())
    } else {
        Err(Diagnostic::error(
            "ZRYNA-D2001",
            None,
            "JavaScript artifact stem is not one portable filename component",
            "use 1 to 128 ASCII letters, digits, underscores, or hyphens, begin with a letter or underscore, and avoid reserved device names",
        ))
    }
}

fn is_windows_device_stem(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || upper.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}

fn validate_real_directory_chain(path: &Path) -> Result<(), Diagnostic> {
    for component in path.ancestors() {
        let metadata = fs::symlink_metadata(component).map_err(|error| {
            invalid_output_root_error(format!(
                "could not inspect JavaScript output path component '{}': {error}",
                component.display()
            ))
        })?;
        if !metadata.is_dir() || metadata_is_link_or_reparse(&metadata) {
            return Err(invalid_output_root_error(format!(
                "JavaScript output path component '{}' is not a real directory",
                component.display()
            )));
        }
    }
    Ok(())
}

fn invalid_output_root_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-D2002",
        None,
        message,
        "use an absolute workspace whose .zryna/out path and ancestors are real directories without links or reparse points",
    )
}

#[cfg(windows)]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn create_temporary(
    output_root: &Path,
    artifact_stem: &str,
) -> Result<(PathBuf, fs::File), Diagnostic> {
    for _ in 0..MAX_TEMPORARY_NAME_ATTEMPTS {
        let sequence = NEXT_TEMPORARY_NAME.fetch_add(1, Ordering::Relaxed);
        let path =
            output_root.join(format!(".zryna-{artifact_stem}-{}-{sequence}.tmp", process::id()));
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(publication_error(
                    "ZRYNA-D2003",
                    artifact_stem,
                    "create a temporary file for",
                    &error,
                ));
            }
        }
    }
    Err(Diagnostic::error(
        "ZRYNA-D2003",
        None,
        format!(
            "could not reserve a temporary name for '{artifact_stem}.{JAVASCRIPT_ARTIFACT_EXTENSION}'"
        ),
        "remove stale sibling .zryna temporary files and retry",
    ))
}

fn destination_exists_error(artifact_stem: &str) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-D2007",
        None,
        format!(
            "JavaScript artifact '{artifact_stem}.{JAVASCRIPT_ARTIFACT_EXTENSION}' already exists"
        ),
        "choose a fresh output stage; create-only publication never replaces an existing artifact",
    )
}

fn publication_error(
    code: &str,
    artifact_stem: &str,
    operation: &str,
    error: &io::Error,
) -> Diagnostic {
    Diagnostic::error(
        code,
        None,
        format!(
            "could not {operation} JavaScript artifact '{artifact_stem}.{JAVASCRIPT_ARTIFACT_EXTENSION}': {error}"
        ),
        "verify output-directory permissions and retry in a fresh declared output stage",
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::JavaScriptOutputRoot;
    use zryna_backend_javascript::JavaScriptArtifact;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TemporaryRoot {
        workspace: PathBuf,
        output: JavaScriptOutputRoot,
    }

    impl TemporaryRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let workspace = std::env::temp_dir()
                .join(format!("zryna-javascript-{}-{label}-{sequence}", std::process::id()));
            let output_path = workspace.join(super::JAVASCRIPT_OUTPUT_RELATIVE_ROOT);
            fs::create_dir_all(&output_path).expect("declared fixture output must be created");
            let output = JavaScriptOutputRoot::for_workspace(&workspace)
                .expect("fixture output capability must validate");
            assert_eq!(output.path(), workspace.join(super::JAVASCRIPT_OUTPUT_RELATIVE_ROOT));
            Self { workspace, output }
        }

        fn path(&self) -> &Path {
            self.output.path()
        }

        const fn output(&self) -> &JavaScriptOutputRoot {
            &self.output
        }

        fn workspace_path(&self) -> &Path {
            &self.workspace
        }
    }

    impl Drop for TemporaryRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.workspace);
        }
    }

    #[test]
    fn publication_is_create_only_complete_and_cleans_its_temporary_name() {
        let root = TemporaryRoot::new("publish");
        let artifact = JavaScriptArtifact { source: "export const value = 42;\n".to_owned() };

        let published = super::publish_javascript(&artifact, root.output(), "module")
            .expect("fresh destination must publish");

        assert_eq!(published.path(), root.path().join("module.mjs"));
        assert!(published.diagnostics().is_empty());
        assert_eq!(
            fs::read(published.path()).expect("published bytes"),
            artifact.source.as_bytes()
        );
        assert_eq!(
            fs::read_dir(root.path()).expect("fixture listing").count(),
            1,
            "the sibling temporary name must be removed"
        );
    }

    #[test]
    fn publication_preserves_an_existing_destination() {
        let root = TemporaryRoot::new("existing");
        let destination = root.path().join("module.mjs");
        fs::write(&destination, b"sentinel").expect("sentinel must be written");
        let artifact = JavaScriptArtifact { source: "replacement\n".to_owned() };

        let diagnostic = super::publish_javascript(&artifact, root.output(), "module")
            .expect_err("create-only publication must refuse replacement");

        assert_eq!(diagnostic.code(), "ZRYNA-D2007");
        assert_eq!(fs::read(destination).expect("sentinel must remain"), b"sentinel");
        assert_eq!(fs::read_dir(root.path()).expect("fixture listing").count(), 1);
    }

    #[test]
    fn publication_rejects_unsafe_names_and_invalid_roots_without_output() {
        let root = TemporaryRoot::new("invalid");
        let artifact = JavaScriptArtifact { source: "export {};\n".to_owned() };
        for invalid in ["", ".", "../escape", "a/b", "a\\b", "1module", "con", "LPT9"] {
            let diagnostic = super::publish_javascript(&artifact, root.output(), invalid)
                .expect_err("unsafe stem must fail");
            assert_eq!(diagnostic.code(), "ZRYNA-D2001");
        }
        let diagnostic = JavaScriptOutputRoot::for_workspace(Path::new("."))
            .expect_err("relative workspace root must fail");
        assert_eq!(diagnostic.code(), "ZRYNA-D2002");
        fs::remove_dir(root.path()).expect("empty output root must be removable");
        let diagnostic = JavaScriptOutputRoot::for_workspace(root.workspace_path())
            .expect_err("missing declared output root must fail");
        assert_eq!(diagnostic.code(), "ZRYNA-D2002");
    }

    #[cfg(unix)]
    #[test]
    fn publication_rejects_a_linked_root_and_preserves_a_linked_destination() {
        use std::os::unix::fs::symlink;

        let root = TemporaryRoot::new("links");
        fs::remove_dir(root.path()).expect("empty output root must be removable");
        let real_output = root.workspace_path().join("real-output");
        fs::create_dir(&real_output).expect("real output directory must be created");
        symlink(&real_output, root.path()).expect("output-root link must be created");
        let artifact = JavaScriptArtifact { source: "export {};\n".to_owned() };

        let diagnostic = super::publish_javascript(&artifact, root.output(), "module")
            .expect_err("linked output root must fail");
        assert_eq!(diagnostic.code(), "ZRYNA-D2002");
        assert!(fs::read_dir(&real_output).expect("real output listing").next().is_none());

        fs::remove_file(root.path()).expect("output-root link must be removed");
        fs::create_dir(root.path()).expect("real output root must be restored");
        let output = JavaScriptOutputRoot::for_workspace(root.workspace_path())
            .expect("restored output capability must validate");
        let sentinel = root.workspace_path().join("sentinel");
        fs::write(&sentinel, b"sentinel").expect("sentinel must be written");
        let linked_destination = root.path().join("module.mjs");
        symlink(&sentinel, &linked_destination).expect("destination link must be created");
        let diagnostic = super::publish_javascript(&artifact, &output, "module")
            .expect_err("linked destination must not be replaced");
        assert_eq!(diagnostic.code(), "ZRYNA-D2007");
        assert_eq!(fs::read(&sentinel).expect("sentinel must remain"), b"sentinel");
        assert!(
            fs::symlink_metadata(linked_destination)
                .expect("destination link must remain")
                .file_type()
                .is_symlink()
        );

        fs::remove_file(root.path().join("module.mjs")).expect("destination link must be removed");
        fs::remove_dir(root.path()).expect("output root must be removable");
        fs::remove_dir(root.workspace_path().join(".zryna"))
            .expect("metadata directory must be removable");
        let replacement_metadata = root.workspace_path().join("replacement-metadata");
        fs::create_dir_all(replacement_metadata.join("out"))
            .expect("replacement output must be created");
        symlink(&replacement_metadata, root.workspace_path().join(".zryna"))
            .expect("metadata ancestor link must be created");
        let diagnostic = super::publish_javascript(&artifact, root.output(), "module")
            .expect_err("linked ancestor introduced after validation must fail");
        assert_eq!(diagnostic.code(), "ZRYNA-D2002");

        let real_parent = root.workspace_path().join("real-parent");
        let nested_workspace = real_parent.join("workspace");
        fs::create_dir_all(nested_workspace.join(super::JAVASCRIPT_OUTPUT_RELATIVE_ROOT))
            .expect("nested declared output must be created");
        let linked_parent = root.workspace_path().join("linked-parent");
        symlink(&real_parent, &linked_parent).expect("ancestor link must be created");
        let diagnostic = JavaScriptOutputRoot::for_workspace(&linked_parent.join("workspace"))
            .expect_err("linked ancestor must fail");
        assert_eq!(diagnostic.code(), "ZRYNA-D2002");
    }
}
