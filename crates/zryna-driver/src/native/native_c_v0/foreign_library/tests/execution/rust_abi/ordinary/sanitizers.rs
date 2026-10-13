//! Initial private C sanitizer observation; unavailable independent bindings block execution.
use super::*;
mod build;
mod compiler_prefix;
mod machine;
mod observation;
mod observe;
mod policy;
mod prefix_capture;
mod source;
mod targets;
mod tests;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Case {
    Positive,
    HeapOverrun,
    SignedOverflow,
}
impl Case {
    fn name(self) -> &'static str {
        match self {
            Self::Positive => "sanitizer-positive",
            Self::HeapOverrun => "sanitizer-heap-overrun",
            Self::SignedOverflow => "sanitizer-signed-overflow",
        }
    }
}
