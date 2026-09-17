/// Document parsing needs two enclosing starts plus a three-event error
/// envelope and matching finishes to preserve a completed prefix.
pub const MIN_PREFIX_PRESERVING_EVENTS: u32 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseLimits {
  pub max_nesting: u32,
  pub max_diagnostics: u32,
  pub max_events: u32,
  pub max_recovery_bytes: u32,
  pub fuel: u64,
}

impl Default for ParseLimits {
  fn default() -> Self {
    Self {
      max_nesting: 256,
      max_diagnostics: 128,
      max_events: 1_000_000,
      max_recovery_bytes: 64 * 1024,
      fuel: 4_000_000,
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseConfig {
  pub limits: ParseLimits,
}

impl Default for ParseConfig {
  fn default() -> Self {
    Self {
      limits: ParseLimits::default(),
    }
  }
}
