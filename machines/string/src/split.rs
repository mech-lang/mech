use mech_core::*;
use mech_utilities::*;
use crate::*; 

export_mech_function!(string_split, string_split_reg);

extern "C" fn string_split_reg(registrar: &mut dyn MechFunctionRegistrar) {
  registrar.register_mech_function(hash_str("string/split"),Box::new(StringSplit{}));
}

// set/none(column: x)
#[derive(Debug)]
pub struct StringSplitCol {
  pub col: ColumnV<MechString>, pub out: ColumnV<MechString>
}

impl MechFunction for StringSplitCol {
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

pub struct StringSplit{}
impl MechFunctionCompiler for StringSplit {
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
            out_brrw.resize(1,1);
            out_brrw.set_col_kind(0,ValueKind::String);
            out_brrw.get_column_unchecked(0)
          };
          match (arg_column,out_col) {
            (Column::String(col),Column::String(out)) => block.plan.push(StringSplitCol{col: col.clone(), out: out.clone()}),
            x => {return Err(MechError{id: 8382, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        } else {
          return Err(MechError{id: 8381, kind: MechErrorKind::GenericError("Unhandled shape".to_string())});
        }
      }
      x => {return Err(MechError{id: 8382, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
}




/*
#[no_mangle]
pub extern "C" fn string_split(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  let arg1 = arguments[0].borrow();
  let string_arg = arg1.name;
  let string_vi = arg1.iterator.clone();
  let arg2 = arguments[0].borrow();
  let separator_arg = arg2.name;
  let separator_vi = arg2.iterator.clone();
  let mut out = arguments.last().unwrap().borrow().iterator.clone();
  
  if string_arg == *TABLE && separator_arg == *SEPARATOR {
    for row in 1..=string_vi.rows() {
      match (string_vi.table.borrow().get_string(&TableIndex::Index(row), &TableIndex::Index(1)),
             separator_vi.table.borrow().get_string(&TableIndex::Index(1), &TableIndex::Index(1))) {
        (Some((string_value,_)),Some((separator,_))) => {
          let split_string = string_value.split(separator).collect::<Vec<_>>();
          out.resize(1,split_string.len());
          for (column, substring) in split_string.iter().enumerate() {
            out.set_string(&TableIndex::Index(row),&TableIndex::Index(column+1),Value::from_string(&substring.to_string()),substring.to_string());
          }
        }
        _ => (),
      };
    }
  } else {
    // TODO Warn about unknown argument
  }
}*/