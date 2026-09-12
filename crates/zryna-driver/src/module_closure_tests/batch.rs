use super::*;
use crate::source_session::{ModuleSourceRoot, ModuleSourceSession};
use crate::workspace_source::{StableSource, WorkspaceSourceSession};
use std::sync::Mutex;
use zryna_diagnostics::Diagnostic;
use zryna_frontend::{
    VerifiedFrontendProviderV3, VerifiedFrontendProviderV4, WorkerError, syntax_v4,
};
use zryna_source::SourceFileInput;

fn map(path: &str, text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput { path: path.to_owned(), text: text.to_owned() }])
        .expect("fixture source map")
}

struct RecordingRoot {
    root: WorkspaceSourceRoot,
    events: Mutex<Vec<String>>,
    forge_request: bool,
}

struct RecordingSession<'a> {
    inner: WorkspaceSourceSession<'a>,
    events: &'a Mutex<Vec<String>>,
    forge_request: bool,
}

impl ModuleSourceRoot for RecordingRoot {
    type Session<'a> = RecordingSession<'a>;

    fn begin(&self) -> Result<Self::Session<'_>, Diagnostic> {
        Ok(RecordingSession {
            inner: self.root.begin()?,
            events: &self.events,
            forge_request: self.forge_request,
        })
    }
}

impl ModuleSourceSession for RecordingSession<'_> {
    fn read_source(&mut self, path: &NormalizedSourcePath) -> Result<StableSource, Diagnostic> {
        let mut source = self.inner.read_source(path)?;
        if self.forge_request {
            source.text.push_str("substitution");
        }
        Ok(source)
    }

    fn validate_provider_batch(&mut self, sources: &SourceMap) -> Result<(), Diagnostic> {
        self.inner.validate_provider_batch(sources)?;
        self.events.lock().expect("events").push(format!("batch:{}", sources.len()));
        Ok(())
    }

    fn revalidate_all(&mut self) -> Result<(), Diagnostic> {
        self.events.lock().expect("events").push("full".to_owned());
        self.inner.revalidate_all()
    }
}

struct RecordingProvider<'a> {
    fixture: FixtureProvider,
    events: &'a Mutex<Vec<String>>,
}

impl VerifiedFrontendProviderV3 for RecordingProvider<'_> {
    fn analyze_verified_v3(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v3::ProjectSyntaxSnapshot, WorkerError> {
        self.events.lock().expect("events").push(format!("provider:{}", sources.len()));
        self.fixture.analyze_verified_v3(sources)
    }
}

impl VerifiedFrontendProviderV4 for RecordingProvider<'_> {
    fn analyze_verified_v4(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v4::ProjectSyntaxSnapshot, WorkerError> {
        self.analyze_verified_v3(sources)?;
        let mut raw = serde_json::to_value(raw_snapshot(sources, false)).expect("raw fixture");
        raw["schema_version"] = serde_json::json!(syntax_v4::PROTOCOL_VERSION);
        for file in raw["files"].as_array_mut().expect("fixture files") {
            file["type_syntax"] = serde_json::json!([]);
            file["data_declarations"] = serde_json::json!([]);
        }
        Ok(syntax_v4::verify_snapshot(serde_json::from_value(raw).expect("v4 fixture"), sources)
            .expect("exact v4 fixture"))
    }
}

fn discover(
    root: &RecordingRoot,
    provider: &RecordingProvider<'_>,
    ownership: bool,
) -> Result<(), ModuleClosureError> {
    if ownership {
        crate::ownership_closure::discover_with_clock(root, entry(), provider, Instant::now)
            .map(|_| ())
    } else {
        discover_module_closure_with_clock(root, entry(), provider, Instant::now).map(|_| ())
    }
}

#[test]
fn both_profiles_validate_only_new_batches_and_fully_check_both_sealing_boundaries() {
    for ownership in [false, true] {
        let workspace = TemporaryWorkspace::new("batch-phases");
        workspace.write("main.zry", "import { value as local } from \"./dep.zry\";\n");
        workspace.write("dep.zry", "");
        let root = RecordingRoot {
            root: workspace.root(),
            events: Mutex::new(Vec::new()),
            forge_request: false,
        };
        let provider = RecordingProvider { fixture: FixtureProvider::new(), events: &root.events };
        discover(&root, &provider, ownership).expect("sealed closure");
        assert_eq!(
            *root.events.lock().expect("events"),
            ["batch:1", "provider:1", "batch:1", "provider:1", "full", "provider:2", "full"]
        );
    }
}

#[test]
fn both_profiles_reject_substituted_request_bytes_before_provider_dispatch() {
    for ownership in [false, true] {
        let workspace = TemporaryWorkspace::new("batch-substitution");
        workspace.write("main.zry", "original");
        let root = RecordingRoot {
            root: workspace.root(),
            events: Mutex::new(Vec::new()),
            forge_request: true,
        };
        let provider = RecordingProvider { fixture: FixtureProvider::new(), events: &root.events };
        assert!(discover(&root, &provider, ownership).is_err());
        assert!(provider.fixture.calls().is_empty());
        assert!(root.events.lock().expect("events").is_empty());
    }
}

