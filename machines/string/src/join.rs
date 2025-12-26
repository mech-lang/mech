use mech_core::*;
use mech_utilities::*;
use crate::*; 

export_mech_function!(string_join, string_join_reg);

extern "C" fn string_join_reg(registrar: &mut dyn MechFunctionRegistrar) {
  registrar.register_mech_function(hash_str("string/join"),Box::new(StringJoin{}));
}

// set/none(column: x)
#[derive(Debug)]
pub struct StringJoinCol {
  pub col: ColumnV<MechString>, pub out: ColumnV<MechString>
}

impl MechFunction for StringJoinCol {
  fn solve(&self) {
    let col_brrw = self.col.borrow();
    let mut output_string = "".to_string();
    for col in 0..col_brrw.len() {
      output_string = format!("{}{}",output_string,col_brrw[col].to_string());
    }
    self.out.borrow_mut()[0] = MechString::from_string(output_string);
  }
  fn to_string(&self) -> String { format!("{:#?}", self)}
}

pub struct StringJoin{}
impl MechFunctionCompiler for StringJoin {
  fn compile(&self, block: &mut Block, arguments: &Vec<Argument>, out: &(TableId, TableIndex, TableIndex)) -> std::result::Result<(),MechError> {
    let arg_dims = block.get_arg_dims(&arguments)?;    
    let (arg_name, mut arg_column, _) = block.get_arg_columns(arguments)?[0].clone();
    let (out_table_id, _, _) = out;
    let out_table = block.get_table(out_table_id)?;
    match arg_dims[0] {
      TableShape::Column(rows) => {
        if arg_name == *COLUMN {
          let out_col = {
            let mut out_brrw = out_table.borrow_mut();
            out_brrw.resize(1,1);
            out_brrw.set_col_kind(0,ValueKind::String);
            out_brrw.get_column_unchecked(0)
          };
          match (arg_column,out_col) {
            (Column::String(col),Column::String(out)) => block.plan.push(StringJoinCol{col: col.clone(), out: out.clone()}),
            x => {return Err(MechError{id: 8372, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        } else {
          return Err(MechError{id: 8371, kind: MechErrorKind::GenericError("Unhandled shape".to_string())});
        }
      }
      x => {return Err(MechError{id: 8372, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}
/*
  } else if string_arg == *ROW {
    out.resize(string_vi.rows(),1);
    for row in 1..=string_vi.rows() {
      let mut output_string = "".to_string();
      for column in 1..=string_vi.columns() {
        match string_vi.table.borrow().get_string(&TableIndex::Index(row), &TableIndex::Index(column)) {
          Some((string_value,_)) => {
            output_string = format!("{}{}",output_string,string_value);
          }
          None => (),
        };
      }
      out.set_string(&TableIndex::Index(row),&TableIndex::Index(1),Value::from_string(&output_string.to_string()),output_string.to_string());
    }
  } else if string_arg == *TABLE {
    let mut output_string = "".to_string();
    out.resize(1,1);
    for row in 1..=string_vi.rows() {
      for column in 1..=string_vi.columns() {
        match string_vi.table.borrow().get_string(&TableIndex::Index(row), &TableIndex::Index(column)) {
          Some((string_value,_)) => {
            output_string = format!("{}{}",output_string,string_value);
          }
          None => (),
        };
      }
    }
    out.set_string(&TableIndex::Index(1),&TableIndex::Index(1),Value::from_string(&output_string.to_string()),output_string.to_string());
  } else {
    // TODO Warn about unknown argument
  }
}*/