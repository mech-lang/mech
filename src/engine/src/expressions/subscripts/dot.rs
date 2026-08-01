use super::super::environment::expression_solves_deferred;
use super::super::registration::register_initialized_expression_function;
use super::catalog_access_function;
use crate::{InterpreterExecution, MResult, Subscript, Value, ValueKind, real};

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
                let function = catalog_access_function(p, "access/column", &fxn_input)?;
                return register_initialized_expression_function(&plan, function, &[]);
            }
            let new_fxn = catalog_access_function(p, "access/column", &fxn_input)?;
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
                    let new_fxn = catalog_access_function(p, "access/scalar", &fxn_input)?;
                    if !expression_solves_deferred(p) {
                        new_fxn.solve();
                    }
                    let res = new_fxn.out();
                    plan.borrow_mut().push(new_fxn);
                    return Ok(res);
                }
                #[cfg(feature = "tuple")]
                ValueKind::Tuple(..) => {
                    let function = catalog_access_function(p, "access/scalar", &fxn_input)?;
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
            let new_fxn = catalog_access_function(p, "access/swizzle", &fxn_input)?;
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
