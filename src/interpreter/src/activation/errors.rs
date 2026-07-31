use crate::{MechErrorKind};

macro_rules! activation_error {
    ($n:ident,$m:expr) => {
        #[derive(Debug, Clone)]
        pub(crate) struct $n;
        impl MechErrorKind for $n {
            fn name(&self) -> &str {
                stringify!($n)
            }
            fn message(&self) -> String {
                $m.into()
            }
        }
    };
}
activation_error!(
    ActivationPatternCaptureKindUnsupported,
    "The capture kind cannot be inferred from the activation trigger."
);
activation_error!(
    ActivationPatternArmsNonExhaustive,
    "Patterned activations require a final unguarded irrefutable arm."
);
activation_error!(
    ActivationPatternWildcardMustBeLast,
    "An unguarded wildcard activation arm must be last."
);
activation_error!(
    ActivationPatternGuardMustBePure,
    "Patterned activation guards must elaborate to a static pure expression graph."
);
activation_error!(
    ActivationPatternGuardDependencyInvariant,
    "The activation guard graph could not be attached to its match pulse."
);
activation_error!(
    ActivationPatternBodyDependencyInvariant,
    "The activation arm body could not sample its committed captures."
);
activation_error!(
    ActivationPatternRegisterWriteUnsupported,
    "Patterned activation register writes must target a whole local register."
);
activation_error!(
    ActivationScopeTriggerWriteUnsupported,
    "An activation scope cannot assign to its own trigger."
);
activation_error!(
    ActivationPatternContextEffectUnsupported,
    "Patterned activation context effects are not supported."
);
activation_error!(
    ActivationPatternTriggerInvariant,
    "Activation trigger root cells disagree with the resolved trigger."
);
activation_error!(
    ActivationPatternTransactionBoolStateUnsupported,
    "Patterned activation transaction state requires boolean values."
);

#[derive(Debug, Clone)]
pub(crate) struct ActivationPatternDefinitionUnsupported;
impl MechErrorKind for ActivationPatternDefinitionUnsupported {
    fn name(&self) -> &str {
        "ActivationPatternDefinitionUnsupported"
    }
    fn message(&self) -> String {
        "This definition or declaration is not supported inside a patterned activation arm."
            .to_string()
    }
}
