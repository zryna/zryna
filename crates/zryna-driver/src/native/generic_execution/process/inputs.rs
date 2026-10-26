//! Exact retained compiler inputs, including mode and ctime beyond the legacy tool identity.
use super::super::{stable, staging_error as rejected};
use crate::native::{NativeStage, ToolFileIdentity, tool_identity_from_metadata};
use sha2::{Digest as _, Sha256};
use std::{fs, os::unix::fs::MetadataExt as _, path::Path};
use zryna_diagnostics::Diagnostic;
const LIMIT: usize = zryna_backend_native::MAX_NATIVE_OBJECT_BYTES;
#[derive(Clone, Copy)]
pub(in crate::native::generic_execution) struct Expected<'a> {
    pub program: &'a [u8],
    pub runtime: &'a [u8],
    pub harness: &'a [u8],
}
#[derive(Eq, PartialEq)]
struct State {
    identity: ToolFileIdentity,
    mode: u32,
    ctime: i64,
    ctime_nsec: i64,
}
impl State {
    fn read(path: &Path) -> Result<Self, Diagnostic> {
        let metadata = fs::symlink_metadata(path).map_err(|_| rejected())?;
        let identity = tool_identity_from_metadata(&metadata).map_err(|()| rejected())?;
        Ok(Self {
            identity,
            mode: metadata.mode(),
            ctime: metadata.ctime(),
            ctime_nsec: metadata.ctime_nsec(),
        })
    }
}
struct Pin {
    name: &'static str,
    state: State,
    sha256: [u8; 32],
}
impl Pin {
    fn capture(
        stage: &NativeStage,
        name: &'static str,
        expected: &[u8],
    ) -> Result<Self, Diagnostic> {
        stage.revalidate()?;
        if expected.is_empty() || expected.len() > LIMIT {
            return Err(rejected());
        }
        let path = stage.capability_file_path(name)?;
        let before = State::read(&path)?;
        let (identity, bytes) = stable::read(&path, LIMIT)?;
        let after = State::read(&path)?;
        if before != after || identity != before.identity || bytes != expected {
            return Err(rejected());
        }
        Ok(Self { name, state: after, sha256: Sha256::digest(&bytes).into() })
    }
    fn validate(&self, stage: &NativeStage) -> Result<(), Diagnostic> {
        let path = stage.capability_file_path(self.name)?;
        let before = State::read(&path)?;
        // Fail on ctime/mode/inode drift before any whole-file reservation/read.
        if before != self.state {
            return Err(rejected());
        }
        let (identity, bytes) = stable::read(&path, LIMIT)?;
        let after = State::read(&path)?;
        if before != after
            || identity != self.state.identity
            || <[u8; 32]>::from(Sha256::digest(&bytes)) != self.sha256
        {
            return Err(rejected());
        }
        Ok(())
    }
}
pub(super) struct Inputs {
    written: [Pin; 3],
    compiled: Option<Pin>,
}
impl Inputs {
    pub fn capture(stage: &NativeStage, expected: Expected<'_>) -> Result<Self, Diagnostic> {
        let result = Self {
            written: [
                Pin::capture(stage, "program.o", expected.program)?,
                Pin::capture(stage, "runtime.c", expected.runtime)?,
                Pin::capture(stage, "invocation.c", expected.harness)?,
            ],
            compiled: None,
        };
        result.validate(stage)?;
        Ok(result)
    }
    pub fn validate(&self, stage: &NativeStage) -> Result<(), Diagnostic> {
        stage.revalidate()?;
        for input in &self.written {
            input.validate(stage)?;
        }
        if let Some(input) = &self.compiled {
            input.validate(stage)?;
        }
        stage.revalidate()
    }
    pub fn compiled(
        &mut self,
        stage: &NativeStage,
        identity: &ToolFileIdentity,
        bytes: &[u8],
    ) -> Result<(), Diagnostic> {
        if self.compiled.is_some() {
            return Err(rejected());
        }
        let pin = Pin::capture(stage, "runtime.o", bytes)?;
        if &pin.state.identity != identity {
            return Err(rejected());
        }
        self.compiled = Some(pin);
        self.validate(stage)
    }
    pub fn require_uncompiled(&self) -> Result<(), Diagnostic> {
        if self.compiled.is_some() { Err(rejected()) } else { Ok(()) }
    }
    pub fn require_compiled(&self) -> Result<(), Diagnostic> {
        if self.compiled.is_none() { Err(rejected()) } else { Ok(()) }
    }
}
