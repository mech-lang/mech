use super::super::registration::register_initialized_expression_function;
use super::super::environment::expression_solves_deferred;
#[cfg(feature = "table")]
use crate::AccessColumn;
#[cfg(feature = "swizzle")]
use crate::AccessSwizzle;
#[cfg(feature = "matrix")]
use crate::MatrixAccessScalar;
#[cfg(feature = "tuple")]
use crate::TupleAccess;
use crate::{
  InterpreterExecution, MResult, NativeFunctionCompiler, Subscript, Value, ValueKind, real,
};

pub(super) fn access(
  sbscrpt: &Subscript,
  val: &Value,
  p: &InterpreterExecution<'_>,
) -> MResult<Value> {
  let plan = p.plan();
  match sbscrpt {
        #[cfg(feature = "table")]
        Subscript::Dot(x) => {
            let key = x.hash();
            let fxn_input: Vec<Value> = vec![val.clone(), Value::Id(key)];
            #[cfg(feature = "record")]
            if matches!(val.deref_kind(), ValueKind::Record(..)) {
                let function = AccessColumn {}.compile(&fxn_input)?;
                return register_initialized_expression_function(&plan, function, &[]);
            }
            let new_fxn = AccessColumn {}.compile(&fxn_input)?;
            if !expression_solves_deferred(p) {
              new_fxn.solve();
            }
            let res = new_fxn.out();
            plan.borrow_mut().push(new_fxn);
            return Ok(res);
        }
        Subscript::DotInt(x) => {
            let mut fxn_input = vec![val.clone()];
            let result = real(&x.clone(), p)?;
            fxn_input.push(result.as_index()?);
            match val.deref_kind() {
                #[cfg(feature = "matrix")]
                ValueKind::Matrix(..) => {
                    let new_fxn = MatrixAccessScalar {}.compile(&fxn_input)?;
                    if !expression_solves_deferred(p) {
                      new_fxn.solve();
                    }
                    let res = new_fxn.out();
                    plan.borrow_mut().push(new_fxn);
                    return Ok(res);
                }
                #[cfg(feature = "tuple")]
                ValueKind::Tuple(..) => {
                    let function = TupleAccess {}.compile(&fxn_input)?;
                    return register_initialized_expression_function(&plan, function, &[]);
                }
                /*ValueKind::Record(_) => {
                  let new_fxn = RecordAccessScalar{}.compile(&fxn_input)?;
                  new_fxn.solve();
                  let res = new_fxn.out();
                  plan.borrow_mut().push(new_fxn);
                  return Ok(res);
                },*/
                _ => todo!("Implement access for dot int"),
            }
        }
        #[cfg(feature = "swizzle")]
        Subscript::Swizzle(x) => {
            let mut keys = x
                .iter()
                .map(|x| Value::Id(x.hash()))
                .collect::<Vec<Value>>();
            let mut fxn_input: Vec<Value> = vec![val.clone()];
            fxn_input.append(&mut keys);
            let new_fxn = AccessSwizzle {}.compile(&fxn_input)?;
            if !expression_solves_deferred(p) {
              new_fxn.solve();
            }
            let res = new_fxn.out();
            plan.borrow_mut().push(new_fxn);
            return Ok(res);
        }
    _ => unreachable!(),
  }
}
