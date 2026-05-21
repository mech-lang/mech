pub enum RuntimeEvent {
  RuntimeCreated { runtime: RuntimeId },
  ModuleCompiled { module_version: ModuleVersionId },
  ModuleActivated { module_version: ModuleVersionId },
  CapabilityGranted { capability: CapabilityId },
  CapabilityRevoked { capability: CapabilityId },
  ProgramStarted { task: TaskId },
  ProgramCompleted { task: TaskId },
  ObjectUpdated { object: ObjectId },
  TransactionCommitted { transaction: TransactionId },
  ActorMessageSent { actor: ActorId },
  ActorTurnCompleted { actor: ActorId },
  RuntimeError { message: String },
}