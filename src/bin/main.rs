extern crate mech_core;

use mech_core::{Core, Quantity, ToQuantity, QuantityMath, make_quantity};

fn main() {

  let x = make_quantity(1,-1,1);
  let y = make_quantity(2,-1,2);
  let z = make_quantity(3,-1,0);
  println!("x {:#066b}", x);
  println!("y {:#066b}", y);
  println!("z {:#066b}", z);
  let q = 0.1 * 0.2;
  println!("{}", q);
  let r = x.add(y);
  println!("Result: {}", r.to_string());

}
