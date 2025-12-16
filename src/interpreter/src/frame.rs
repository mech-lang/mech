use crate::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FrameState {
  Running,
  Suspended,
  Completed,
}

#[derive(Clone)]
pub struct Frame {
  plan: Plan,
  ip: usize,               // next instruction
  locals: SymbolTableRef,  // for subroutine variables
  out: Option<Value>,      // optional coroutine return
  state: FrameState,       // Running, Suspended, Completed
}

#[derive(Clone)]
pub struct Stack {
  frames: Vec<Frame>,
}