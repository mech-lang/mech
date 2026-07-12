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
config_error_kind!(
    MissingConfigBinding,
    "MissingConfigBinding",
    "Missing Mech config binding"
);
config_error_kind!(
    ConfigEvaluationBudgetExceeded,
    "ConfigEvaluationBudgetExceeded",
    "Mech config evaluation budget exceeded"
);
config_error_kind!(
    ConfigRecursionNotAllowed,
    "ConfigRecursionNotAllowed",
    "Mech config recursion is not allowed"
);
config_error_kind!(
    ConfigUnknownFunction,
    "ConfigUnknownFunction",
    "Unknown Mech config function"
);
config_error_kind!(
    ConfigEffectfulFunctionNotAllowed,
    "ConfigEffectfulFunctionNotAllowed",
    "Effectful functions are not allowed in Mech config"
);
