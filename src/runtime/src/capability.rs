use std::collections::HashMap;

pub struct Capability {
  pub id: CapabilityId,
  pub subject: SubjectId,
  pub resource: ResourceId,
  pub operations: Vec<Operation>,
  pub constraints: Vec<CapabilityConstraint>,
  pub delegable: bool,
  pub revocable: bool,
}

pub enum Operation {
  Read,
  Write,
  Execute,
  Import,
  Spawn,
  Send,
  Receive,
  Query,
  Admin,
}

pub enum CapabilityConstraint {
  ResourcePrefix(String),
  MaxRows(u64),
  MaxBytes(u64),
  MaxDurationMs(u64),
  LocalOnly,
}

pub struct CapabilityKernel {
  grants: HashMap<CapabilityId, Capability>,
}

impl CapabilityKernel {
  pub fn grant(&mut self, cap: Capability) -> MResult<CapabilityId>;

  pub fn revoke(&mut self, id: CapabilityId) -> MResult<()>;

  pub fn check(
    &self,
    subject: SubjectId,
    resource: ResourceId,
    operation: Operation,
  ) -> MResult<()>;
}