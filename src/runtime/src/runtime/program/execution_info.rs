//! Route inspection without enabling a resident evaluator.
use crate::runtime::MechRuntime;
use mech_core::{LayoutGeneration, PlanGeneration, ProgramRevision};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RuntimeProgramRoute {
    #[default]
    None,
    ResidentPure,
    ResidentExternal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProgramExecutionInfo {
    pub route: RuntimeProgramRoute,
    pub program_revision: Option<ProgramRevision>,
    pub plan_generation: Option<PlanGeneration>,
    pub layout_generation: Option<LayoutGeneration>,
    pub requirement_count: usize,
    pub observation_count: usize,
    pub effect_count: usize,
    pub resident_accepted_turns: u64,
    pub resident_rejected_turns: u64,
    pub coalesced_host_packets: u64,
    pub ignored_host_packets: u64,
}

impl Default for RuntimeProgramExecutionInfo {
    fn default() -> Self {
        Self {
            route: RuntimeProgramRoute::None,
            program_revision: None,
            plan_generation: None,
            layout_generation: None,
            requirement_count: 0,
            observation_count: 0,
            effect_count: 0,
            resident_accepted_turns: 0,
            resident_rejected_turns: 0,
            coalesced_host_packets: 0,
            ignored_host_packets: 0,
        }
    }
}

impl MechRuntime {
    pub fn program_route(&self) -> RuntimeProgramRoute {
        #[cfg(feature = "resident-routing")]
        {
            self.program_execution_info.route
        }
        #[cfg(not(feature = "resident-routing"))]
        {
            RuntimeProgramRoute::None
        }
    }

    pub fn program_execution_info(&self) -> RuntimeProgramExecutionInfo {
        #[cfg(feature = "resident-routing")]
        {
            self.program_execution_info.clone()
        }
        #[cfg(not(feature = "resident-routing"))]
        {
            RuntimeProgramExecutionInfo::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unactivated_runtime_reports_no_program_with_or_without_resident_routing() {
        let mut runtime = MechRuntime::builder().build().unwrap();
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
        let info = runtime.program_execution_info();
        assert_eq!(info, RuntimeProgramExecutionInfo::default());
        assert_eq!(runtime.root_plan_len(), 0);
        assert!(runtime.published_outputs().is_empty());
        runtime.shutdown().unwrap();
        assert_eq!(runtime.program_execution_info(), info);
    }
}
