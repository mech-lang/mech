//! Driver-only profile selection. The production entry never selects a bypass.
use super::{Inputs, Path, Value, json, process};
#[path = "canonical_hosted_full_json.rs"]
mod unique_json;
pub(super) fn decode(bytes: &[u8]) -> Result<Value, String> {
    unique_json::decode(bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RunSet {
    Production,
    #[cfg(feature = "canonical-native-benchmark-policy")]
    BenchmarkComparison,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TrialPolicy {
    ProductionSecured,
    #[cfg(feature = "canonical-native-benchmark-policy")]
    BenchmarkSecured,
    #[cfg(feature = "canonical-native-benchmark-policy")]
    BenchmarkTrusted,
}
impl TrialPolicy {
    pub(super) fn name(self) -> &'static str {
        match self {
            #[cfg(feature = "canonical-native-benchmark-policy")]
            Self::BenchmarkTrusted => "trusted",
            _ => "secured",
        }
    }
    pub(super) fn route(self) -> &'static str {
        match self {
            Self::ProductionSecured => "canonical-artifact-program-object-secured-cli",
            #[cfg(feature = "canonical-native-benchmark-policy")]
            _ => "canonical-artifact-program-object-benchmark-cli",
        }
    }
    pub(super) fn label(self, base: &str) -> String {
        if self == Self::ProductionSecured {
            base.into()
        } else {
            format!("{base}-{}", self.name())
        }
    }
    pub(super) fn annotate(self, mut value: Value) -> Value {
        if self != Self::ProductionSecured {
            value["policy"] = json!(self.name());
            value["route"] = json!(self.route());
        }
        value
    }
}
impl RunSet {
    pub(super) fn modes(self) -> &'static [TrialPolicy] {
        match self {
            Self::Production => &[TrialPolicy::ProductionSecured],
            #[cfg(feature = "canonical-native-benchmark-policy")]
            Self::BenchmarkComparison => {
                &[TrialPolicy::BenchmarkSecured, TrialPolicy::BenchmarkTrusted]
            }
        }
    }
    pub(super) fn trials(self) -> usize {
        2 * self.modes().len()
    }
    pub(super) fn sample_slot(self, round: usize, offset: usize) -> usize {
        assert!(offset < self.modes().len());
        if round % 2 == 0 {
            offset
        } else {
            self.modes().len() - 1 - offset
        }
    }
    pub(super) fn result_schema(self) -> &'static str {
        match self {
            Self::Production => "mech.canonical-hosted-full-input-result.v1",
            #[cfg(feature = "canonical-native-benchmark-policy")]
            Self::BenchmarkComparison => "mech.canonical-native-policy-comparison-result.v1",
        }
    }
    pub(super) fn stage_schema(self) -> &'static str {
        match self {
            Self::Production => "mech.canonical-hosted-staged-input-result.v1",
            #[cfg(feature = "canonical-native-benchmark-policy")]
            Self::BenchmarkComparison => "mech.canonical-native-policy-comparison-stages.v1",
        }
    }
    pub(super) fn qualifies(self, rows: &[Value]) -> bool {
        if rows.len() != self.trials() {
            return false;
        }
        let complete = rows.iter().enumerate().all(|(index, row)| {
            let mode = self.modes()[index % self.modes().len()];
            row["variant"].as_u64() == Some((index / self.modes().len()) as u64)
                && row["ok"] == true
                && (self == Self::Production
                    || (row["policy"] == mode.name() && row["route"] == mode.route()))
        });
        if !complete || self == Self::Production {
            return complete;
        }
        for field in [
            "native_work",
            "native_comparison_work",
            "native_visits",
            "native_transport_bytes",
            "host_result_carrier_bytes",
        ] {
            let Some(expected) = rows[0]["observed"]["report"][field].as_u64() else {
                return false;
            };
            if rows
                .iter()
                .any(|row| row["observed"]["report"][field].as_u64() != Some(expected))
            {
                return false;
            }
        }
        true
    }
    pub(super) fn summaries(self, variant: u64, samples: &[Value]) -> Vec<Value> {
        self.modes()
            .iter()
            .map(|mode| {
                let mut ns = samples
                    .iter()
                    .filter(|row| {
                        row["variant"] == variant
                            && (self == Self::Production || row["policy"] == mode.name())
                    })
                    .map(|row| row["elapsed_ns"].as_u64().unwrap())
                    .collect::<Vec<_>>();
                ns.sort_unstable();
                mode.annotate(json!({"variant":variant,"samples":ns.len(),
                    "min_ns":ns.first(),"median_upper_ns":ns.get(ns.len()/2),"max_ns":ns.last()}))
            })
            .collect()
    }
    pub(super) fn launches(
        self,
        base: &Inputs,
        policy: &Value,
        directory: &Path,
    ) -> Result<Vec<(TrialPolicy, Inputs)>, String> {
        if policy.get("benchmark_policy").is_some() {
            return Err("scale policy must not preselect a benchmark mode".into());
        }
        self.modes()
            .iter()
            .map(|&mode| {
                let launch = if mode == TrialPolicy::ProductionSecured {
                    base.launch.clone()
                } else {
                    let mut derived = policy.clone();
                    derived["benchmark_policy"] = json!(mode.name());
                    process::freeze_bytes(
                        &directory.join(format!("launch-{}.json", mode.name())),
                        &serde_json::to_vec_pretty(&derived).map_err(|e| e.to_string())?,
                        65536,
                    )?
                };
                Ok((
                    mode,
                    Inputs {
                        input: base.input.clone(),
                        oracle: base.oracle.clone(),
                        launch,
                    },
                ))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "canonical_hosted_full_policy_tests.rs"]
mod tests;
