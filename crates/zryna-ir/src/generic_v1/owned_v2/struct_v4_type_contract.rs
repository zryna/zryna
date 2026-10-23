//! PROPOSED doctests. Register only with `#[cfg(doctest)] mod struct_v4_type_contract;`.
//! Imports/type names have a positive control, so missing APIs cannot make the negative tests pass silently.
//! ```
//! use zryna_ir::generic_v1::{Failure,owned_v2};
//! let _: fn(&[u8])->Result<owned_v2::wire::v4::DecodedOwnedV4,Failure> = owned_v2::wire::v4::decode;
//! let _: fn(&[u8])->Result<owned_v2::wire::DecodedProgram,Failure> = owned_v2::wire::decode;
//! ```
//! A fresh v4 wrapper is never an old decoded capability.
//! ```compile_fail
//! use zryna_ir::generic_v1::owned_v2::wire;
//! fn launder(fresh:wire::v4::DecodedOwnedV4) { let _:wire::DecodedProgram = fresh; }
//! ```
//! Immutable raw claims have no conversion to old admission.
//! ```compile_fail
//! use zryna_ir::generic_v1::owned_v2::wire;
//! fn launder(fresh:wire::v4::DecodedOwnedV4) { let _:wire::DecodedProgram = fresh.claims().clone().into(); }
//! ```
//! Fresh decoded authority has no public raw constructor.
//! ```compile_fail
//! use zryna_ir::generic_v1::owned_v2::{raw,wire};
//! fn forge(raw:raw::Program) { let _ = wire::v4::DecodedOwnedV4(raw); }
//! ```
//! An old wrapper is not a v4 wrapper either.
//! ```compile_fail
//! use zryna_ir::generic_v1::owned_v2::wire;
//! fn promote(old:wire::DecodedProgram) { let _:wire::v4::DecodedOwnedV4 = old.into(); }
//! ```
//! Even an otherwise complete call cannot pass v4 to the old issuer.
//! ```compile_fail
//! use zryna_ir::generic_v1::owned_v2;
//! fn call<'a>(fresh:owned_v2::wire::v4::DecodedOwnedV4,
//!     syntax:&zryna_syntax::v5::VerifiedProjectSyntaxV5,sources:&zryna_source::SourceMap,
//!     entry:zryna_source::FileId,linear:&'a zryna_layout::generic_v1::VerifiedLayouts,
//!     linux:&'a zryna_layout::generic_v1::VerifiedLayouts,
//!     runtime:&'a zryna_ownership_runtime_abi::generic_v1::VerifiedOwnershipRuntimeAbi<'a>) {
//!     let _ = owned_v2::verify(fresh,syntax,sources,entry,linear,linux,runtime);
//! }
//! ```
