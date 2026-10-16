//! In-memory scalar compilation for an independently isolated browser transport.
//!
//! This facade issues no filesystem publication or target execution capability. Its caller
//! must independently authenticate the executable and enforce whole-process-tree isolation.

use std::{fmt, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};
use zryna_diagnostics::Diagnostic;
use zryna_source::{SourceFileInput, SourceMap};

use crate::{SourceToIrError, diagnostic_sessions::ToolingCompiler};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) mod daemon;
mod frame;
mod isolated;
mod runtime;
pub use runtime::restrict_browser_runtime;
#[cfg(test)]
mod tests;

/// Fixed logical source identity; requests cannot choose paths or a source graph.
pub const BROWSER_SOURCE_PATH: &str = "src/main.zry";
/// Inclusive byte budget of one edited source, before compiler admission.
pub const MAX_BROWSER_SOURCE_BYTES: usize = 4096;
const MAX_BINDING_BYTES: usize = 65_536;
const MAX_COMPONENT_BYTES: usize = 1_048_576;
const MAX_REVISION: u64 = (1_u64 << 53) - 1;

/// A captured fixed compiler material closure, separate from language-server query sessions.
pub struct RestrictedBrowserCompiler {
    compiler: ToolingCompiler,
}

/// Failures of transport policy, compiler configuration, or sealed output encoding.
#[derive(Clone, Copy, Debug)]
pub struct RestrictedBrowserError(&'static str);

impl fmt::Display for RestrictedBrowserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for RestrictedBrowserError {}

impl RestrictedBrowserCompiler {
    /// Captures fixed Node/provider bytes below an independently authenticated material root.
    ///
    /// This does not authenticate this facade's executable or establish an OS sandbox.
    ///
    /// # Errors
    /// Rejects unsafe roots, substituted material bytes, or unavailable compiler configuration.
    pub fn capture(material_root: &Path) -> Result<Self, RestrictedBrowserError> {
        let compiler = ToolingCompiler::discover_installed(material_root)
            .map_err(|_| RestrictedBrowserError("PLAYGROUND-COMPILER-MATERIALS"))?;
        Ok(Self { compiler })
    }

    /// Compiles the exact edited bytes through the existing scalar compiler and component audit.
    ///
    /// No source file, artifact, process selected by source, or executable target is published.
    /// Compiler rejections retain their actual source-bound structured diagnostics.
    ///
    /// # Errors
    /// Rejects transport limits or an invariant failure without returning executable bytes.
    pub fn compile(
        &self,
        revision: u64,
        source: &str,
    ) -> Result<BrowserCompilation, RestrictedBrowserError> {
        validate_input(revision, source)?;
        let sources = SourceMap::build(vec![SourceFileInput {
            path: BROWSER_SOURCE_PATH.to_owned(),
            text: source.to_owned(),
        }])
        .map_err(|_| RestrictedBrowserError("PLAYGROUND-SOURCE-INVARIANT"))?;
        let result = self.compiler.compile_scalar_source(&sources);
        let mut response = BrowserCompilation::empty(revision, source);
        match result {
            Ok(success) => {
                let component = zryna_backend_webassembly::emit_scalar_component(
                    success.program(),
                    &zryna_backend_webassembly::pinned_wit_sources(),
                );
                match component {
                    Ok(component) => {
                        let bindings = crate::browser_component::generate(&component)
                            .map_err(|_| RestrictedBrowserError("PLAYGROUND-BINDING-INVARIANT"))?;
                        if component.bytes().len() > MAX_COMPONENT_BYTES
                            || bindings.loader.len() > MAX_BINDING_BYTES
                            || bindings.declarations.len() > MAX_BINDING_BYTES
                        {
                            return Err(RestrictedBrowserError("PLAYGROUND-OUTPUT-LIMIT"));
                        }
                        response.report = report(success.diagnostics(), &sources)?;
                        response.status = "compiled";
                        response.identity = Some(ComponentIdentity {
                            revision: crate::browser_component::REVISION,
                            world: component.world_identity(),
                            wit_sha256: hex(component.wit_source_digest()),
                            component_sha256: hex(component.digest()),
                            core_sha256: hex(component.core_digest()),
                            interface_sha256: hex(component.interface_digest()),
                            core_offset: bindings.core_offset,
                            core_bytes: component.core().bytes().len(),
                        });
                        response.exports = component
                            .exports()
                            .iter()
                            .map(|export| Export {
                                logical: export.logical_name().to_owned(),
                                component: export.component_name().to_owned(),
                                core: export.webassembly_name().to_owned(),
                                arity: export.arity(),
                            })
                            .collect();
                        response.artifacts = vec![
                            Artifact::new("component", component.bytes().to_vec()),
                            Artifact::new("loader", bindings.loader),
                            Artifact::new("declarations", bindings.declarations),
                        ];
                    }
                    Err(error) => response.report = report(&[error], &sources)?,
                }
            }
            Err(error) => {
                response.report = report(error.diagnostics(), &sources)?;
                if let SourceToIrError::Frontend(error) = error
                    && error.diagnostics().is_empty()
                {
                    response.status = "unavailable";
                    response.failure = Some(CompilerFailure::new(error.code(), error.to_string())?);
                }
            }
        }
        Ok(response)
    }
}

/// Source-bound compiler output; executable fields can only be constructed by this facade.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCompilation {
    format: &'static str,
    version: u32,
    revision: u64,
    source_path: &'static str,
    source_sha256: String,
    source_bytes: usize,
    profile: &'static str,
    status: &'static str,
    report: serde_json::Value,
    failure: Option<CompilerFailure>,
    identity: Option<ComponentIdentity>,
    exports: Vec<Export>,
    artifacts: Vec<Artifact>,
}

