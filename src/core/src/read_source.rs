use crate::{CellId, ConstantId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReadSource {
    Constant(ConstantId),
    Cell(CellId),
}
