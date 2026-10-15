//! Caller-selected source policy; classifications never issue execution authority.
use std::{fmt, str::FromStr};

/// Closed source trust classes. Dependency content cannot select or upgrade this mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceTrustMode {
    /// Sources remain data for the caller-owned trusted compiler.
    PureSource,
    /// Each native invocation needs separate exact approval and enforced isolation.
    TrustedNative,
    /// User native recipes are forbidden; no publication authority is issued.
    UntrustedPlayground,
}

impl SourceTrustMode {
    /// Returns the exact accepted caller-policy spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PureSource => "pure-source",
            Self::TrustedNative => "trusted-native",
            Self::UntrustedPlayground => "untrusted-playground",
        }
    }
}

/// Policy disposition only, never a native execution capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeRecipeDisposition {
    /// No native recipe may start in this mode.
    Forbidden,
    /// The driver must separately establish exact approval and actual host isolation.
    RequiresApprovalAndIsolation,
}

/// An unknown caller-selected trust class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceTrustModeError;
impl fmt::Display for SourceTrustModeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown source trust mode")
    }
}
impl std::error::Error for SourceTrustModeError {}
impl FromStr for SourceTrustMode {
    type Err = SourceTrustModeError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pure-source" => Ok(Self::PureSource),
            "trusted-native" => Ok(Self::TrustedNative),
            "untrusted-playground" => Ok(Self::UntrustedPlayground),
            _ => Err(SourceTrustModeError),
        }
    }
}

/// Classifies a native recipe without approving its inputs, tools, host or execution.
#[must_use]
pub const fn native_recipe_disposition(mode: SourceTrustMode) -> NativeRecipeDisposition {
    match mode {
        SourceTrustMode::PureSource | SourceTrustMode::UntrustedPlayground => {
            NativeRecipeDisposition::Forbidden
        }
        SourceTrustMode::TrustedNative => NativeRecipeDisposition::RequiresApprovalAndIsolation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_caller_modes_classify_without_issuing_execution_authority() {
        for (name, mode, disposition) in [
            ("pure-source", SourceTrustMode::PureSource, NativeRecipeDisposition::Forbidden),
            (
                "trusted-native",
                SourceTrustMode::TrustedNative,
                NativeRecipeDisposition::RequiresApprovalAndIsolation,
            ),
            (
                "untrusted-playground",
                SourceTrustMode::UntrustedPlayground,
                NativeRecipeDisposition::Forbidden,
            ),
        ] {
            assert_eq!(name.parse::<SourceTrustMode>(), Ok(mode));
            assert_eq!(mode.as_str(), name);
            assert_eq!(native_recipe_disposition(mode), disposition);
        }
    }
    #[test]
    fn unknown_or_normalized_mode_spellings_fail_closed() {
        for value in [
            "",
            "native",
            "Trusted-Native",
            " trusted-native",
            "trusted-native\n",
            "trusted_native",
        ] {
            assert_eq!(value.parse::<SourceTrustMode>(), Err(SourceTrustModeError));
        }
    }
}
