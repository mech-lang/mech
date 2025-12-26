use mech_core::*;
use mech_utilities::*;
use crate::*; 

export_mech_function!(string_length, string_length_reg);

extern "C" fn string_length_reg(registrar: &mut dyn MechFunctionRegistrar) {
  registrar.register_mech_function(hash_str("string/length"),Box::new(StringLength{}));
}

// set/none(column: x)
#[derive(Debug)]
pub struct StringLengthCol {
  pub col: ColumnV<MechString>, pub out: ColumnV<U64>
}

impl MechFunction for StringLengthCol {
  fn solve(&self) {
    let col_brrw = self.col.borrow();
    for col in 0..col_brrw.len() {
      let len = col_brrw[col].len();
      self.out.borrow_mut()[col] = U64::new(len as u64);
    }
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct StringLength{}
impl MechFunctionCompiler for StringLength {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &Out) -> std::result::Result<(),MechError> {
    let arg_dims = block.get_arg_dims(&arguments)?;    
    let (arg_name, mut arg_column, _) = block.get_arg_columns(arguments)?[0].clone();
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    match arg_dims[0] {
      TableShape::Column(rows) => {
        if arg_name == *COLUMN {
          let out_col = {
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(rows,1);
            out_brrw.set_col_kind(0,ValueKind::U64);
            out_brrw.get_column_unchecked(0)
          };
          match (arg_column,out_col) {
            (Column::String(col),Column::U64(out)) => block.plan.push(StringLengthCol{col: col.clone(), out: out.clone()}),
            x => {return Err(MechError{id: 8392, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        } else {
          return Err(MechError{id: 8391, kind: MechErrorKind::GenericError("Unhandled shape".to_string())});
        }
      }
      x => {return Err(MechError{id: 8392, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}