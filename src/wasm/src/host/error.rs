use std::fmt;

use mech_core::BrowserCapabilityError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserHostError {
    Capability(BrowserCapabilityError),
    BrowserDeniedOrUnavailable { reason: String },
}

impl From<BrowserCapabilityError> for BrowserHostError {
    fn from(error: BrowserCapabilityError) -> Self {
        Self::Capability(error)
    }
}

impl fmt::Display for BrowserHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capability(error) => write!(f, "{error}"),
            Self::BrowserDeniedOrUnavailable { reason } => {
                write!(f, "browser denied or unavailable: {reason}")
            }
        }
    }
}

impl std::error::Error for BrowserHostError {}
