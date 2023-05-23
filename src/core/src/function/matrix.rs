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

#[derive(Debug)]
pub struct MatrixMulMM<T,U,V> {
  pub lhs: Vec<ColumnV<U>>,
  pub rhs: Vec<ColumnV<T>>,
  pub out: Vec<ColumnV<V>>
}

impl<T,U,V> MechFunction for MatrixMulMM<T,U,V> 
where T: Copy + Debug + Clone + MechNumArithmetic<T> + Into<V> + Sync + Send + Zero,
      U: Copy + Debug + Clone + MechNumArithmetic<U> + Into<V> + Sync + Send + Zero,
      V: Copy + Debug + Clone + MechNumArithmetic<V> + Sync + Send + Zero,
{
  fn solve(&self) {    

    for i in 0..self.out.len() {
      let mut out_col = self.out[i].borrow_mut();
      for j in 0..out_col.len() {
        let mut result: V = zero();
        for k in 0..self.lhs.len() {
          let lhs = self.lhs[k].borrow()[j];
          let rhs = &self.rhs[i].borrow()[k];
          result += U::into(lhs) * T::into(*rhs);
        }
        out_col[j] = result;
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct MatrixMul{}
impl MechFunctionCompiler for MatrixMul {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let arg_shapes = block.get_arg_dims(&arguments)?;
    let (lhs_arg_name,lhs_arg_table_id,_) = arguments[0];
    let (rhs_arg_name,rhs_arg_table_id,_) = arguments[1];
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    let lhs_kind = { block.get_table(&lhs_arg_table_id)?.borrow().kind() };
    let rhs_kind = { block.get_table(&rhs_arg_table_id)?.borrow().kind() };
    match (&lhs_kind, &rhs_kind) {
      (_,ValueKind::Compound(_)) |
      (ValueKind::Compound(_),_) => {
        return Err(MechError{msg: "".to_string(), id: 9049, kind: MechErrorKind::GenericError("matrix/multiply doesn't support compound table kinds.".to_string())});
      }
      (k,j) => {
        if (*k != *j) {
          return Err(MechError{msg: "".to_string(), id: 9050, kind: MechErrorKind::GenericError("matrix/multiply doesn't support disparate table kinds.".to_string())});
        }
      }
    }
    match (&arg_shapes[0],&arg_shapes[1]) {
      (TableShape::Row(columns), TableShape::Column(rows)) => {
        if columns != rows {
          return Err(MechError{msg: "".to_string(), id: 9403, kind: MechErrorKind::GenericError("Dimension mismatch".to_string())});
        }
        out_brrw.resize(1,1);    
        out_brrw.set_kind(rhs_kind);
        let arg_col = block.get_arg_column(&arguments[1])?;
        match (arg_col,out_brrw.get_column_unchecked(0)) {
          ((_,Column::F32(rhs),_),Column::F32(out_col)) => {
            let (arg_name,arg_table_id,_) = arguments[0];
            let lhs = { block.get_table(&arg_table_id)?.borrow().collect_columns_f32() };
            block.plan.push(MatrixMulRV{lhs: lhs.clone(), rhs: rhs.clone(), out: out_col.clone()});
          }
          ((_,Column::F64(rhs),_),Column::F64(out_col)) => {
            let (arg_name,arg_table_id,_) = arguments[0];
            let lhs = { block.get_table(&arg_table_id)?.borrow().collect_columns_f64() };
            block.plan.push(MatrixMulRV{lhs: lhs.clone(), rhs: rhs.clone(), out: out_col.clone()});
          }
          x => {return Err(MechError{msg: "".to_string(), id: 9044, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      }
      (TableShape::Matrix(lhs_rows,lhs_columns), TableShape::Column(rows)) => {
        if lhs_columns != rows {
          return Err(MechError{msg: "".to_string(), id: 9403, kind: MechErrorKind::GenericError("Dimension mismatch".to_string())});
        }
        out_brrw.resize(*rows,1);    
        out_brrw.set_kind(rhs_kind);
        match lhs_kind {
          ValueKind::F32 => {
            let lhs = { block.get_table(&lhs_arg_table_id)?.borrow().collect_columns_f32() };
            let rhs = { block.get_table(&rhs_arg_table_id)?.borrow().collect_columns_f32() };
            let out_cols = out_brrw.collect_columns_f32();
            block.plan.push(MatrixMulMM{lhs: lhs.clone(), rhs: rhs.clone(), out: out_cols.clone()});
          }
          ValueKind::F64 => {
            let lhs = { block.get_table(&lhs_arg_table_id)?.borrow().collect_columns_f64() };
            let rhs = { block.get_table(&rhs_arg_table_id)?.borrow().collect_columns_f64() };
            let out_cols = out_brrw.collect_columns_f64();
            block.plan.push(MatrixMulMM{lhs: lhs.clone(), rhs: rhs.clone(), out: out_cols.clone()});
          }
          x => {return Err(MechError{msg: "".to_string(), id: 9044, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      }
      (TableShape::Column(rows),TableShape::Row(columns)) => {
        out_brrw.resize(*rows,*columns);
        out_brrw.set_kind(rhs_kind);
        let arg_col = block.get_arg_column(&arguments[0])?;
        match (arg_col,out_brrw.get_column_unchecked(0)) {
          ((_,Column::F32(lhs),_),Column::F32(out_col)) => {
            let (arg_name,arg_table_id,_) = arguments[1];
            let rhs = { block.get_table(&arg_table_id)?.borrow().collect_columns_f32() };
            let out_cols = out_brrw.collect_columns_f32();
            block.plan.push(MatrixMulVR{lhs: lhs.clone(), rhs: rhs.clone(), out: out_cols.clone()});
          }
          ((_,Column::F64(lhs),_),Column::F64(out_col)) => {
            let (arg_name,arg_table_id,_) = arguments[1];
            let rhs = { block.get_table(&arg_table_id)?.borrow().collect_columns_f64() };
            let out_cols = out_brrw.collect_columns_f64();
            block.plan.push(MatrixMulVR{lhs: lhs.clone(), rhs: rhs.clone(), out: out_cols.clone()});
          }
          x => {return Err(MechError{msg: "".to_string(), id: 9047, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      }
      (TableShape::Row(lhs_columns),TableShape::Matrix(rhs_rows,rhs_columns)) => {
        if lhs_columns != rhs_rows {
          return Err(MechError{msg: "".to_string(), id: 9048, kind: MechErrorKind::GenericError("Dimension mismatch".to_string())});
        }        
        out_brrw.resize(1,*rhs_columns);
        out_brrw.set_kind(rhs_kind);
        match lhs_kind {
          ValueKind::F32 => {
            let lhs = { block.get_table(&lhs_arg_table_id)?.borrow().collect_columns_f32() };
            let rhs = { block.get_table(&rhs_arg_table_id)?.borrow().collect_columns_f32() };
            let out_cols = out_brrw.collect_columns_f32();
            block.plan.push(MatrixMulMM{lhs: lhs.clone(), rhs: rhs.clone(), out: out_cols.clone()});
          }
          ValueKind::F64 => {
            let lhs = { block.get_table(&lhs_arg_table_id)?.borrow().collect_columns_f64() };
            let rhs = { block.get_table(&rhs_arg_table_id)?.borrow().collect_columns_f64() };
            let out_cols = out_brrw.collect_columns_f64();
            block.plan.push(MatrixMulMM{lhs: lhs.clone(), rhs: rhs.clone(), out: out_cols.clone()});
          }
          x => {return Err(MechError{msg: "".to_string(), id: 9048, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        } 
      }
      (TableShape::Matrix(lhs_rows,lhs_columns),TableShape::Matrix(rhs_rows,rhs_columns)) => {
        if lhs_columns != rhs_rows {
          return Err(MechError{msg: "".to_string(), id: 9048, kind: MechErrorKind::GenericError("Dimension mismatch".to_string())});
        }        
        out_brrw.resize(*lhs_rows,*rhs_columns);
        out_brrw.set_kind(rhs_kind);
        match lhs_kind {
          ValueKind::F32 => {
            let lhs = { block.get_table(&lhs_arg_table_id)?.borrow().collect_columns_f32() };
            let rhs = { block.get_table(&rhs_arg_table_id)?.borrow().collect_columns_f32() };
            let out_cols = out_brrw.collect_columns_f32();
            block.plan.push(MatrixMulMM{lhs: lhs.clone(), rhs: rhs.clone(), out: out_cols.clone()});
          }
          ValueKind::F64 => {
            let lhs = { block.get_table(&lhs_arg_table_id)?.borrow().collect_columns_f64() };
            let rhs = { block.get_table(&rhs_arg_table_id)?.borrow().collect_columns_f64() };
            let out_cols = out_brrw.collect_columns_f64();
            block.plan.push(MatrixMulMM{lhs: lhs.clone(), rhs: rhs.clone(), out: out_cols.clone()});
          }
          x => {return Err(MechError{msg: "".to_string(), id: 9049, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }        
      }
      x => {return Err(MechError{msg: "".to_string(), id: 9051, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}

#[derive(Debug)]
pub struct MatrixTransposeR<T,V> {
  pub arg: Vec<ColumnV<T>>,
  pub out: ColumnV<V>
}

impl<T,V> MechFunction for MatrixTransposeR<T,V> 
where T: Copy + Debug + Clone + MechNumArithmetic<T> + Into<V> + Sync + Send + Zero,
      V: Copy + Debug + Clone + MechNumArithmetic<V> + Sync + Send + Zero,
{
  fn solve(&self) {    
    let mut out = self.out.borrow_mut();
    for i in 0..self.arg.len() {
      out[i] = T::into(self.arg[i].borrow()[0]);
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

#[derive(Debug)]
pub struct MatrixTransposeM<T,V> {
  pub arg: Vec<ColumnV<T>>,
  pub out: Vec<ColumnV<V>>,
}

impl<T,V> MechFunction for MatrixTransposeM<T,V> 
where T: Copy + Debug + Clone + MechNumArithmetic<T> + Into<V> + Sync + Send + Zero,
      V: Copy + Debug + Clone + MechNumArithmetic<V> + Sync + Send + Zero,
{
  fn solve(&self) {    
    for i in 0..self.arg.len() {
      let arg_brrw = self.arg[i].borrow();
      for j in 0..arg_brrw.len() {
        let mut out_brrw = self.out[j].borrow_mut();
        out_brrw[i] = T::into(arg_brrw[j]);
      }
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct MatrixTranspose{}
impl MechFunctionCompiler for MatrixTranspose {

  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let arg_shape = block.get_arg_dim(&arguments[0])?;
    let (arg_name,arg_table_id,_) = arguments[0];
    let (out_table_id,_,_) = out;
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    match arg_shape {
      TableShape::Row(columns) => {
        let (arg_name,arg_table_id,arg_indices) = &arguments[0];
        let arg_table = block.get_table(&arg_table_id)?;
        let arg_kind = { arg_table.borrow().kind() };
        match arg_kind {
          ValueKind::Compound(_) => {
            return Err(MechError{msg: "".to_string(), id: 9152, kind: MechErrorKind::GenericError("matrix/transpose doesn't support compound table kinds.".to_string())});
          }
          _ => (),
        }
        out_brrw.resize(columns,1);
        out_brrw.set_kind(arg_kind);
        match out_brrw.get_column_unchecked(0) {
          Column::F32(out_col) => {
            let arg = { block.get_table(&arg_table_id)?.borrow().collect_columns_f32() };
            block.plan.push(MatrixTransposeR{arg: arg.clone(), out: out_col.clone()});
          }
          Column::F64(out_col) => {
            let arg = { block.get_table(&arg_table_id)?.borrow().collect_columns_f64() };
            block.plan.push(MatrixTransposeR{arg: arg.clone(), out: out_col.clone()});
          }
          x => {return Err(MechError{msg: "".to_string(), id: 9153, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      TableShape::Matrix(rows,columns) => {
        let arg_kind = { block.get_table(&arg_table_id)?.borrow().kind() };
        match arg_kind {
          ValueKind::Compound(_) => {
            return Err(MechError{msg: "".to_string(), id: 9154, kind: MechErrorKind::GenericError("matrix/transpose doesn't support compound table kinds.".to_string())});
          }
          _ => (),
        }
        out_brrw.resize(columns,rows);
        out_brrw.set_kind(arg_kind.clone());
        match arg_kind {
          ValueKind::F32 => {
            let arg = { block.get_table(&arg_table_id)?.borrow().collect_columns_f32() };
            let out_cols = { out_brrw.collect_columns_f32() };
            block.plan.push(MatrixTransposeM{arg: arg.clone(), out: out_cols.clone()});
          }
          ValueKind::F64 => {
            let arg = { block.get_table(&arg_table_id)?.borrow().collect_columns_f64() };
            let out_cols = { out_brrw.collect_columns_f64() };
            block.plan.push(MatrixTransposeM{arg: arg.clone(), out: out_cols.clone()});
          }
          x => {return Err(MechError{msg: "".to_string(), id: 9047, kind: MechErrorKind::GenericError(format!("{:?}",x))})},
        }
      }
      x => {return Err(MechError{msg: "".to_string(), id: 9156, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}