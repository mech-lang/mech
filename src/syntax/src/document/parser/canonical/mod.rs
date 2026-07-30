pub(crate) mod base;
pub(crate) mod combinator;
pub(crate) mod found;
pub(crate) mod grammar;
pub(crate) mod kinds;
pub(crate) mod literals;
pub(crate) mod mechdown;
pub(crate) mod operators;
pub(crate) mod paths;
mod ports;
pub(crate) mod roots;
pub(crate) mod statements;
mod test_support;
pub mod terminal_spec;

pub use mechdown::{CanonicalMechdownRuleSnapshot, parse_canonical_mechdown_rule_for_test};
pub use ports::{
    CanonicalRuleSnapshot, canonical_base_rule_supported, parse_canonical_base_rule_for_test,
    parse_canonical_tag_for_test,
};
pub use test_support::{
    CanonicalSourceRuleSnapshot, parse_canonical_phase_2c_rule_for_test,
    parse_canonical_phase_2d_rule_for_test,
};
pub use terminal_spec::{
    FIXED_TERMINAL_COUNT, FIXED_TERMINALS, FixedTerminalSpec, TerminalSpacing, fixed_terminal_spec,
};
