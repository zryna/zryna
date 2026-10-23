//! Private v4 proposal tests. Actual v4 APIs remain unimplemented.
//! Fixed vectors are physical oracles; source seals require fresh branded authorities.
#[path = "generic_struct_v4/authority.rs"]
mod authority;
#[path = "generic_struct_v4/claims.rs"]
mod claims;
#[path = "generic_struct_v4/fixture.rs"]
mod fixture;
#[path = "generic_struct_v4/limits.rs"]
mod limits;
#[path = "generic_struct_v4/source.rs"]
mod source;
#[path = "generic_struct_v4/wire.rs"]
mod wire;
