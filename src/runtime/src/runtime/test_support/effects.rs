use std::sync::{Arc, Mutex};

use mech_core::MResult;

use crate::{
    RuntimeAfterCommitEffect, RuntimeCompensatableEffect, RuntimeEffectMetadata,
    RuntimeEffectSource, RuntimeTransactionalEffect,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObservedEffectPhase {
    Prepare,
    Apply,
    Commit,
    Abort,
    Compensate,
    Deliver,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObservedEffect {
    pub(crate) label: String,
    pub(crate) phase: ObservedEffectPhase,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct EffectLifecycleLog {
    observations: Arc<Mutex<Vec<ObservedEffect>>>,
}

impl EffectLifecycleLog {
    pub(crate) fn observations(&self) -> Vec<ObservedEffect> {
        self.observations.lock().unwrap().clone()
    }

    fn record(&self, label: &str, phase: ObservedEffectPhase) {
        self.observations.lock().unwrap().push(ObservedEffect {
            label: label.to_string(),
            phase,
        });
    }
}

fn metadata(label: &str) -> RuntimeEffectMetadata {
    RuntimeEffectMetadata::new(
        RuntimeEffectSource::Custom {
            name: label.to_string(),
        },
        "test-probe",
    )
}

#[derive(Debug)]
pub(crate) struct TransactionalEffectProbe {
    label: String,
    log: EffectLifecycleLog,
}

impl TransactionalEffectProbe {
    pub(crate) fn new(label: impl Into<String>, log: EffectLifecycleLog) -> Self {
        Self {
            label: label.into(),
            log,
        }
    }
}

impl RuntimeTransactionalEffect for TransactionalEffectProbe {
    fn metadata(&self) -> RuntimeEffectMetadata {
        metadata(&self.label)
    }

    fn prepare(&mut self) -> MResult<()> {
        self.log.record(&self.label, ObservedEffectPhase::Prepare);
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        self.log.record(&self.label, ObservedEffectPhase::Commit);
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        self.log.record(&self.label, ObservedEffectPhase::Abort);
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct CompensatableEffectProbe {
    label: String,
    log: EffectLifecycleLog,
}

impl CompensatableEffectProbe {
    pub(crate) fn new(label: impl Into<String>, log: EffectLifecycleLog) -> Self {
        Self {
            label: label.into(),
            log,
        }
    }
}

impl RuntimeCompensatableEffect for CompensatableEffectProbe {
    fn metadata(&self) -> RuntimeEffectMetadata {
        metadata(&self.label)
    }

    fn apply(&mut self) -> MResult<()> {
        self.log.record(&self.label, ObservedEffectPhase::Apply);
        Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
        self.log
            .record(&self.label, ObservedEffectPhase::Compensate);
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        self.log.record(&self.label, ObservedEffectPhase::Abort);
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct AfterCommitEffectProbe {
    label: String,
    log: EffectLifecycleLog,
}

impl AfterCommitEffectProbe {
    pub(crate) fn new(label: impl Into<String>, log: EffectLifecycleLog) -> Self {
        Self {
            label: label.into(),
            log,
        }
    }
}

impl RuntimeAfterCommitEffect for AfterCommitEffectProbe {
    fn metadata(&self) -> RuntimeEffectMetadata {
        metadata(&self.label)
    }

    fn deliver(&mut self) -> MResult<()> {
        self.log.record(&self.label, ObservedEffectPhase::Deliver);
        Ok(())
    }
}

#[test]
fn effect_probes_record_typed_lifecycle_phases() {
    let log = EffectLifecycleLog::default();
    let mut transactional = TransactionalEffectProbe::new("transactional", log.clone());
    let mut compensatable = CompensatableEffectProbe::new("compensatable", log.clone());
    let mut after_commit = AfterCommitEffectProbe::new("after-commit", log.clone());

    transactional.prepare().unwrap();
    transactional.commit().unwrap();
    transactional.abort().unwrap();
    compensatable.apply().unwrap();
    compensatable.compensate().unwrap();
    compensatable.abort().unwrap();
    after_commit.deliver().unwrap();

    assert_eq!(
        log.observations(),
        vec![
            ObservedEffect {
                label: "transactional".to_string(),
                phase: ObservedEffectPhase::Prepare,
            },
            ObservedEffect {
                label: "transactional".to_string(),
                phase: ObservedEffectPhase::Commit,
            },
            ObservedEffect {
                label: "transactional".to_string(),
                phase: ObservedEffectPhase::Abort,
            },
            ObservedEffect {
                label: "compensatable".to_string(),
                phase: ObservedEffectPhase::Apply,
            },
            ObservedEffect {
                label: "compensatable".to_string(),
                phase: ObservedEffectPhase::Compensate,
            },
            ObservedEffect {
                label: "compensatable".to_string(),
                phase: ObservedEffectPhase::Abort,
            },
            ObservedEffect {
                label: "after-commit".to_string(),
                phase: ObservedEffectPhase::Deliver,
            },
        ],
    );
}
