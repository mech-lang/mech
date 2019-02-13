extern crate mech_core;

use mech_core::{Core, Quantity, ToQuantity, QuantityMath, make_quantity};

fn main() {

  let w = make_quantity(0,0,0);
  let x = make_quantity(14336512000000,-12,0);
  let y = make_quantity(8,-1,0);
  let z = make_quantity(1,0,0);
  println!("x {:#066b}", x);
  println!("y {:#066b}", y);
  println!("z {:#066b}", z);
  let q = 0.1 * 0.2;
  println!("{}", q);
  //let p = x.add(z);
  let q = w.sub(x);
  let r = q.multiply(y);
  println!("Result: {} {}", r.mantissa(), r.range());

}
