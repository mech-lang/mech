use crate::TransactionId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeHealth {
  Healthy,
  Poisoned(RuntimePoisonRecord),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePoisonRecord {
  pub operation: String,
  pub transaction_id: Option<TransactionId>,
  pub original_error: String,
  pub rollback_failures: Vec<String>,
}
