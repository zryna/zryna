//! Closed exact source derivatives, independently selected before any compile.
use super::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    UnknownStatus,
    WriteOnError,
    NonfaultingControl,
}
impl Kind {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::UnknownStatus => "unknown-status",
            Self::WriteOnError => "write-on-error",
            Self::NonfaultingControl => "nonfaulting-control",
        }
    }
    pub(super) const fn status(self) -> i32 {
        match self {
            Self::UnknownStatus => 7,
            Self::WriteOnError => 1,
            Self::NonfaultingControl => 0,
        }
    }
    pub(super) const fn output(self) -> i32 {
        match self {
            Self::UnknownStatus => 0,
            Self::WriteOnError | Self::NonfaultingControl => 6,
        }
    }
    pub(super) fn source(self) -> Result<Vec<u8>, Diagnostic> {
        let original = capture::FOREIGN_RUST_SOURCE;
        material::check_source(original.as_bytes())?;
        let tail = "unsafe { out.write(result) };\n    0\n}";
        if original.matches(tail).count() != 1 {
            return Err(rejected());
        }
        let replacement = match self {
            Self::UnknownStatus => "let _ = result;\n    7\n}",
            Self::WriteOnError => "unsafe { out.write(result) };\n    1\n}",
            Self::NonfaultingControl => tail,
        };
        let changed = original.replace(tail, replacement).into_bytes();
        let (length, expected) = match self {
            Self::UnknownStatus => {
                (1682, "22673e80283b6748acbf22b4c065dffb7a6ea4a12693c56fe48c14160949df18")
            }
            Self::WriteOnError => {
                (1696, "4a10d3cc49f2abfb8115e8e00bdb1464f128022c5549222f21164d9012fb84c9")
            }
            Self::NonfaultingControl => {
                (1696, "26c4f32993bd1aabb118b8a3dfe66902bc623852564f562776218c366758826b")
            }
        };
        if changed.len() != length
            || digest(&changed) != expected
            || changed.len() > zryna_backend_native::MAX_NATIVE_OBJECT_BYTES
            || (self != Self::NonfaultingControl && changed == original.as_bytes())
        {
            return Err(rejected());
        }
        Ok(changed)
    }
}
