#[macro_use]
use crate::stdlib::*;

// x.a = 1 --------------------------------------------------------------------

// Record Set -----------------------------------------------------------------

#[derive(Debug)]
pub struct RecordSet<T> {
  pub sink: Ref<T>,
  pub source: Ref<T>,
}
impl<T> MechFunction for RecordSet<T> 
  where
  T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<T>: ToValue
{
  fn solve(&self) {
    let source_ptr = self.source.as_ptr();
    let sink_ptr = self.sink.as_ptr();
    unsafe {
      *sink_ptr = *source_ptr.clone();
    }
  }
  fn out(&self) -> Value { self.sink.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}