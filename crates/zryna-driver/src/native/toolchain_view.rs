//! Read-only views of the existing captured GNU toolchain capability.
use super::{LinuxX8664LinkToolchain, Path};

impl LinuxX8664LinkToolchain {
    /// Returns the canonical compiler-driver path retained by this capability.
    #[must_use]
    pub fn driver(&self) -> &Path {
        &self.driver
    }

    /// Returns the validated GCC version text.
    #[must_use]
    pub fn gcc_version(&self) -> &str {
        &self.gcc_version
    }

    /// Returns the validated GNU linker version text.
    #[must_use]
    pub fn linker_version(&self) -> &str {
        &self.linker_version
    }
}
