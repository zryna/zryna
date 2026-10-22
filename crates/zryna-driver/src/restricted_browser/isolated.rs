//! Isolated material capture and owned cleanup before releasing browser output.

use std::{fmt, path::Path};

use zryna_diagnostics::Diagnostic;

use super::{
    BrowserCompilation, RestrictedBrowserCompiler, RestrictedBrowserError, ToolingCompiler,
    restrict_browser_runtime, runtime,
};

#[derive(Debug)]
struct IsolatedBrowserCaptureError {
    reason: &'static str,
    primary: Option<String>,
    cleanup: Option<Box<Diagnostic>>,
}

impl IsolatedBrowserCaptureError {
    fn policy(error: RestrictedBrowserError) -> Self {
        Self { reason: "PLAYGROUND-RUNTIME-LIMIT", primary: Some(error.to_string()), cleanup: None }
    }

    fn materials(error: impl fmt::Display) -> Self {
        Self {
            reason: "PLAYGROUND-COMPILER-MATERIALS",
            primary: Some(error.to_string()),
            cleanup: None,
        }
    }
}

impl fmt::Display for IsolatedBrowserCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason)?;
        if let Some(primary) = &self.primary
            && primary != self.reason
            && primary != "PLAYGROUND-UNSUPPORTED-HOST"
        {
            write!(formatter, ": {primary}")?;
        }
        if let Some(cleanup) = &self.cleanup {
            write!(formatter, "; owned cleanup failed: {cleanup}")?;
        }
        Ok(())
    }
}

impl std::error::Error for IsolatedBrowserCaptureError {}

impl RestrictedBrowserCompiler {
    /// Prepares authenticated materials, then tightens limits before the first Node probe.
    ///
    /// Use only in a dedicated Linux process already isolated by its trusted supervisor.
    /// This irreversibly lowers inherited hard limits; it does not create an OS sandbox.
    /// Pair with `compile_isolated` to complete owned cleanup before delivering a response.
    ///
    /// # Errors
    /// Rejects incorrect inherited limits, changed materials, failed cleanup or runtime discovery.
    pub fn capture_isolated(material_root: &Path) -> Result<Self, impl std::error::Error> {
        runtime::restrict_material_setup().map_err(IsolatedBrowserCaptureError::policy)?;
        let prepared = ToolingCompiler::prepare_installed(material_root)
            .map_err(IsolatedBrowserCaptureError::materials)?;
        if let Err(primary) = prepared.revalidate() {
            let cleanup = prepared.abort().err().map(Box::new);
            return Err(IsolatedBrowserCaptureError {
                cleanup,
                ..IsolatedBrowserCaptureError::materials(primary)
            });
        }
        if let Err(primary) = restrict_browser_runtime() {
            let cleanup = prepared.abort().err().map(Box::new);
            return Err(IsolatedBrowserCaptureError {
                cleanup,
                ..IsolatedBrowserCaptureError::policy(primary)
            });
        }
        let compiler = prepared.finish().map_err(IsolatedBrowserCaptureError::materials)?;
        Ok(Self { compiler })
    }

    /// Compiles one request and consumes its private stage before returning any frame bytes.
    ///
    /// # Errors
    /// Retains compilation and independent owned-cleanup failures without executable output.
    pub fn compile_isolated(
        self,
        revision: u64,
        source: &str,
    ) -> Result<BrowserCompilation, impl std::error::Error> {
        let compiled = self.compile(revision, source);
        let cleanup = self.compiler.abort().err().map(Box::new);
        match (compiled, cleanup) {
            (Ok(response), None) => Ok(response),
            (compiled, cleanup) => Err(IsolatedBrowserCaptureError {
                reason: if compiled.is_err() {
                    "PLAYGROUND-COMPILER-FAILED"
                } else {
                    "PLAYGROUND-CLEANUP-FAILED"
                },
                primary: compiled.err().map(|error| error.to_string()),
                cleanup,
            }),
        }
    }
}
