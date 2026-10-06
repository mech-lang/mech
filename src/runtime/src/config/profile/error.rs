use mech_core::{MechError, MechErrorKind};

macro_rules! config_error_kind {
    ($name:ident, $kind_name:literal, $prefix:literal) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            pub reason: String,
        }

        impl $name {
            pub fn new(reason: impl Into<String>) -> Self {
                Self {
                    reason: reason.into(),
                }
            }

            pub fn error(reason: impl Into<String>) -> MechError {
                MechError::new(Self::new(reason), None).with_compiler_loc()
            }
        }

        impl MechErrorKind for $name {
            fn name(&self) -> &str {
                $kind_name
            }
            fn message(&self) -> String {
                format!("{}: {}", $prefix, self.reason)
            }
        }
    };
}

#[cfg(feature = "source")]
config_error_kind!(
    ConfigProfileViolation,
    "ConfigProfileViolation",
    "Mech config profile violation"
);
config_error_kind!(
    InvalidConfigField,
    "InvalidConfigField",
    "Invalid Mech config field"
);
#[cfg(feature = "source")]
config_error_kind!(
    MissingConfigBinding,
    "MissingConfigBinding",
    "Missing Mech config binding"
);
#[cfg(feature = "source")]
config_error_kind!(
    ConfigEvaluationBudgetExceeded,
    "ConfigEvaluationBudgetExceeded",
    "Mech config evaluation budget exceeded"
);
#[cfg(feature = "source")]
config_error_kind!(
    ConfigRecursionNotAllowed,
    "ConfigRecursionNotAllowed",
    "Mech config recursion is not allowed"
);
#[cfg(feature = "source")]
config_error_kind!(
    ConfigUnknownFunction,
    "ConfigUnknownFunction",
    "Unknown Mech config function"
);
#[cfg(feature = "source")]
config_error_kind!(
    ConfigEffectfulFunctionNotAllowed,
    "ConfigEffectfulFunctionNotAllowed",
    "Effectful functions are not allowed in Mech config"
);

/// A rejected finite configuration retains the exact canonical diagnostic owner.
/// Callers can inspect anchors, fixes, source identity, and the raw text without
/// parsing again or translating locations through another representation.
#[cfg(feature = "source")]
#[derive(Clone, Debug)]
pub struct InvalidConfigSyntax {
    pub source_name: String,
    pub source: crate::resolver::SourceDocument,
}

#[cfg(feature = "source")]
impl MechErrorKind for InvalidConfigSyntax {
    fn name(&self) -> &str {
        "InvalidConfigSyntax"
    }

    fn message(&self) -> String {
        let mut message = format!(
            "{}: configuration requires a complete canonical source document",
            self.source_name,
        );
        let snapshot = self.source.snapshot();
        for diagnostic in snapshot.diagnostics.iter() {
            message.push('\n');
            message.push_str(&mech_syntax::document::render_plain(
                diagnostic,
                &snapshot.source,
                &snapshot.nodes,
            ));
        }
        message
    }
}
