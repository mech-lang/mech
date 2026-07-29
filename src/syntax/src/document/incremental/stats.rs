#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReparseStats {
  pub source_bytes: u64,
  pub parser_steps: u64,
  pub events_emitted: u64,
  pub diagnostics_emitted: u64,
  pub recovery_bytes: u64,
  pub reparse_root_count: u64,
  pub reused_node_count: u64,
  pub new_node_count: u64,
  pub attempted_roots: u64,
  pub document_fallbacks: u64,
}
