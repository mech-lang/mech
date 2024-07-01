
#[macro_export]
macro_rules! trigonometry_rad_vv {
  (
    $function_type: ident,
    $function_compiler_type: ident,
    $libm_function_call: tt,
    $registry_function_name: ident,
    $export_name: tt,
    $mech_function_name: expr,
  ) =>
  {
    use libm::$libm_function_call;

    use std::cell::RefCell;
    use std::rc::Rc;
    use mech_core::*;
    use mech_utilities::*;

    lazy_static! {
      static ref ANGLE: u64 = hash_str("angle");
    }

    #[derive(Debug)]
    pub struct $function_type {
      pub col: (ColumnV<F32>, usize, usize),
      pub out: ColumnV<F32>,
    }

    impl MechFunction for $function_type {
      fn solve(&self) {
        let (col,six,eix) = &self.col;
        self.out.borrow_mut()
                .iter_mut()
                .zip(col.borrow()[*six..=*eix].iter())
                .for_each(|(out, rhs)| *out = F32::new($libm_function_call(rhs.unwrap()))); 
      }
      fn to_string(&self) -> String { format!("{:#?}", self)}
    }

    pub struct $function_compiler_type {}

    impl MechFunctionCompiler for $function_compiler_type {
      fn compile(
          &self,
          block: &mut Block,
          arguments: &Vec<Argument>,
          out: &(TableId, TableIndex, TableIndex)
      ) -> std::result::Result<(),MechError>
      {
        if arguments.len() > 1 {
          return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1347, kind: MechErrorKind::TooManyInputArguments(arguments.len(),1)});
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
                  (_,Column::F32(col),ColumnIndex::All) => block.plan.push($function_type{col: (col.clone(),0,0), out: out_col.clone()}),
                  (_,Column::Angle(col),ColumnIndex::All) => block.plan.push($function_type{col: (col.clone(),0,0), out: out_col.clone()}),
                  (_,col,_) => { return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1348, kind: MechErrorKind::UnhandledFunctionArgumentKind}); }
                }
              }
            }
            TableShape::Column(rows) => {
              let arg = block.get_arg_columns(arguments)?[0].clone();
              out_brrw.resize(rows,1);
              out_brrw.set_kind(ValueKind::F32);
              if let Column::F32(out_col) = out_brrw.get_column_unchecked(0) {
                match arg {
                  (_,Column::F32(col),ColumnIndex::All) => block.plan.push($function_type{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                  (_,Column::Angle(col),ColumnIndex::All) => block.plan.push($function_type{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                  (_,col,_) => { return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1349, kind: MechErrorKind::UnhandledFunctionArgumentKind}); }
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
                    (_,Column::F32(col),ColumnIndex::All) => block.plan.push($function_type{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                    (_,Column::Angle(col),ColumnIndex::All) => block.plan.push($function_type{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                    (_,col,_) => { return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1349, kind: MechErrorKind::UnhandledFunctionArgumentKind}); }
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
                    (_,Column::F32(col),ColumnIndex::All) => block.plan.push($function_type{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                    (_,Column::Angle(col),ColumnIndex::All) => block.plan.push($function_type{col: (col.clone(),0,col.len()-1), out: out_col.clone()}),
                    (_,col,_) => { return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1349, kind: MechErrorKind::UnhandledFunctionArgumentKind}); }
                  }
                }
              }
            }
            x => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1350, kind: MechErrorKind::UnhandledTableShape(arg_dims[0])});},
          }
        } else {
          return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1351, kind: MechErrorKind::UnknownFunctionArgument(arg_name)});
        }
        Ok(())
      }
    }

    extern "C" fn $registry_function_name(registrar: &mut dyn MechFunctionRegistrar) {
      registrar.register_mech_function(
          hash_str($mech_function_name), Box::new($function_compiler_type{}));
    }

    export_mech_function!($export_name, $registry_function_name);
  }  // end macro pattern
}  // end macro trigonometry_rad_vv
