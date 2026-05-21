pub struct ModuleRecord {
  pub id: ModuleId,
  pub name: String,
}

pub struct ModuleVersionRecord {
  pub id: ModuleVersionId,
  pub module: ModuleId,
  pub version: u64,
  pub source: String,
  pub bytecode: Option<Vec<u8>>,
  pub dependencies: Vec<ModuleVersionId>,
  pub required_capabilities: Vec<CapabilityRequirement>,
}

pub struct ObjectRecord {
  pub id: ObjectId,
  pub kind: String,
  pub version: u64,
  pub value: StoredValue,
}

pub struct TransactionRecord {
  pub id: TransactionId,
  pub subject: SubjectId,
  pub read_set: Vec<ObjectId>,
  pub write_set: Vec<ObjectId>,
  pub events: Vec<RuntimeEvent>,
}

pub struct RuntimeTransaction {
  pub id: TransactionId,
  pub subject: SubjectId,
  pub read_set: Vec<ObjectId>,
  pub write_set: Vec<ObjectId>,
  pub events: Vec<RuntimeEvent>,
}

pub trait MechStore {
  fn put_module_version(&mut self,record: ModuleVersionRecord) -> MResult<ModuleVersionId>;

  fn get_module_version(
    &self,
    id: ModuleVersionId,
  ) -> MResult<Option<ModuleVersionRecord>>;

  fn put_object(
    &mut self,
    record: ObjectRecord,
  ) -> MResult<ObjectId>;

  fn get_object(
    &self,
    id: ObjectId,
  ) -> MResult<Option<ObjectRecord>>;

  fn append_event(
    &mut self,
    event: RuntimeEvent,
  ) -> MResult<EventId>;

  fn commit_transaction(
    &mut self,
    transaction: TransactionRecord,
  ) -> MResult<TransactionId>;
}

