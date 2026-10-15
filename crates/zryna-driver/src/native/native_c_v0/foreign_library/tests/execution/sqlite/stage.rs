//! Test-only ownership guard for unchanged native staging filenames and cleanup.
use super::*;
use crate::native::NativeStage;
use std::ops::Deref;

pub(super) struct Stage(NativeStage);
impl Stage {
    pub(super) fn new(fixture: &Fixture) -> Self {
        Self(fixture.stage())
    }
    pub(super) fn retain(stage: NativeStage) -> Self {
        Self(stage)
    }
}
impl Deref for Stage {
    type Target = NativeStage;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        let warnings = self.0.cleanup();
        if std::thread::panicking() {
            for warning in warnings {
                eprintln!("417-sqlite-cleanup {warning:?}");
            }
        } else {
            assert!(warnings.is_empty(), "owned SQLite test stage cleanup: {warnings:?}");
        }
    }
}
