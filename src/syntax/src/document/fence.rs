use alloc::string::{String, ToString};

/// Execution ownership selected by the canonical fence information string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeFenceScope {
    Inert,
    Root,
    Disabled,
    Named(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeFenceInfo {
    pub scope: CodeFenceScope,
    pub hidden: bool,
}

impl CodeFenceInfo {
    pub(crate) const MECH_PREFIXES: [&'static str; 3] = ["mech", "mec", "🤖"];

    pub fn from_info_string(info: &str) -> Self {
        let info = info.trim();
        if !Self::MECH_PREFIXES
            .into_iter()
            .any(|prefix| info.starts_with(prefix))
        {
            return Self {
                scope: CodeFenceScope::Inert,
                hidden: false,
            };
        }
        // The document selection contract permits a namespace without a colon
        // and removes repeated language prefixes in this fixed order.
        let suffix = info
            .trim_start_matches("mech")
            .trim_start_matches("mec")
            .trim_start_matches("🤖");
        let suffix = suffix.strip_prefix(':').unwrap_or(suffix).trim();
        match suffix {
            "disabled" => Self {
                scope: CodeFenceScope::Disabled,
                hidden: false,
            },
            "hidden" => Self {
                scope: CodeFenceScope::Root,
                hidden: true,
            },
            "" => Self {
                scope: CodeFenceScope::Root,
                hidden: false,
            },
            name => Self {
                scope: CodeFenceScope::Named(name.to_string()),
                hidden: false,
            },
        }
    }

    pub fn is_mech(&self) -> bool {
        !matches!(self.scope, CodeFenceScope::Inert)
    }
}

/// Presentation settings owned by a canonical code fence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeFencePresentation {
    pub show_output: bool,
    pub styles: alloc::vec::Vec<(String, String)>,
}

impl Default for CodeFencePresentation {
    fn default() -> Self {
        Self {
            show_output: true,
            styles: alloc::vec::Vec::new(),
        }
    }
}
