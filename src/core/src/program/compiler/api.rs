use crate::*;

#[cfg(feature = "compiler")]
pub trait BytecodeCompilerContext {
  fn register_for_ptr_with_initialization_status(
    &mut self,
    pointer: usize,
  ) -> (Register, bool);

  fn compile_const(
    &mut self,
    bytes: &[u8],
    kind: ValueKind,
  ) -> MResult<u32>;

  fn define_symbol(
    &mut self,
    pointer: usize,
    register: Register,
    name: &str,
    mutable: bool,
  );

  fn require(
    &mut self,
    requirement: FeatureFlag,
  );

  fn emit_const_load(
    &mut self,
    destination: Register,
    constant: u32,
  );

  fn emit_nullop(
    &mut self,
    function: u64,
    destination: Register,
  );

  fn emit_unop(
    &mut self,
    function: u64,
    destination: Register,
    source: Register,
  );

  fn emit_binop(
    &mut self,
    function: u64,
    destination: Register,
    lhs: Register,
    rhs: Register,
  );

  fn emit_ternop(
    &mut self,
    function: u64,
    destination: Register,
    a: Register,
    b: Register,
    c: Register,
  );

  fn emit_quadop(
    &mut self,
    function: u64,
    destination: Register,
    a: Register,
    b: Register,
    c: Register,
    d: Register,
  );

  fn emit_varop(
    &mut self,
    function: u64,
    destination: Register,
    arguments: Vec<Register>,
  );

  fn emit_ret(
    &mut self,
    source: Register,
  );
}
