use mech_core::*;
use mech_utilities::*;
use libm::sinf;

lazy_static! {
  static ref ANGLE: u64 = hash_str("angle");
  static ref TABLE: u64 = hash_str("table");
}

export_mech_function!(math_sin, math_sin_reg);

extern "C" fn math_sin_reg(registrar: &mut dyn MechFunctionRegistrar) {
  registrar.register_mech_function(hash_str("math/sin"),Box::new(MathSin{}));
}

#[derive(Debug)]
pub struct MathSinRadVV {
  pub col: (ColumnV<F32>,usize,usize), pub out: ColumnV<F32>
}

impl MechFunction for MathSinRadVV {
  fn solve(&self) {
    let (col,six,eix) = &self.col;
    self.out.borrow_mut()
            .iter_mut()
            .zip(col.borrow()[*six..=*eix].iter())
            .for_each(|(out, rhs)| *out = F32::new(sinf(rhs.unwrap()))); 
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct MathSin{}
impl MechFunctionCompiler for MathSin {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    if arguments.len() > 1 {
      return Err(MechError{id: 1347, kind: MechErrorKind::TooManyInputArguments(arguments.len(),1)});
    }
    let arg_dims = block.get_arg_dims(&arguments)?;
    let (arg_name,arg_table_id,_) = arguments[0];
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    let mut out_brrw = out_table.borrow_mut();
    if arg_name == *ANGLE {
      match arg_dims[0] {
        TableShape::Scalar => {
          let arg = block.get_arg_columns(arguments)?[0].clone();
          out_brrw.resize(1,1);
          out_brrw.set_kind(ValueKind::F32);
          if let Column::F32(out_col) = out_brrw.get_column_unchecked(0) {
            match arg {
              (_,Column::F32(col),ColumnIndex::Index(_)) |
              (_,Column::F32(col),ColumnIndex::All) => block.plan.push(MathSinRadVV{col: (col.clone(),0,0), out: out_col.clone()}),
              (_,Column::Angle(col),ColumnIndex::All) => block.plan.push(MathSinRadVV{col: (col.clone(),0,0), out: out_col.clone()}),
              (_,col,_) => { return Err(MechError{id: 1348, kind: MechErrorKind::UnhandledFunctionArgumentKind(col.kind())}); }
            }
          }
        }
        TableShape::Column(rows) => {
          let arg = block.get_arg_columns(arguments)?[0].clone();
          out_brrw.resize(rows,1);
          out_brrw.set_kind(ValueKind::F32);
          if let Column::F32(out_col) = out_brrw.get_column_unchecked(0) {
            match arg {
              (_,Column::F32(col),ColumnIndex::All) => block.plan.push(MathSinRadVV{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
              (_,Column::Angle(col),ColumnIndex::All) => block.plan.push(MathSinRadVV{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
              (_,col,_) => { return Err(MechError{id: 1349, kind: MechErrorKind::UnhandledFunctionArgumentKind(col.kind())}); }
            }
          }
        }
        TableShape::Row(cols) => {
          let arg_cols = block.get_whole_table_arg_cols(&arguments[0])?;
          out_brrw.resize(1,cols);
          out_brrw.set_kind(ValueKind::F32);
          for col_ix in 0..cols {
            if let Column::F32(out_col) = out_brrw.get_column_unchecked(col_ix) {
              match &arg_cols[col_ix] {
                (_,Column::F32(col),ColumnIndex::All) => block.plan.push(MathSinRadVV{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                (_,Column::Angle(col),ColumnIndex::All) => block.plan.push(MathSinRadVV{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                (_,col,_) => { return Err(MechError{id: 1349, kind: MechErrorKind::UnhandledFunctionArgumentKind(col.kind())}); }
              }
            }
          }
        }
        TableShape::Matrix(rows,cols) => {
          let arg_cols = block.get_whole_table_arg_cols(&arguments[0])?;
          out_brrw.resize(rows,cols);
          out_brrw.set_kind(ValueKind::F32);
          for col_ix in 0..cols {
            if let Column::F32(out_col) = out_brrw.get_column_unchecked(col_ix) {
              match &arg_cols[col_ix] {
                (_,Column::F32(col),ColumnIndex::All) => block.plan.push(MathSinRadVV{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                (_,Column::Angle(col),ColumnIndex::All) => block.plan.push(MathSinRadVV{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                (_,col,_) => { return Err(MechError{id: 1349, kind: MechErrorKind::UnhandledFunctionArgumentKind(col.kind())}); }
              }
            }
          }
        }
        x => {return Err(MechError{id: 1350, kind: MechErrorKind::UnhandledTableShape(arg_dims[0])});},
      }
    } else {
      return Err(MechError{id: 1351, kind: MechErrorKind::UnknownFunctionArgument(arg_name)});
    }
    Ok(())
  }
}