impl BrowserCompilation {
    fn empty(revision: u64, source: &str) -> Self {
        Self {
            format: "zryna.playground-compilation.v1",
            version: 1,
            revision,
            source_path: BROWSER_SOURCE_PATH,
            source_sha256: digest(source.as_bytes()),
            source_bytes: source.len(),
            profile: "browser-component-v1",
            status: "rejected",
            report: serde_json::json!({"schema_version": 1, "diagnostics": []}),
            failure: None,
            identity: None,
            exports: Vec::new(),
            artifacts: Vec::new(),
        }
    }
}

#[derive(Serialize)]
struct CompilerFailure {
    code: String,
    message: String,
}

impl CompilerFailure {
    fn new(code: &str, message: String) -> Result<Self, RestrictedBrowserError> {
        if code.len() > 128 || message.len() > 4096 {
            return Err(RestrictedBrowserError("PLAYGROUND-DIAGNOSTIC-LIMIT"));
        }
        Ok(Self { code: code.to_owned(), message })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComponentIdentity {
    revision: &'static str,
    world: &'static str,
    wit_sha256: String,
    component_sha256: String,
    core_sha256: String,
    interface_sha256: String,
    core_offset: usize,
    core_bytes: usize,
}

#[derive(Serialize)]
struct Export {
    logical: String,
    component: String,
    core: String,
    arity: usize,
}

#[derive(Serialize)]
struct Artifact {
    role: &'static str,
    bytes: usize,
    sha256: String,
    #[serde(skip)]
    content: Vec<u8>,
}

impl Artifact {
    fn new(role: &'static str, content: Vec<u8>) -> Self {
        Self { role, bytes: content.len(), sha256: digest(&content), content }
    }
}

fn report(
    diagnostics: &[Diagnostic],
    sources: &SourceMap,
) -> Result<serde_json::Value, RestrictedBrowserError> {
    if diagnostics.len() > 256
        || diagnostics.iter().any(|diagnostic| {
            diagnostic.code().len() > 128
                || diagnostic.message().len() > 4096
                || diagnostic.guidance().len() > 4096
                || diagnostic.path().is_some_and(|path| path.len() > 1024)
        })
    {
        return Err(RestrictedBrowserError("PLAYGROUND-DIAGNOSTIC-LIMIT"));
    }
    let report = zryna_diagnostics::render_json(diagnostics, sources)
        .map_err(|_| RestrictedBrowserError("PLAYGROUND-DIAGNOSTIC-INVARIANT"))?;
    if report.len() > 65_536 {
        return Err(RestrictedBrowserError("PLAYGROUND-DIAGNOSTIC-LIMIT"));
    }
    serde_json::from_str(&report)
        .map_err(|_| RestrictedBrowserError("PLAYGROUND-DIAGNOSTIC-INVARIANT"))
}

fn validate_input(revision: u64, source: &str) -> Result<(), RestrictedBrowserError> {
    if revision == 0 || revision > MAX_REVISION || source.len() > MAX_BROWSER_SOURCE_BYTES {
        Err(RestrictedBrowserError("PLAYGROUND-INPUT-LIMIT"))
    } else {
        Ok(())
    }
}

fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes).into())
}

fn hex(bytes: &[u8; 32]) -> String {
    crate::browser_component::hex(bytes)
}
