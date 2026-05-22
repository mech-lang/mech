use crate::*;
use mech_program::*;

pub struct MechRuntime {
  id: RuntimeId,
  config: RuntimeConfig,
  program: MechProgram,
  capabilities: Box<dyn CapabilityKernel>,
  store: Box<dyn MechStore>,
  //scheduler: Scheduler,
}
/*
impl MechRuntime {
  pub fn new(config: RuntimeConfig) -> MResult<Self>;

  pub fn run_string(&mut self, source: &str) -> MResult<Value>;

  pub fn compile_module(&mut self, source: ModuleSource) -> MResult<ModuleVersionId>;

  pub fn run_module(&mut self, module: ModuleVersionId) -> MResult<Value>;

  pub fn grant_capability(&mut self, subject: SubjectId, capability: Capability) -> MResult<CapabilityId>;

  pub fn events(&self) -> Vec<RuntimeEvent>;

  pub fn shutdown(&mut self) -> MResult<()>;
}*/