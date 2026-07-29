#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReparseStats {
  pub source_bytes: u64,
  /// Alias for `total_parser_steps`.
  pub parser_steps: u64,
  /// Alias for `total_events_emitted`.
  pub events_emitted: u64,
  pub fragment_parser_steps: u64,
  pub fragment_events_emitted: u64,
  pub validation_parser_steps: u64,
  pub validation_events_emitted: u64,
  pub rejected_parser_steps: u64,
  pub rejected_events_emitted: u64,
  pub fallback_parser_steps: u64,
  pub fallback_events_emitted: u64,
  pub total_parser_steps: u64,
  pub total_events_emitted: u64,
  pub diagnostics_emitted: u64,
  pub diagnostics_truncated: bool,
  pub recovery_bytes: u64,
  pub reparse_root_count: u64,
  pub reused_node_count: u64,
  pub new_node_count: u64,
  pub attempted_roots: u64,
  pub document_fallbacks: u64,
}
