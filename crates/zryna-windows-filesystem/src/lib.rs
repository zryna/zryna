//! Narrow Windows directory mutation primitives for Zryna-owned transactions.
//!
//! The API creates a directory and acquires its authoritative capability in one
//! operation. That opaque capability can then perform a no-replace rename or an
//! exact empty-directory removal without selecting the source again by path.
//! No ambient-path operation or arbitrary source handle is exposed.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{OwnedDirectory, create_directory};
