use crate::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::*;
use num_traits::*;
use std::ops::*;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::thread;

lazy_static! {
  pub static ref MATRIX_MULTIPLY: u64 = hash_str("matrix/multiply");
  pub static ref MATRIX_TRANSPOSE: u64 = hash_str("matrix/transpose");
}

#[derive(Debug)]
pub struct MatrixMulRV<T,U,V> {
  pub lhs: Vec<ColumnV<T>>,
  pub rhs: ColumnV<U>,
  pub out: ColumnV<V>
}

impl<T,U,V> MechFunction for MatrixMulRV<T,U,V> 
where T: Copy + Debug + Clone + MechNumArithmetic<T> + Into<V> + Sync + Send + Zero,
      U: Copy + Debug + Clone + MechNumArithmetic<U> + Into<V> + Sync + Send + Zero,
      V: Copy + Debug + Clone + MechNumArithmetic<V> + Sync + Send + Zero,
{
  fn solve(&self) {    
    let result = self.lhs.iter()
                         .zip(self.rhs.borrow().iter())
                         .fold(zero(),|sum: V, (lhs,rhs)| sum + T::into(lhs.borrow()[0]) * U::into(*rhs));
    self.out.borrow_mut()[0] = result
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct MatrixMulVR<T,U,V> {
  pub lhs: ColumnV<U>,
  pub rhs: Vec<ColumnV<T>>,
  pub out: Vec<ColumnV<V>>
}

impl<T,U,V> MechFunction for MatrixMulVR<T,U,V> 
where T: Copy + Debug + Clone + MechNumArithmetic<T> + Into<V> + Sync + Send + Zero,
      U: Copy + Debug + Clone + MechNumArithmetic<U> + Into<V> + Sync + Send + Zero,
      V: Copy + Debug + Clone + MechNumArithmetic<V> + Sync + Send + Zero,
{
  fn solve(&self) {    
    let lhs = self.lhs.borrow();
    for j in 0..self.rhs.len() {
      let rhs = self.rhs[j].borrow();
      let mut out_brrw = self.out[j].borrow_mut();
      for i in 0..lhs.len() {
        let result: V = U::into(lhs[i]) * T::into(rhs[0]);
        out_brrw[i] = result;
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}


pub struct MatrixMul{}
impl MechFunctionCompiler for MatrixMul {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let arg_shapes = block.get_arg_dims(&arguments)?;
    match (&arg_shapes[0],&arg_shapes[1]) {
      (TableShape::Scalar, TableShape::Scalar) => {
      }
      (TableShape::Row(columns), TableShape::Column(rows)) => {
        if columns != rows {
          return Err(MechError{id: 9403, kind: MechErrorKind::GenericError("Dimension mismatch".to_string())});
        }
        let (out_table_id, _, _) = out;
        let out_table = block.get_table(out_table_id)?;
        let mut out_brrw = out_table.borrow_mut();
        out_brrw.resize(1,1);    
        let (arg_name,arg_table_id,_) = arguments[1];
        let arg_table = block.get_table(&arg_table_id)?;
        let rhs_kind = {
          let arg_table_brrw = arg_table.borrow();
          arg_table_brrw.kind()
        };
        let (arg_name,arg_table_id,_) = arguments[0];
        let arg_table = block.get_table(&arg_table_id)?;
        let lhs_kind = {
          let arg_table_brrw = arg_table.borrow();
          arg_table_brrw.kind()
        };
        match (lhs_kind, rhs_kind) {
          (_,ValueKind::Compound(_)) |
          (ValueKind::Compound(_),_) => {
            return Err(MechError{id: 9042, kind: MechErrorKind::GenericError("matrix/multiply doesn't support compound table kinds.".to_string())});
          }
          (k,j) => {
            if (k == j) {
              out_brrw.resize(arg_table.borrow().rows,1);
              out_brrw.set_kind(k);
            } else {
              return Err(MechError{id: 9043, kind: MechErrorKind::GenericError("matrix/multiply doesn't support disparate table kinds.".to_string())});
            }
          }
        }
        let arg_col = block.get_arg_column(&arguments[1])?;
        match (arg_col,out_brrw.get_column_unchecked(0)) {
          ((_,Column::F32(rhs),_),Column::F32(out_col)) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<F32>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::F32(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            block.plan.push(MatrixMulRV{lhs: cols.clone(), rhs: rhs.clone(), out: out_col.clone()});
          }
          x => {return Err(MechError{id: 9044, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      }
      (TableShape::Column(rows),TableShape::Row(columns)) => {
        let (out_table_id, _, _) = out;
        let out_table = block.get_table(out_table_id)?;
        let mut out_brrw = out_table.borrow_mut();
        let (arg_name,arg_table_id,_) = arguments[0];
        let arg_table = block.get_table(&arg_table_id)?;
        let lhs_kind = {
          let arg_table_brrw = arg_table.borrow();
          arg_table_brrw.kind()
        }; 
        let (arg_name,arg_table_id,_) = arguments[1];
        let arg_table = block.get_table(&arg_table_id)?;
        let rhs_kind = {
          let arg_table_brrw = arg_table.borrow();
          arg_table_brrw.kind()
        };
        match (lhs_kind, rhs_kind) {
          (_,ValueKind::Compound(_)) |
          (ValueKind::Compound(_),_) => {
            return Err(MechError{id: 9045, kind: MechErrorKind::GenericError("matrix/multiply doesn't support compound table kinds.".to_string())});
          }
          (k,j) => {
            if (k == j) {
              out_brrw.resize(2,2);
              out_brrw.set_kind(k);
            } else {
              return Err(MechError{id: 9046, kind: MechErrorKind::GenericError("matrix/multiply doesn't support disparate table kinds.".to_string())});
            }
          }
        }
        let arg_col = block.get_arg_column(&arguments[0])?;
        match (arg_col,out_brrw.get_column_unchecked(0)) {
          ((_,Column::F32(lhs),_),Column::F32(out_col)) => {
            let (cols,rows) = {
              let mut cols: Vec<ColumnV<F32>> = vec![];
              let arg_table_brrw = arg_table.borrow();
              for col_ix in 0..arg_table_brrw.cols {
                if let Column::F32(col) = arg_table_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              (cols,arg_table_brrw.rows)
            };
            let out_cols = {
              let mut cols: Vec<ColumnV<F32>> = vec![];
              for col_ix in 0..out_brrw.cols {
                if let Column::F32(col) = out_brrw.get_column_unchecked(col_ix) {
                  cols.push(col);
                }
              }
              cols
            };
            block.plan.push(MatrixMulVR{lhs: lhs.clone(), rhs: cols.clone(), out: out_cols.clone()});
          }
          x => {return Err(MechError{id: 9047, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      }
      x => {return Err(MechError{id: 9048, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}

pub struct MatrixTranspose{}
impl MechFunctionCompiler for MatrixTranspose {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    Ok(())
  }
}