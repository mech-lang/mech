use super::*;
use crate::*;

pub(super) fn access(
  sbscrpt: &Subscript,
  val: &Value,
  env: Option<&Environment>,
  p: &InterpreterExecution<'_>,
) -> MResult<Value> {
  let plan = p.plan();
  match sbscrpt {
        Subscript::Brace(subs) => {
            let mut fxn_input = vec![val.clone()];
            match &subs[..] {
                #[cfg(feature = "subscript_formula")]
                [Subscript::Formula(ix)] => {
                    let result = subscript_formula(&subs[0], env, p)?;
                    let shape = result.shape();
                    fxn_input.push(result);
                    match shape[..] {
                        [1, 1] => { plan.borrow_mut().push(AccessScalar {}.compile(&fxn_input)?); }
                        #[cfg(feature = "subscript_range")]
                        [n, 1] => { plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?); }
                        #[cfg(feature = "subscript_range")]
                        [1, n] => { plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?); }
                        _ => todo!(),
                    }
                }
                #[cfg(feature = "subscript_range")]
                [Subscript::Range(ix)] => {
                    let result = subscript_range(&subs[0], env, p)?;
                    fxn_input.push(result);
                    plan.borrow_mut().push(AccessRange {}.compile(&fxn_input)?);
                }
                /*[Subscript::All] => {
                  fxn_input.push(Value::IndexAll);
                  #[cfg(feature = "matrix")]
                  plan.borrow_mut().push(MapAccessAll{}.compile(&fxn_input)?);
                },*/
                _ => {
                    todo!("Implement brace subscript")
                }
            }
            let plan_brrw = plan.borrow();
            let mut new_fxn = &plan_brrw.last().unwrap();
            if !expression_solves_deferred(p) {
              new_fxn.solve();
            }
            let res = new_fxn.out();
            return Ok(res);
        }
    _ => unreachable!(),
  }
}
