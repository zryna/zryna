//! Input approval is not compilation; every fixture has genuine resolver and FFI issuers.
use super::support;
use crate::native::recipe::{approval, plan};
mod cache;
mod identity;
mod limits;
mod output;
mod public_api;
mod rejection;