#[test]
fn workspace_batch_rejects_forged_bytes_and_uncaptured_paths() {
    let workspace = TemporaryWorkspace::new("batch-authority");
    workspace.write("main.zry", "original");
    workspace.write("other.zry", "present but not captured");
    let root = workspace.root();
    let mut session = root.begin().expect("session");
    session.read_source(&entry()).expect("captured entry");
    assert!(session.validate_provider_batch(&map("main.zry", "forged!!")).is_err());
    assert!(
        session.validate_provider_batch(&map("other.zry", "present but not captured")).is_err()
    );
    session.validate_provider_batch(&map("main.zry", "original")).expect("exact request");
}

#[test]
fn workspace_batch_checks_current_content_and_selected_parent_identity() {
    let workspace = TemporaryWorkspace::new("batch-current");
    workspace.write("nested/main.zry", "original");
    let root = workspace.root();
    let mut session = root.begin().expect("session");
    let path = NormalizedSourcePath::new("nested/main.zry").expect("path");
    session.read_source(&path).expect("capture");
    if fs::write(workspace.path.join("nested/main.zry"), "modified").is_ok() {
        assert!(session.validate_provider_batch(&map("nested/main.zry", "original")).is_err());
    } else {
        session
            .validate_provider_batch(&map("nested/main.zry", "original"))
            .expect("mutation denied");
    }

    let moved = TemporaryWorkspace::new("batch-parent");
    moved.write("nested/main.zry", "original");
    let root = moved.root();
    let mut session = root.begin().expect("session");
    session.read_source(&path).expect("capture");
    if fs::rename(moved.path.join("nested"), moved.path.join("held")).is_ok() {
        moved.write("nested/main.zry", "original");
        assert!(session.validate_provider_batch(&map("nested/main.zry", "original")).is_err());
    } else {
        session
            .validate_provider_batch(&map("nested/main.zry", "original"))
            .expect("replacement denied");
    }
}

#[cfg(unix)]
#[test]
fn workspace_batches_reject_replaced_root_and_linked_current_source() {
    let workspace = TemporaryWorkspace::new("batch-root");
    workspace.write("selected/main.zry", "original");
    let root = WorkspaceSourceRoot::capture(&workspace.path.join("selected")).expect("root");
    let mut session = root.begin().expect("session");
    session.read_source(&entry()).expect("capture");
    fs::rename(workspace.path.join("selected"), workspace.path.join("held")).expect("move root");
    workspace.write("selected/main.zry", "original");
    assert!(session.validate_provider_batch(&map("main.zry", "original")).is_err());

    let workspace = TemporaryWorkspace::new("batch-link");
    workspace.write("main.zry", "original");
    workspace.write("target.zry", "original");
    let root = workspace.root();
    let mut session = root.begin().expect("session");
    session.read_source(&entry()).expect("capture");
    fs::remove_file(workspace.path.join("main.zry")).expect("remove source");
    std::os::unix::fs::symlink("target.zry", workspace.path.join("main.zry")).expect("link source");
    assert!(session.validate_provider_batch(&map("main.zry", "original")).is_err());
}

#[cfg(unix)]
#[test]
fn earlier_mutation_cannot_replace_later_input_or_reach_final_analysis() {
    for ownership in [false, true] {
        let workspace = TemporaryWorkspace::new("batch-earlier");
        workspace.write("main.zry", "import { value as local } from \"./dep.zry\";\n");
        workspace.write("dep.zry", "original\n");
        let earlier = workspace.path.join("main.zry");
        let root = RecordingRoot {
            root: workspace.root(),
            events: Mutex::new(Vec::new()),
            forge_request: false,
        };
        let provider = RecordingProvider {
            fixture: FixtureProvider::with_hook(2, move || {
                fs::write(&earlier, "modified\n").expect("earlier mutation");
            }),
            events: &root.events,
        };
        assert!(discover(&root, &provider, ownership).is_err());
        assert_eq!(provider.fixture.calls().len(), 2);
        assert_eq!(provider.fixture.calls()[1].paths, ["dep.zry"]);
        assert_eq!(
            *root.events.lock().expect("events"),
            ["batch:1", "provider:1", "batch:1", "provider:1", "full"]
        );
    }
}

#[cfg(unix)]
#[test]
fn both_profiles_reject_mutation_during_final_analysis() {
    for ownership in [false, true] {
        let workspace = TemporaryWorkspace::new("batch-final");
        workspace.write("main.zry", "original\n");
        let source = workspace.path.join("main.zry");
        let root = RecordingRoot {
            root: workspace.root(),
            events: Mutex::new(Vec::new()),
            forge_request: false,
        };
        let provider = RecordingProvider {
            fixture: FixtureProvider::with_hook(2, move || {
                fs::write(&source, "modified\n").expect("final mutation");
            }),
            events: &root.events,
        };
        assert!(discover(&root, &provider, ownership).is_err());
        assert_eq!(
            *root.events.lock().expect("events"),
            ["batch:1", "provider:1", "full", "provider:1", "full"]
        );
    }
}
