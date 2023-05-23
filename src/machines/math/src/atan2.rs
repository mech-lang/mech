use libm::atan2f;

use std::cell::RefCell;
use std::rc::Rc;
use mech_core::*;
use mech_utilities::*;

lazy_static! {
  static ref Y: u64 = hash_str("y");
  static ref X: u64 = hash_str("x");
}

#[derive(Debug)]
pub struct MathAtan2VV {
  pub y_col: (ColumnV<F32>, usize, usize),
  pub x_col: (ColumnV<F32>, usize, usize),
  pub out: ColumnV<F32>,
}

impl MechFunction for MathAtan2VV {
  fn solve(&self) {
    let (y_col,siy,eiy) = &self.y_col;
    let (x_col,six,eix) = &self.x_col;
    self.out.borrow_mut()
            .iter_mut()
            .zip(y_col.borrow()[*siy..=*eiy].iter())
            .zip(x_col.borrow()[*six..=*eix].iter())
            .for_each(|((out,y), x)| *out = F32::new(atan2f(y.unwrap(),x.unwrap()))); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct MathAtan2 {}

impl MechFunctionCompiler for MathAtan2 {
  fn compile(
      &self,
      block: &mut Block,
      arguments: &Vec<Argument>,
      out: &(TableId, TableIndex, TableIndex)
  ) -> std::result::Result<(),MechError>
  {
    if arguments.len() > 2 {
      return Err(MechError{msg: "".to_string(), id: 1347, kind: MechErrorKind::TooManyInputArguments(arguments.len(),1)});
    }
    let arg_dims = block.get_arg_dims(&arguments)?;
    let (arg_name1,arg_table_id1,_) = arguments[0];
    let (arg_name2,arg_table_id2,_) = arguments[1];
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    if (arg_name1 == *Y && arg_name2 == *X) {
      match (arg_dims[0],arg_dims[1]) {
        (TableShape::Scalar,TableShape::Scalar) => {
          let arg1 = block.get_arg_columns(arguments)?[0].clone();
          let arg2 = block.get_arg_columns(arguments)?[0].clone();
          out_brrw.resize(1,1);
          out_brrw.set_kind(ValueKind::F32);
          if let Column::F32(out_col) = out_brrw.get_column_unchecked(0) {
            match (arg1,arg2) {
              ((_,Column::F32(y_col),ColumnIndex::Index(_)),(_,Column::F32(x_col),ColumnIndex::Index(_))) |
              ((_,Column::F32(y_col),ColumnIndex::All),(_,Column::F32(x_col),ColumnIndex::All)) => block.plan.push(MathAtan2VV{y_col: (y_col.clone(),0,0), x_col: (x_col.clone(),0,0), out: out_col.clone()}),
              ((_,col,_),_) => { return Err(MechError{msg: "".to_string(), id: 1348, kind: MechErrorKind::UnhandledFunctionArgumentKind(col.kind())}); }
            }
          }
        }
        x => {return Err(MechError{msg: "".to_string(), id: 1350, kind: MechErrorKind::UnhandledTableShape(arg_dims[0])});},
      }
    } else {
      return Err(MechError{msg: "".to_string(), id: 1351, kind: MechErrorKind::UnknownFunctionArgument(arg_name1)});
    }
    Ok(())
  }
}


export_mech_function!(math_atan2, math_atan2_reg);

extern "C" fn math_atan2_reg(registrar: &mut dyn MechFunctionRegistrar) {
  registrar.register_mech_function(hash_str("math/atan2"),Box::new(MathAtan2{}));
}