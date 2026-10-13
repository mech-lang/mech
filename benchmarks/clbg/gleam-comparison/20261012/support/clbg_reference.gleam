// Every node carries a fresh ordinary reference so leaves cannot be pooled.
pub type Reference

@external(erlang, "erlang", "make_ref")
pub fn new() -> Reference
