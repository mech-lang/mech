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
  pub static ref MATH_ADD__UPDATE: u64 = hash_str("math/add-update");
  pub static ref MATH_SUBTRACT__UPDATE: u64 = hash_str("math/subtract-update");
  pub static ref MATH_MULTIPLY__UPDATE: u64 = hash_str("math/multiply-update");
  pub static ref MATH_DIVIDE__UPDATE: u64 = hash_str("math/divide-update");
  pub static ref MATH_EXPONENT__UPDATE: u64 = hash_str("math/exponent-update");
}

math_update_SIxSIx!(MathAddSIxSIx,+=);
math_update_SIxSIx!(MathSubtractSIxSIx,-=);
math_update_SIxSIx!(MathMultiplySIxSIx,*=);
math_update_SIxSIx!(MathDivideSIxSIx,/=);

math_update_compiler!(MathAddUpdate,MathAddSIxSIx);
math_update_compiler!(MathSubtractUpdate,MathSubtractSIxSIx);
math_update_compiler!(MathMultiplyUpdate,MathMultiplySIxSIx);
math_update_compiler!(MathDivideUpdate,MathDivideSIxSIx);

// Update Scalar{ix} : Scalar{ix}
#[macro_export]
macro_rules! math_update_SIxSIx {
  ($func_name:ident, $op1:tt) => (
    #[derive(Debug)]
    pub struct $func_name<T,U> {
      pub arg: ColumnV<T>, pub ix: usize, pub out: ColumnV<U>, pub oix: usize
    }
    impl<T,U> MechFunction for $func_name<T,U>
    where T: Clone + Debug + Into<U> + MechNumArithmetic<T>,
          U: Clone + Debug + Into<T> + MechNumArithmetic<U>
    {
      fn solve(&self) {
        (self.out.borrow_mut())[self.oix] $op1 T::into((self.arg.borrow())[self.ix].clone());
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }
  )
}

#[macro_export]
macro_rules! math_update_compiler {
  ($func_name:ident, $op1:tt) => ( //,$op2:tt,$op3:tt,$op4:tt) => (

    pub struct $func_name {}

    impl MechFunctionCompiler for $func_name {
      fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
        let (_,src_id,src_indices) = &arguments[0];
        let (dest_id,dest_row,dest_col) = out;
        let arg_shapes = block.get_arg_dims(&arguments)?;
        let out_arg = (0,*dest_id,vec![(dest_row.clone(),dest_col.clone())]);
        let dest_shape = block.get_arg_dim(&out_arg)?;
        let src_table = block.get_table(src_id)?;
        let dest_table = block.get_table(dest_id)?;
        let mut arguments = arguments.clone();
        // The destination is pushed into the arguments here in order to use the
        // get_argument_column() machinery later.
        arguments.push(out_arg);
        // Now decide on the correct tfm based on the shape
        match (&arg_shapes[0],&dest_shape) {
          (TableShape::Scalar,TableShape::Scalar) => {
            let arg_cols = block.get_arg_columns(&arguments)?;
            match (&arg_cols[0],&arg_cols[1]) {
              ((_,Column::U8(src),ColumnIndex::Index(in_ix)),(_,Column::U8(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::U16(src),ColumnIndex::Index(in_ix)),(_,Column::U16(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::U32(src),ColumnIndex::Index(in_ix)),(_,Column::U32(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::U64(src),ColumnIndex::Index(in_ix)),(_,Column::U64(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::U128(src),ColumnIndex::Index(in_ix)),(_,Column::U128(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::I8(src),ColumnIndex::Index(in_ix)),(_,Column::I8(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::I16(src),ColumnIndex::Index(in_ix)),(_,Column::I16(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::I32(src),ColumnIndex::Index(in_ix)),(_,Column::I32(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::I64(src),ColumnIndex::Index(in_ix)),(_,Column::I64(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::I128(src),ColumnIndex::Index(in_ix)),(_,Column::I128(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::F32(src),ColumnIndex::Index(in_ix)),(_,Column::F32(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::F64(src),ColumnIndex::Index(in_ix)),(_,Column::F64(out),ColumnIndex::Index(out_ix))) => {block.plan.push($op1{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              /*((_,Column::F32(arg),ColumnIndex::Index(ix)),(_,Column::F32(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::F32(src),ColumnIndex::Index(in_ix)),(_,Column::U8(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::F32(src),ColumnIndex::Index(in_ix)),(_,Column::F32(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::U64(arg),ColumnIndex::Index(ix)),(_,Column::U64(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::U64(src),ColumnIndex::Index(in_ix)),(_,Column::U64(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::U128(arg),ColumnIndex::Index(ix)),(_,Column::U128(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::U128(src),ColumnIndex::Index(in_ix)),(_,Column::U128(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::Bool(src),ColumnIndex::Index(in_ix)),(_,Column::Bool(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::Bool(arg),ColumnIndex::Index(ix)),(_,Column::Bool(out),ColumnIndex::Bool(oix))) => block.plan.push(SetSIxVB{arg: arg.clone(), ix: *ix, out: out.clone(), oix: oix.clone()}),
              ((_,Column::Ref(src),ColumnIndex::Index(in_ix)),(_,Column::Ref(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}
              ((_,Column::String(src),ColumnIndex::Index(in_ix)),(_,Column::String(out),ColumnIndex::Index(out_ix))) => {block.plan.push(SetSIxSIx{arg: src.clone(), ix: *in_ix, out: out.clone(), oix: *out_ix});}*/
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6115, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
          /*
          (TableShape::Scalar, TableShape::Column(rows)) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let mut out_column = block.get_out_column(out, *rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_), Column::F32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_), Column::U8(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U16(lhs),_), (_,Column::U16(rhs),_), Column::U16(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U32(lhs),_), (_,Column::U32(rhs),_), Column::U32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U64(lhs),_), (_,Column::U64(rhs),_), Column::U64(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::U128(lhs),_), (_,Column::U128(rhs),_), Column::U128(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              /*((_,Column::I8(lhs),_), (_,Column::I8(rhs),_), Column::I8(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I16(lhs),_), (_,Column::I16(rhs),_), Column::I16(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I32(lhs),_), (_,Column::I32(rhs),_), Column::I32(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I64(lhs),_), (_,Column::I64(rhs),_), Column::I64(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              ((_,Column::I128(lhs),_), (_,Column::I128(rhs),_), Column::I128(out)) => { block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) }
              */
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6005, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }   
          (TableShape::Column(rows), TableShape::Scalar) => {
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            match (&argument_columns[0], &argument_columns[1]) {
              ((_,Column::U8(lhs),_), (_,Column::U8(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::U8)?;
                if let Column::U8(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::F32(lhs),_), (_,Column::F32(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::F32)?;
                if let Column::F32(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::Length(lhs),_), (_,Column::Length(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::Length)?;
                if let Column::Length(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::Speed(lhs),_), (_,Column::Speed(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::Speed)?;
                if let Column::Speed(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::Time(lhs),_), (_,Column::Time(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::Time)?;
                if let Column::Time(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              ((_,Column::Speed(lhs),_), (_,Column::Time(rhs),_)) => { 
                let mut out_column = block.get_out_column(out, *rows, ValueKind::Length)?;
                if let Column::Length(out) = out_column {
                  block.plan.push($op3{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                }
              }
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6006, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }                   
          (TableShape::Column(lhs_rows), TableShape::Column(rhs_rows)) => {
            if lhs_rows != rhs_rows {
              return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6007, kind: MechErrorKind::DimensionMismatch(((*lhs_rows,0),(*rhs_rows,0)))});
            }
            let mut argument_columns = block.get_arg_columns(arguments)?;
            let (_,col,_) = &argument_columns[0];
            let out_column = block.get_out_column(out, *lhs_rows, col.kind())?;
            match (&argument_columns[0], &argument_columns[1], &out_column) {
              ((_,Column::U8(lhs),_),(_,Column::U8(rhs),_),Column::U8(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U16(lhs),_),(_,Column::U16(rhs),_),Column::U16(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U32(lhs),_),(_,Column::U32(rhs),_),Column::U32(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U64(lhs),_),(_,Column::U64(rhs),_),Column::U64(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::U128(lhs),_),(_,Column::U128(rhs),_),Column::U128(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::F32(lhs),_),(_,Column::F32(rhs),_),Column::F32(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::F64(lhs),_),(_,Column::F64(rhs),_),Column::F64(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::Length(lhs),_),(_,Column::Length(rhs),_),Column::Length(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::Speed(lhs),_),(_,Column::Speed(rhs),_),Column::Speed(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              ((_,Column::Time(lhs),_),(_,Column::Time(rhs),_),Column::Time(out)) => { block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() }) },
              x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6008, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            }
          }
          (TableShape::Row(cols), TableShape::Scalar) => {
            let lhs_columns = block.get_whole_table_arg_cols(&arguments[0])?;
            let rhs_column = block.get_arg_column(&arguments[1])?;

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(1,*cols);

            for (col_ix,(_,lhs_column,_)) in lhs_columns.iter().enumerate() {
              match (lhs_column,&rhs_column) {
                (Column::U8(lhs), (_,Column::U8(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op3::<U8>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F32(lhs), (_,Column::F32(rhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op3::<F32>{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6009, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          (TableShape::Scalar, TableShape::Row(cols)) => {
            let rhs_columns = block.get_whole_table_arg_cols(&arguments[1])?;
            let lhs_column = block.get_arg_column(&arguments[0])?;

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(1,*cols);

            for (col_ix,(_,rhs_column,_)) in rhs_columns.iter().enumerate() {
              match (rhs_column,&lhs_column) {
                (Column::U8(rhs), (_,Column::U8(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                (Column::F32(rhs), (_,Column::F32(lhs),_)) => { 
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op2{lhs: lhs.clone(), rhs: rhs.clone(), out: out.clone() }) 
                  }
                }
                x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6010, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }            
          (TableShape::Row(lhs_cols), TableShape::Row(rhs_cols)) => {
            let lhs_rows = 1;
            let rhs_rows = 1;

            if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
              return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6011, kind: MechErrorKind::DimensionMismatch(((lhs_rows,*lhs_cols),(rhs_rows,*rhs_cols)))});
            }

            let lhs_columns = block.get_whole_table_arg_cols(&arguments[0])?;
            let rhs_columns = block.get_whole_table_arg_cols(&arguments[1])?;

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(lhs_rows,*lhs_cols);

            for (col_ix,lhs_rhs) in lhs_columns.iter().zip(rhs_columns).enumerate() {
              match (lhs_rhs) {
                 (((_,Column::U8(lhs),_), (_,Column::U8(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::U64(lhs),_), (_,Column::U64(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U64)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U64(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::F32(lhs),_), (_,Column::F32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6012, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          (TableShape::Matrix(lhs_rows,lhs_cols), TableShape::Matrix(rhs_rows,rhs_cols)) => {
           
            if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
              return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6011, kind: MechErrorKind::DimensionMismatch(((*lhs_rows,*lhs_cols),(*rhs_rows,*rhs_cols)))});
            }

            let lhs_columns = block.get_whole_table_arg_cols(&arguments[0])?;
            let rhs_columns = block.get_whole_table_arg_cols(&arguments[1])?;

            let (out_table_id, _, _) = out;
            let out_table = block.get_table(out_table_id)?;
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(*lhs_rows,*lhs_cols);

            for (col_ix,lhs_rhs) in lhs_columns.iter().zip(rhs_columns).enumerate() {
              match (lhs_rhs) {
                 (((_,Column::U8(lhs),_), (_,Column::U8(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::U8)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::U8(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                (((_,Column::F32(lhs),_), (_,Column::F32(rhs),_))) => {
                  out_brrw.set_col_kind(col_ix, ValueKind::F32)?;
                  let out_col = out_brrw.get_column(&TableIndex::Index(col_ix+1))?;
                  if let Column::F32(out) = out_col {
                    block.plan.push($op4{lhs: (lhs.clone(),0,lhs.len()-1), rhs: (rhs.clone(),0,rhs.len()-1), out: out.clone() })
                  }
                }
                x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6012, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          (TableShape::Pending(table_id),_) => { return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6013, kind: MechErrorKind::PendingTable(*table_id)}); }
          (_,TableShape::Pending(table_id)) => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6014, kind: MechErrorKind::PendingTable(*table_id)}); },
          */
          x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 6115, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
        Ok(())
      }
    }
  )
}