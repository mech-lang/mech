pub(crate) mod base;
pub(crate) mod combinator;
pub(crate) mod found;
pub(crate) mod grammar;
mod ports;
pub(crate) mod roots;
pub mod terminal_spec;

pub use ports::{
  CanonicalRuleSnapshot, canonical_base_rule_supported, parse_canonical_base_rule_for_test,
  parse_canonical_tag_for_test,
};
pub use terminal_spec::{
  FIXED_TERMINAL_COUNT, FIXED_TERMINALS, FixedTerminalSpec, TerminalSpacing, fixed_terminal_spec,
};
