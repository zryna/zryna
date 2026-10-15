//! Internal exact-world execution/denial/teardown evidence; no public selector or listener.
#![forbid(unsafe_code)]

#[path = "../runtime_boundary_tests/fuel_meter.rs"]
mod runtime_boundary_tests;
#[path = "../server_lifecycle/mod.rs"]
mod server_lifecycle;
#[path = "mod.rs"]
mod server_runtime;
#[path = "../server_transport/mod.rs"]
mod server_transport;

use server_lifecycle::{Input, Limits};
use server_runtime::test_support::{Resource, attach, input, limits};
use zryna_driver::{compile_to_verified_ir, lower_verified_syntax};
