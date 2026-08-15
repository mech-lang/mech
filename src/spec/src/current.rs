use std::collections::BTreeMap;

use mech_core::MechError;
use mech_runtime::{
    ResidentDurabilityPolicy, ResidentRouteFailure, ResidentRouteFailureClass, RuntimeBuilder,
    RuntimeProgramRoute,
};

use crate::reference::ReferenceEvaluation;
use crate::{Result, SpecError};

#[derive(Clone, Debug)]
pub(crate) struct MechEvaluation {
    pub passed: bool,
}

pub(crate) fn evaluate(
    observations: &str,
    constraints: &[ReferenceEvaluation],
) -> Result<BTreeMap<String, MechEvaluation>> {
    constraints
        .iter()
        .map(|constraint| {
            let source = format!(
                "{observations}\n{} := {}\n",
                constraint.name, constraint.expression,
            );
            let mut runtime = runtime()?;
            let passed =
                match runtime.load_source_program(&source, ResidentDurabilityPolicy::Volatile) {
                    Ok(outcome) if outcome.route == RuntimeProgramRoute::ResidentPure => true,
                    Ok(outcome) => {
                        return Err(SpecError::new(format!(
                            "constraint `{}` used unexpected executor route {:?}",
                            constraint.name, outcome.route,
                        )));
                    }
                    Err(error) if is_integrity_failure(&error) => false,
                    Err(error) => {
                        return Err(SpecError::new(format!(
                            "evaluate constraint `{}` with resident Mech failed: {error:?}",
                            constraint.name,
                        )));
                    }
                };
            Ok((constraint.name.clone(), MechEvaluation { passed }))
        })
        .collect()
}

fn runtime() -> Result<mech_runtime::MechRuntime> {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build()
        .map_err(|error| SpecError::new(format!("build resident contract evaluator: {error:?}")))
}

fn is_integrity_failure(error: &MechError) -> bool {
    error
        .kind_as::<ResidentRouteFailure>()
        .is_some_and(|failure| {
            failure.class == ResidentRouteFailureClass::ActivationFailure
                && failure.reason.contains("Integrity")
        })
}
