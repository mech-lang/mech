extern crate mech_core;
extern crate serde; // 1.0.68
#[macro_use]
extern crate serde_derive; // 1.0.68

use mech_core::{Core, Quantity, ToQuantity, QuantityMath, make_quantity};

extern crate hashbrown;
use hashbrown::hash_map::HashMap;
use serde::*;
use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeMap};


fn main() {
  let x = make_quantity(1, 3, 1);
  //let y = make_quantity(1, 3, 0);
  //println!("{:#010b}", x.domain());
  ////println!("{:#066b}", y);
  assert_eq!(x.range(), 3);
}