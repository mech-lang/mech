use alloc::string::{String, ToString};

/// Execution ownership selected by the canonical fence information string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeFenceScope {
    Inert,
    Root,
    Disabled,
    Named(String),
    UnsupportedInfo(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeFenceInfo {
    pub scope: CodeFenceScope,
    pub hidden: bool,
}

impl CodeFenceInfo {
    pub fn from_info_string(info: &str) -> Self {
        let info = info.trim();
        let suffix = ["mech", "mec", "🤖"]
            .into_iter()
            .find_map(|prefix| info.strip_prefix(prefix));
        let Some(suffix) = suffix else {
            return Self {
                scope: CodeFenceScope::Inert,
                hidden: false,
            };
        };
        let suffix = suffix.trim();
        if !suffix.is_empty() && !suffix.starts_with(':') {
            return Self {
                scope: CodeFenceScope::UnsupportedInfo(info.to_string()),
                hidden: false,
            };
        }
        match suffix.strip_prefix(':').map(str::trim) {
            Some("disabled") => Self {
                scope: CodeFenceScope::Disabled,
                hidden: false,
            },
            Some("hidden") => Self {
                scope: CodeFenceScope::Root,
                hidden: true,
            },
            Some("") | None => Self {
                scope: CodeFenceScope::Root,
                hidden: false,
            },
            Some(name) => Self {
                scope: CodeFenceScope::Named(name.to_string()),
                hidden: false,
            },
        }
    }

    pub fn is_mech(&self) -> bool {
        !matches!(self.scope, CodeFenceScope::Inert)
    }
}
